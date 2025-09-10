use std::collections::HashMap;

use hydro_lang::compile::rewrites::persist_pullup;
use hydro_lang::deploy::HydroDeploy;
use hydro_lang::location::Location;
use hydro_lang::prelude::*;

use crate::decoupler;
use crate::partitioner::Partitioner;

mod two_pc;

struct DecoupledCluster;

#[test]
fn decoupled_compute_pi_ir() {
    let builder = FlowBuilder::new();
    let (cluster, _) = hydro_test::cluster::compute_pi::compute_pi(&builder, 8192);
    let decoupled_cluster = builder.cluster::<DecoupledCluster>();
    let decoupler = decoupler::Decoupler {
        output_to_decoupled_machine_after: vec![4],
        output_to_original_machine_after: vec![],
        place_on_decoupled_machine: vec![],
        decoupled_location: decoupled_cluster.id().clone(),
        orig_location: cluster.id().clone(),
    };
    let built = builder
        .optimize_with(persist_pullup::persist_pullup)
        .optimize_with(|roots| decoupler::decouple(roots, &decoupler))
        .into_deploy::<HydroDeploy>();

    hydro_build_utils::assert_debug_snapshot!(built.ir());

    for (id, ir) in built.preview_compile().all_dfir() {
        hydro_build_utils::insta::with_settings!({
            snapshot_suffix => format!("surface_graph_{id}"),
        }, {
            hydro_build_utils::assert_snapshot!(ir.surface_syntax_string());
        });
    }
}

#[test]
fn partitioned_simple_cluster_ir() {
    let builder = FlowBuilder::new();
    let (_, cluster) = hydro_test::cluster::simple_cluster::simple_cluster(&builder);
    let partitioner = Partitioner {
        nodes_to_partition: HashMap::from([(5, vec!["1".to_string()])]),
        num_partitions: 3,
        location_id: cluster.id().raw_id(),
        new_cluster_id: None,
    };
    let built = builder
        .optimize_with(persist_pullup::persist_pullup)
        .optimize_with(|roots| crate::partitioner::partition(roots, &partitioner))
        .into_deploy::<HydroDeploy>();

    hydro_build_utils::assert_debug_snapshot!(built.ir());

    for (id, ir) in built.preview_compile().all_dfir() {
        hydro_build_utils::insta::with_settings!({
            snapshot_suffix => format!("surface_graph_{id}")
        }, {
            hydro_build_utils::assert_snapshot!(ir.surface_syntax_string());
        });
    }
}
