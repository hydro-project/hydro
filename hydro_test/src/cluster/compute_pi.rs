use std::time::Duration;

use hydro_lang::*;

pub struct Worker {}
pub struct Leader {}

pub fn compute_pi<'a>(
    flow: &FlowBuilder<'a>,
    batch_size: usize,
) -> (Cluster<'a, Worker>, Process<'a, Leader>) {
    let cluster = flow.cluster();
    let process = flow.process();

    let trials = cluster
        .tick()
        .spin_batch(q!(batch_size))
        .map(q!(|_| rand::random::<(f64, f64)>()))
        .map(q!(|(x, y)| x * x + y * y < 1.0))
        .fold(
            q!(|| (0u64, 0u64)),
            q!(|(inside, total), sample_inside| {
                if sample_inside {
                    *inside += 1;
                }

                *total += 1;
            }),
        )
        .all_ticks();

    let estimate = trials
        .send_bincode(&process)
        .values()
        .reduce_commutative(q!(|(inside, total), (inside_batch, total_batch)| {
            *inside += inside_batch;
            *total += total_batch;
        }));

    unsafe {
        // SAFETY: intentional non-determinism
        estimate.sample_every(q!(Duration::from_secs(1)))
    }
    .for_each(q!(|(inside, total)| {
        println!(
            "pi: {} ({} trials)",
            4.0 * inside as f64 / total as f64,
            total
        );
    }));

    (cluster, process)
}

#[cfg(test)]
mod tests {
    use hydro_lang::Location;
    use hydro_lang::deploy::HydroDeploy;
    use hydro_lang::rewrites::persist_pullup;
    use hydro_optimize::decoupler;

    struct DecoupledCluster {}

    #[test]
    fn compute_pi_ir() {
        let builder = hydro_lang::FlowBuilder::new();
        let _ = super::compute_pi(&builder, 8192);
        let built = builder.with_default_optimize::<HydroDeploy>();

        insta::assert_debug_snapshot!(built.ir());

        for (id, ir) in built.preview_compile().all_dfir() {
            insta::with_settings!({snapshot_suffix => format!("surface_graph_{id}")}, {
                insta::assert_snapshot!(ir.surface_syntax_string());
            });
        }
    }

    #[test]
    fn decoupled_compute_pi_ir() {
        let builder = hydro_lang::FlowBuilder::new();
        let (cluster, _) = super::compute_pi(&builder, 8192);
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
            .optimize_with(|leaves| decoupler::decouple(leaves, &decoupler))
            .into_deploy::<HydroDeploy>();

        insta::assert_debug_snapshot!(built.ir());

        for (id, ir) in built.preview_compile().all_dfir() {
            insta::with_settings!({snapshot_suffix => format!("surface_graph_{id}")}, {
                insta::assert_snapshot!(ir.surface_syntax_string());
            });
        }
    }
}
