use std::time::Duration;

use hydroflow_plus::*;
use stageleft::*;

pub struct Worker {}
pub struct Leader {}

pub fn compute_pi<'a>(
    flow: &FlowBuilder<'a>,
    batch_size: usize,
) -> (Cluster<'a, Worker>, Process<'a, Leader>) {
    let cluster = flow.cluster();
    let process = flow.process();

    let trials = cluster
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

    trials
        .send_bincode_interleaved(&process)
        .reduce(q!(|(inside, total), (inside_batch, total_batch)| {
            *inside += inside_batch;
            *total += total_batch;
        }))
        .sample_every(q!(Duration::from_secs(1)))
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
    use hydroflow_plus::deploy::DeployRuntime;
    use stageleft::RuntimeData;

    #[test]
    fn compute_pi_ir() {
        let builder = hydroflow_plus::FlowBuilder::new();
        let _ = super::compute_pi(&builder, 8192);
        let built = builder.with_default_optimize();

        insta::assert_debug_snapshot!(built.ir());

        for (id, ir) in built
            .compile::<DeployRuntime>(&RuntimeData::new("FAKE"))
            .hydroflow_ir()
        {
            insta::with_settings!({snapshot_suffix => format!("surface_graph_{id}")}, {
                insta::assert_snapshot!(ir.surface_syntax_string());
            });
        }
    }
}
