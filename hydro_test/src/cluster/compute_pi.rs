use std::time::Duration;

use hydro_lang::prelude::*;

pub enum Worker {}
pub enum Leader {}

pub fn compute_pi<'a>(
    flow: &mut FlowBuilder<'a>,
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

    let estimate = trials.send(&process, TCP.bincode()).values().reduce(q!(
        |(inside, total), (inside_batch, total_batch)| {
            *inside += inside_batch;
            *total += total_batch;
        },
        commutative = ManualProof(/* int addition is commutative */)
    ));

    estimate
        .sample_every(
            q!(Duration::from_secs(1)),
            nondet!(/** intentional output */),
        )
        .assume_retries(nondet!(/** extra logs due to duplicate samples are okay */))
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
    use hydro_lang::deploy::HydroDeploy;

    #[test]
    fn compute_pi_ir() {
        let mut builder = hydro_lang::compile::builder::FlowBuilder::new();
        let _ = super::compute_pi(&mut builder, 8192);
        let mut built = builder.with_default_optimize::<HydroDeploy>();

        hydro_build_utils::assert_debug_snapshot!(built.ir());

        for (location_key, ir) in built.preview_compile().all_dfir() {
            hydro_build_utils::insta::with_settings!({
                snapshot_suffix => format!("surface_graph_{location_key}"),
            }, {
                hydro_build_utils::assert_snapshot!(ir.surface_syntax_string());
            });
        }
    }
}
