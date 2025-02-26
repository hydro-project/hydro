use std::cell::RefCell;
use std::collections::HashMap;

use grb::prelude::*;

use crate::ir::*;
use crate::location::LocationId;

struct ModelMetadata {
    // Const fields
    cluster_to_decouple: LocationId,
    decoupling_send_overhead: f64, /* CPU usage per cardinality to send, assuming all messages serialize/deserialize similarly */
    decoupling_recv_overhead: f64,
    // Model variables to construct final cost function
    model: Model,
    stmt_id_to_var: HashMap<usize, Var>,
    stmt_id_to_metadata: HashMap<usize, HydroIrMetadata>,
    ops_with_same_tick: HashMap<usize, Vec<usize>>, // tick_id: vec of op_id
    tees_with_same_inner: HashMap<usize, (Vec<usize>, usize)>, // inner_id: (vec of Tee op_id, cardinality)
    potential_decoupling_network_cardinalities: Vec<(usize, usize, usize)>, /* (operator ID, input ID, cardinality) */
}

fn add_var_and_metadata(metadata: &HydroIrMetadata, model_metadata: &RefCell<ModelMetadata>) {
    let ModelMetadata {
        model,
        stmt_id_to_var,
        stmt_id_to_metadata,
        ..
    } = &mut *model_metadata.borrow_mut();

    // Create var
    let id = metadata.id.unwrap();
    let name = id.to_string();
    let var = add_binvar!(model, name: &name, bounds: ..).unwrap();

    stmt_id_to_var.insert(id, var);
    stmt_id_to_metadata.insert(id, metadata.clone());
}

// Store how much data we would need to send if we decoupled above this
fn add_decoupling_overhead(input_metadatas: Vec<&mut HydroIrMetadata>, model_metadata: &RefCell<ModelMetadata>) {
    let ModelMetadata {
        cluster_to_decouple,
        potential_decoupling_network_cardinalities,
        ..
    } = &mut *model_metadata.borrow_mut();

    for input_metadata in input_metadatas {
        if cluster_to_decouple != input_metadata.location_kind.root() {
            continue;
        }
        if let Some(input_id) = input_metadata.id {
            if let Some(input_cardinality) = input_metadata.cardinality {
                potential_decoupling_network_cardinalities.push((
                    input_metadata.id.unwrap(),
                    input_id,
                    input_cardinality,
                ));
            }
        }
    }
}

// Store the tick that an op is constrained to
fn add_tick_constraint(metadata: &HydroIrMetadata, model_metadata: &RefCell<ModelMetadata>) {
    let ModelMetadata {
        ops_with_same_tick,
        ..
    } = &mut *model_metadata.borrow_mut();
    if let LocationId::Tick(tick_id, _) = metadata.location_kind {
        ops_with_same_tick
            .entry(tick_id)
            .or_insert_with(Vec::new)
            .push(metadata.id.unwrap());
    }
}

fn decouple_analysis_leaf(
    leaf: &mut HydroLeaf,
    _next_stmt_id: &mut usize,
    model_metadata: &RefCell<ModelMetadata>,
) {
    // Ignore nodes that are not in the cluster to decouple
    if model_metadata.borrow().cluster_to_decouple != *leaf.metadata().location_kind.root() {
        return;
    }

    add_var_and_metadata(&leaf.metadata(), model_metadata);
    add_decoupling_overhead(leaf.input_metadata_mut(), model_metadata);
    add_tick_constraint(&leaf.metadata(), model_metadata);
}

fn decouple_analysis_node(
    node: &mut HydroNode,
    next_stmt_id: &mut usize,
    model_metadata: &RefCell<ModelMetadata>,
) {
    // Ignore nodes that are not in the cluster to decouple
    if model_metadata.borrow().cluster_to_decouple != *node.metadata().location_kind.root() {
        return;
    }

    add_var_and_metadata(&node.metadata(), model_metadata);
    // Add metadata for calculating decoupling overhead, special cases for Tee and CycleSource
    match node {
        HydroNode::Tee { inner, .. } => {
            let inner_node = inner.0.borrow();
            let inner_metadata = inner_node.metadata();
            if model_metadata.borrow().cluster_to_decouple != *inner_metadata.location_kind.root() {
                if let Some(inner_cardinality) = inner_metadata.cardinality {
                    let ModelMetadata {
                        tees_with_same_inner,
                        ..
                    } = &mut *model_metadata.borrow_mut();
                    let entry = tees_with_same_inner
                        .entry(inner_metadata.id.unwrap())
                        .or_insert_with(|| (vec![], inner_cardinality));
                    entry.0.push(*next_stmt_id);
                }
            }
        }
        HydroNode::CycleSource { .. } => {
            // Do nothing, will be handled later
        }
        _ => {
            add_decoupling_overhead(node.input_metadata_mut(), model_metadata);
        }
    }
    add_tick_constraint(&node.metadata(), model_metadata);
}

// Return a variable representing whether op1 and op2 are assigned to different machines
fn add_decoupled_var(model: &mut Model, op1: usize, op2: usize, stmt_id_to_var: &HashMap<usize, Var>) -> Var {
    let op1_var = stmt_id_to_var.get(&op1).unwrap();
    let op2_var = stmt_id_to_var.get(&op2).unwrap();
    let diff_var = add_intvar!(model, bounds: ..).unwrap();
    model
        .add_constr(
            &format!("{}_{}_diff", op1, op2),
            c!(diff_var == *op1_var - *op2_var),
        )
        .unwrap(); 
    // Variable that is 1 if op and input are on different nodes
    let decoupled_var = add_binvar!(model, bounds: ..).unwrap();
    model
        .add_genconstr_abs(
            &format!("{}_{}_decoupled", op1, op2),
            decoupled_var,
            diff_var,
        )
        .unwrap();
    decoupled_var
}

fn construct_objective_fn(model_metadata: &RefCell<ModelMetadata>, cycle_sink_to_sources: &HashMap<usize, usize>) {
    let ModelMetadata {
        decoupling_send_overhead,
        decoupling_recv_overhead,
        model,
        stmt_id_to_var,
        stmt_id_to_metadata,
        ops_with_same_tick,
        tees_with_same_inner,
        potential_decoupling_network_cardinalities,
        ..
    } = &mut *model_metadata.borrow_mut();

    // Manually make sure all vars are added to the model so we can look them up with var_for_op_id
    model.update().unwrap();

    // Add tick constraints
    for (_, ops) in ops_with_same_tick {
        let mut prev_op: Option<usize> = None;
        for op_id in ops {
            if let Some(prev_op_id) = prev_op {
                let prev_op_var = stmt_id_to_var.get(&prev_op_id).unwrap();
                let op_var = stmt_id_to_var.get(op_id).unwrap();
                model.add_constr(
                        &format!("tick_constraint_{}_{}", prev_op_id, op_id),
                        c!(prev_op_var == op_var),
                    )
                    .unwrap();
            }
            prev_op = Some(*op_id);
        }
    }

    let mut orig_node_cpu_expr = Expr::default();
    let mut decoupled_node_cpu_expr = Expr::default();

    // Calculate total CPU usage on each node (before overheads)
    for (stmt_id, metadata) in stmt_id_to_metadata.iter() {
        if let Some(cpu_usage) = metadata.cpu_usage {
            let var = stmt_id_to_var.get(stmt_id).unwrap();

            orig_node_cpu_expr = orig_node_cpu_expr + cpu_usage * *var;
            decoupled_node_cpu_expr = decoupled_node_cpu_expr + cpu_usage * (1 - *var);
        }
    }

    // Calculate overheads
    for (op, input, cardinality) in potential_decoupling_network_cardinalities {
        // Penalize if the op and input are on different nodes
        let op_input_decoupled_var = add_decoupled_var(model, *op, *input, &stmt_id_to_var);
        orig_node_cpu_expr =
            orig_node_cpu_expr + *decoupling_send_overhead * *cardinality as f64 * op_input_decoupled_var;
        decoupled_node_cpu_expr = decoupled_node_cpu_expr
            + *decoupling_recv_overhead * *cardinality as f64 * op_input_decoupled_var;

        if let Some(source_id) = cycle_sink_to_sources.get(op) {
            // If the op is a CycleSink, then decoupling above the sink = decoupling above the source as well, so factor that overhead in too
            let source_input_decoupled_var = add_decoupled_var(model, *source_id, *input, &stmt_id_to_var);
            orig_node_cpu_expr =
                orig_node_cpu_expr + *decoupling_send_overhead * *cardinality as f64 * source_input_decoupled_var;
            decoupled_node_cpu_expr = decoupled_node_cpu_expr
                + *decoupling_recv_overhead * *cardinality as f64 * source_input_decoupled_var;
        }
    }
    // Calculate overhead of decoupling any set of Tees from its inner. Only penalize decoupling once for decoupling any number of Tees.
    for (tee_inner, (ops, cardinality)) in tees_with_same_inner {
        // Variable that is 1 if the inner and any Tees are on different nodes
        let any_decoupled_var = add_binvar!(model, bounds: ..).unwrap();

        for op in ops {
            let op_inner_decoupled_var = add_decoupled_var(model, *op, *tee_inner, &stmt_id_to_var);
            // any_decoupled_var is at least decoupled_var
            model.add_constr(
                &format!("tee{}_inner{}_any_decoupled", op, tee_inner),
                c!(any_decoupled_var >= op_inner_decoupled_var),
            ).unwrap();
        }

        orig_node_cpu_expr =
            orig_node_cpu_expr + *decoupling_send_overhead * *cardinality as f64 * any_decoupled_var;
        decoupled_node_cpu_expr = decoupled_node_cpu_expr
            + *decoupling_recv_overhead * *cardinality as f64 * any_decoupled_var;
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

pub fn decouple_analysis(ir: &mut [HydroLeaf], modelname: &str, cluster_to_decouple: &LocationId, cycle_sink_to_sources: &HashMap<usize, usize>) {
    let model_metadata = RefCell::new(ModelMetadata {
        cluster_to_decouple: cluster_to_decouple.clone(),
        decoupling_send_overhead: 0.001, // TODO: Calculate
        decoupling_recv_overhead: 0.001,
        model: Model::new(modelname).unwrap(),
        stmt_id_to_var: HashMap::new(),
        stmt_id_to_metadata: HashMap::new(),
        ops_with_same_tick: HashMap::new(),
        tees_with_same_inner: HashMap::new(),
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

    construct_objective_fn(&model_metadata, cycle_sink_to_sources);
    let ModelMetadata {
        stmt_id_to_var,
        stmt_id_to_metadata,
        model,
        ..
    } = &mut *model_metadata.borrow_mut();
    model.optimize().unwrap();

    println!("We're decoupling the following operators:");    
    for (stmt_id, _) in stmt_id_to_metadata.iter() {
        if model.get_obj_attr(attr::X, stmt_id_to_var.get(stmt_id).unwrap()).unwrap() == 0.0 {
            println!("{}", stmt_id);
        }
    }
}
