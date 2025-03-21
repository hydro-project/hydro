use std::time::Duration;

use dfir_rs::scheduled::graph::Dfir;
use hydro_lang::deploy::SingleProcessGraph;
use hydro_lang::*;
use stageleft::{Quoted, RuntimeData};

pub fn compute_pi<'a>(flow: &FlowBuilder<'a>, batch_size: RuntimeData<usize>) -> Process<'a, ()> {
    let process = flow.process();
    let tick = process.tick();

    let trials = tick
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

    let estimate = trials.reduce(q!(|(inside, total), (inside_batch, total_batch)| {
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

    process
}

#[stageleft::entry]
pub fn compute_pi_runtime<'a>(
    flow: FlowBuilder<'a>,
    batch_size: RuntimeData<usize>,
) -> impl Quoted<'a, Dfir<'a>> {
    let _ = compute_pi(&flow, batch_size);
    flow.compile_no_network::<SingleProcessGraph>()
}
