//! A panic raised by program code inside the simulation dylib must reach the host as an ordinary
//! Rust panic, with its message, rather than aborting the process.

use stageleft::q;

use crate::live_collections::stream::{ExactlyOnce, TotalOrder};
use crate::location::Location;
use crate::prelude::FlowBuilder;
use crate::sim::{SimReceiver, SimSender};

/// Compiles a process that echoes its input and panics on `7`.
#[cfg(stageleft_runtime)]
fn panicking_echo() -> (
    crate::sim::compiled::CompiledSim,
    SimSender<u32, TotalOrder, ExactlyOnce>,
    SimReceiver<u32, TotalOrder, ExactlyOnce>,
) {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let (in_send, input) = node.sim_input::<u32, _, _>();
    let out_recv = input
        .map(q!(|x: u32| {
            if x == 7 {
                panic!("program panicked on {}", x);
            }
            x
        }))
        .sim_output();
    (flow.sim().compiled(), in_send, out_recv)
}

#[test]
fn a_program_panic_is_caught_on_the_host() {
    let (sim, in_send, out_recv) = panicking_echo();

    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        sim.exhaustive(async || {
            in_send.send(7);
            out_recv.assert_yields_only(Vec::<u32>::new()).await;
        });
    }));
    let payload = caught.expect_err("the program's panic must propagate to the host");
    let msg = payload
        .downcast_ref::<String>()
        .expect("the panic carries its message");
    assert!(msg.contains("program panicked on 7"), "{msg}");

    // The compiled simulation is still usable afterwards.
    let runs = sim.exhaustive(async || {
        in_send.send(1);
        out_recv.assert_yields_only([1]).await;
    });
    assert!(runs >= 1);
}

#[test]
#[should_panic(expected = "program panicked on 7")]
fn a_program_panic_fails_the_test_with_its_message() {
    let (sim, in_send, out_recv) = panicking_echo();
    sim.exhaustive(async || {
        in_send.send(7);
        out_recv.assert_yields_only(Vec::<u32>::new()).await;
    });
}
