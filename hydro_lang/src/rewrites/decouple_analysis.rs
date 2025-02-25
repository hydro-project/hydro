use std::cell::RefCell;
use std::collections::HashMap;

use grb::prelude::*;

use crate::ir::*;
use crate::location::LocationId;

struct ModelMetadata {
    cluster_to_decouple: LocationId,
    decoupling_send_overhead: f64, /* CPU usage per cardinality to send, assuming all messages serialize/deserialize similarly */
    decoupling_recv_overhead: f64,
    // Model variables to construct final cost function
    model: Model,
    stmt_id_to_metadata: HashMap<usize, HydroIrMetadata>,
    ops_with_same_tick: HashMap<LocationId, Vec<usize>>, // (location, op_id)
    potential_decoupling_network_cardinalities: Vec<(usize, usize, usize)>, /* (operator ID, input ID, cardinality) */
}

fn decouple_analysis_leaf(
    leaf: &mut HydroLeaf,
    next_stmt_id: &mut usize,
    model_metadata: &RefCell<ModelMetadata>,
) {
    let ModelMetadata {
        cluster_to_decouple,
        model,
        stmt_id_to_metadata,
        potential_decoupling_network_cardinalities,
        ..
    } = &mut *model_metadata.borrow_mut();

    // Ignore nodes that are not in the cluster to decouple
    if cluster_to_decouple != &leaf.metadata().location_kind {
        return;
    }

    // Create var
    let name = next_stmt_id.to_string();
    add_binvar!(model, name: &name, bounds: ..).unwrap();

    stmt_id_to_metadata.insert(*next_stmt_id, leaf.metadata().clone());

    // Store how much data we would need to send if we decoupled above this
    for input_metadata in leaf.input_metadata_mut() {
        if *cluster_to_decouple != input_metadata.location_kind {
            continue;
        }
        if let Some(input_id) = input_metadata.id {
            if let Some(input_cardinality) = input_metadata.cardinality {
                potential_decoupling_network_cardinalities.push((
                    *next_stmt_id,
                    input_id,
                    input_cardinality,
                ));
            }
        }
    }

    // TODO: Tick constraints
}

fn decouple_analysis_node(
    node: &mut HydroNode,
    next_stmt_id: &mut usize,
    model_metadata: &RefCell<ModelMetadata>,
) {
}

fn construct_objective_fn(model_metadata: &RefCell<ModelMetadata>) {
    let ModelMetadata {
        decoupling_send_overhead,
        decoupling_recv_overhead,
        model,
        stmt_id_to_metadata,
        potential_decoupling_network_cardinalities,
        ..
    } = &mut *model_metadata.borrow_mut();

    let mut orig_node_cpu_expr = Expr::default();
    let mut decoupled_node_cpu_expr = Expr::default();

    // Calculate total CPU usage on each node (before overheads)
    for (stmt_id, metadata) in stmt_id_to_metadata.iter() {
        if let Some(cpu_usage) = metadata.cpu_usage {
            let var = model
                .get_var_by_name(&stmt_id.to_string())
                .unwrap()
                .unwrap();

            orig_node_cpu_expr = orig_node_cpu_expr + cpu_usage * var;
            decoupled_node_cpu_expr = decoupled_node_cpu_expr + cpu_usage * (1 - var);
        }
    }

    // Calculate overheads
    for (op, input, cardinality) in potential_decoupling_network_cardinalities {
        let op_var = model.get_var_by_name(&op.to_string()).unwrap().unwrap();
        let input_var = model.get_var_by_name(&input.to_string()).unwrap().unwrap();

        // Variable that is 1 if the op and its input are on different nodes
        let op_or_input_var = add_binvar!(model, bounds: ..).unwrap();
        let constr_name = format!("op{}_or_input{}", op, input);
        model
            .add_genconstr_or(&constr_name, op_or_input_var, [op_var, input_var])
            .unwrap();

        orig_node_cpu_expr =
            orig_node_cpu_expr + *decoupling_send_overhead * *cardinality as f64 * op_or_input_var;
        decoupled_node_cpu_expr = decoupled_node_cpu_expr
            + *decoupling_recv_overhead * *cardinality as f64 * op_or_input_var;
    }

    // Create vars that store the CPU usage of each node
    let orig_node_cpu_var = add_ctsvar!(model, bounds: ..).unwrap();
    let decoupled_node_cpu_var = add_ctsvar!(model, bounds: ..).unwrap();
    model
        .add_constr("orig_node_cpu", c!(orig_node_cpu_var == orig_node_cpu_expr))
        .unwrap();
    model
        .add_constr(
            "decoupled_node_cpu",
            c!(decoupled_node_cpu_var == decoupled_node_cpu_expr),
        )
        .unwrap();

    // Which node has the highest CPU usage?
    let highest_cpu = add_ctsvar!(model, bounds: ..).unwrap();
    model
        .add_genconstr_max(
            "highest_cpu",
            highest_cpu,
            [orig_node_cpu_var, decoupled_node_cpu_var],
            None,
        )
        .unwrap();

    // Minimize the CPU usage of that node
    model.set_objective(highest_cpu, Minimize).unwrap();
}

pub fn decouple_analysis(ir: &mut [HydroLeaf], modelname: &str, cluster_to_decouple: &LocationId) {
    let model_metadata = RefCell::new(ModelMetadata {
        cluster_to_decouple: cluster_to_decouple.clone(),
        decoupling_send_overhead: 0.001, // TODO: Calculate
        decoupling_recv_overhead: 0.001,
        model: Model::new(modelname).unwrap(),
        stmt_id_to_metadata: HashMap::new(),
        ops_with_same_tick: HashMap::new(),
        potential_decoupling_network_cardinalities: vec![],
    });

    traverse_dfir(
        ir,
        |leaf, next_stmt_id| {
            decouple_analysis_leaf(leaf, next_stmt_id, &model_metadata);
        },
        |node, next_stmt_id| {
            decouple_analysis_node(node, next_stmt_id, &model_metadata);
        },
    );

    construct_objective_fn(&model_metadata);
}
