//! Minimal witness: a finite network round trip `A -> B -> A` hangs above a
//! byte threshold, while the one-way `A -> B` never does. Reduced from a
//! consensus stall down to two `.send()`s, so nothing protocol-specific is
//! required to reproduce it.
//!
//! `TCP.fail_stop().bincode()` is a framing + failure policy, not a wire choice:
//! on a unix localhost the legs actually run over Unix domain sockets. The
//! behavior is transport-agnostic (`unix_bytes`/`tcp_bytes` use the same
//! `Framed<_, LengthDelimitedCodec>` sink).

use hydro_lang::live_collections::stream::NoOrder;
use hydro_lang::prelude::*;

/// `A -> B`. One-way control.
pub fn one_way<'a, A, B>(
    b: &Process<'a, B>,
    input: Stream<Vec<u8>, Process<'a, A>, Unbounded, NoOrder>,
) -> Stream<Vec<u8>, Process<'a, B>, Unbounded, NoOrder>
where
    A: 'a,
    B: 'a,
{
    input.send(b, TCP.fail_stop().bincode())
}

/// `A -> B -> A`. The finite round trip under test.
pub fn round_trip<'a, A, B>(
    a: &Process<'a, A>,
    b: &Process<'a, B>,
    input: Stream<Vec<u8>, Process<'a, A>, Unbounded, NoOrder>,
) -> Stream<Vec<u8>, Process<'a, A>, Unbounded, NoOrder>
where
    A: 'a,
    B: 'a,
{
    input
        .send(b, TCP.fail_stop().bincode())
        .send(a, TCP.fail_stop().bincode())
}

/// Same as [`round_trip`] but with `inspect` probes on each leg, printing a
/// `[PROBE]` line per item so a deployed run shows how far items travel: `AtoB`
/// = leaving A, `atB` = arrived at B, `backAtA` = returned to A.
#[cfg(stageleft_runtime)]
pub fn round_trip_probed<'a, A, B>(
    a: &Process<'a, A>,
    b: &Process<'a, B>,
    input: Stream<Vec<u8>, Process<'a, A>, Unbounded, NoOrder>,
) -> Stream<Vec<u8>, Process<'a, A>, Unbounded, NoOrder>
where
    A: 'a,
    B: 'a,
{
    input
        .inspect(q!(|_| println!("[PROBE] AtoB")))
        .send(b, TCP.fail_stop().bincode())
        .inspect(q!(|_| println!("[PROBE] atB")))
        .send(a, TCP.fail_stop().bincode())
        .inspect(q!(|_| println!("[PROBE] backAtA")))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use futures::{SinkExt, StreamExt};
    use hydro_deploy::Deployment;
    use hydro_lang::location::Location;

    /// Deploy the round trip or the one-way control, start, then concurrently
    /// send `item_count` payloads of `payload_bytes` and require all of them
    /// back within the timeout. Starting before sending is load-bearing:
    /// buffering the whole burst pre-start hides the stall.
    async fn run(round_trip: bool, item_count: usize, payload_bytes: usize) {
        let mut deployment = Deployment::new();
        let mut builder = hydro_lang::compile::builder::FlowBuilder::new();
        let external = builder.external::<()>();
        let a = builder.process::<()>();
        let b = builder.process::<()>();

        let (input_port, input) = a.source_external_bincode(&external);
        let input = input.weaken_ordering();
        let output_port = if round_trip {
            super::round_trip(&a, &b, input).send_bincode_external(&external)
        } else {
            super::one_way(&b, input).send_bincode_external(&external)
        };

        let nodes = builder
            .with_process(&a, deployment.Localhost())
            .with_process(&b, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);
        deployment.deploy().await.unwrap();

        let mut input = nodes.connect(input_port).await;
        let mut output = nodes.connect(output_port).await;
        deployment.start().await.unwrap();

        let route = if round_trip { "A->B->A" } else { "A->B" };
        let sent = std::sync::atomic::AtomicUsize::new(0);
        let recv = std::sync::atomic::AtomicUsize::new(0);
        let result = tokio::time::timeout(Duration::from_secs(20), async {
            use std::sync::atomic::Ordering::Relaxed;
            let sender = async {
                for _ in 0..item_count {
                    input.send(vec![0x5a; payload_bytes]).await.unwrap();
                    sent.fetch_add(1, Relaxed);
                }
            };
            let receiver = async {
                for _ in 0..item_count {
                    let payload = output.next().await.expect("output ended early");
                    assert_eq!(payload.len(), payload_bytes);
                    recv.fetch_add(1, Relaxed);
                }
            };
            tokio::join!(sender, receiver);
        })
        .await;
        result.unwrap_or_else(|_| {
            use std::sync::atomic::Ordering::Relaxed;
            // Frozen far below item_count => hard stall, not slow progress.
            panic!(
                "{route} stalled: {item_count} x {payload_bytes} B; \
                 frozen at sent={} recv={}",
                sent.load(Relaxed),
                recv.load(Relaxed),
            )
        });

        deployment.stop().await.unwrap();
    }

    /// Same topology and item count as the witness, few bytes: passes.
    #[tokio::test]
    async fn round_trip_156x32b_passes() {
        run(true, 156, 32).await;
    }

    /// Witness's item count and payload, but one-way: passes. The return leg is
    /// what stalls.
    #[tokio::test]
    async fn one_way_156x1kib_passes() {
        run(false, 156, 1_024).await;
    }

    /// The witness: same as `round_trip_156x32b_passes` but enough bytes in
    /// flight to hang. Deterministic locally. Run under an outer watchdog:
    ///
    /// ```text
    /// perl -e 'alarm 90; exec @ARGV' cargo test -p hydro_test \
    ///   local::finite_round_trip::tests::round_trip_156x1kib_stalls \
    ///   -- --exact --ignored --nocapture
    /// ```
    ///
    /// Threshold at 156 items is ~192 B (passes) to ~224 B (hangs); 1 KiB is a
    /// comfortable margin.
    #[tokio::test]
    #[ignore = "reproduces the round-trip stall; run explicitly under a watchdog"]
    async fn round_trip_156x1kib_stalls() {
        run(true, 156, 1_024).await;
    }

    // ------------------------------------------------------------------
    // Diagnostics: is the hang in Hydro, or in this harness? Each runs the
    // SAME hanging config (round trip, 156 x 1 KiB) but changes one thing about
    // how the *test* drives it. If any PASSES (recv reaches 156), the hang is
    // (at least partly) a harness artifact, not a Hydro deadlock.
    // ------------------------------------------------------------------

    macro_rules! deploy_rt {
        ($deployment:ident, $input:ident, $output:ident) => {
            let mut $deployment = Deployment::new();
            let mut builder = hydro_lang::compile::builder::FlowBuilder::new();
            let external = builder.external::<()>();
            let a = builder.process::<()>();
            let b = builder.process::<()>();
            let (input_port, input) = a.source_external_bincode(&external);
            let input = input.weaken_ordering();
            let output_port = super::round_trip(&a, &b, input).send_bincode_external(&external);
            let nodes = builder
                .with_process(&a, $deployment.Localhost())
                .with_process(&b, $deployment.Localhost())
                .with_external(&external, $deployment.Localhost())
                .deploy(&mut $deployment);
            $deployment.deploy().await.unwrap();
            let mut $input = nodes.connect(input_port).await;
            let mut $output = nodes.connect(output_port).await;
            $deployment.start().await.unwrap();
        };
    }

    /// DIAG 1: same hang config, but on a MULTI-THREAD runtime. If this passes,
    /// the "hang" was single-threaded executor starvation in the harness.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[ignore = "diagnostic"]
    async fn diag_multithread() {
        deploy_rt!(deployment, input, output);
        let mut sent = 0usize;
        let mut recv = 0usize;
        let _ = tokio::time::timeout(Duration::from_secs(20), async {
            let sender = async {
                for _ in 0..156 {
                    input.send(vec![0x5a; 1024]).await.unwrap();
                    sent += 1;
                }
            };
            let receiver = async {
                for _ in 0..156 {
                    output.next().await.expect("ended early");
                    recv += 1;
                }
            };
            tokio::join!(sender, receiver);
        })
        .await;
        eprintln!("DIAG multithread: sent={sent} recv={recv} (of 156)");
        deployment.stop().await.unwrap();
    }

    /// DIAG 2: same hang config, single thread, but flush after every send. If
    /// this passes, the hang was unflushed client-side buffering in the harness.
    #[tokio::test]
    #[ignore = "diagnostic"]
    async fn diag_flush_each() {
        deploy_rt!(deployment, input, output);
        let mut sent = 0usize;
        let mut recv = 0usize;
        let _ = tokio::time::timeout(Duration::from_secs(20), async {
            let sender = async {
                for _ in 0..156 {
                    input.feed(vec![0x5a; 1024]).await.unwrap();
                    input.flush().await.unwrap();
                    sent += 1;
                }
            };
            let receiver = async {
                for _ in 0..156 {
                    output.next().await.expect("ended early");
                    recv += 1;
                }
            };
            tokio::join!(sender, receiver);
        })
        .await;
        eprintln!("DIAG flush_each: sent={sent} recv={recv} (of 156)");
        deployment.stop().await.unwrap();
    }

    /// DIAG 3: the decisive backpressure-vs-wedge test. Deploys the probed round
    /// trip and drives the hanging config, but instead of trusting the external
    /// receiver it reads the deployed processes' stdout `[PROBE]` lines to see
    /// how far items actually travel through the flow. Interpretation:
    ///   - counts climb on all three probes  => items flow; NOT a wedge (would
    ///     mean the external receiver / my harness was the problem).
    ///   - `AtoB`/`atB` climb but `backAtA` stalls => the RETURN leg wedges
    ///     inside the flow (a real transport stall, not benign backpressure:
    ///     backpressure throttles a fast producer, it does not take a live
    ///     pipeline to zero return output).
    #[tokio::test]
    #[ignore = "diagnostic"]
    async fn diag_probe() {
        use hydro_lang::deploy::DeployCrateWrapper;

        let mut deployment = Deployment::new();
        let mut builder = hydro_lang::compile::builder::FlowBuilder::new();
        let external = builder.external::<()>();
        let a = builder.process::<()>();
        let b = builder.process::<()>();
        let (input_port, input) = a.source_external_bincode(&external);
        let input = input.weaken_ordering();
        let output_port = super::round_trip_probed(&a, &b, input).send_bincode_external(&external);
        let nodes = builder
            .with_process(&a, deployment.Localhost())
            .with_process(&b, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);
        deployment.deploy().await.unwrap();

        let mut a_out = nodes.get_process(&a).stdout_filter("[PROBE]");
        let mut b_out = nodes.get_process(&b).stdout_filter("[PROBE]");
        let mut input = nodes.connect(input_port).await;
        let mut output = nodes.connect(output_port).await;
        deployment.start().await.unwrap();

        let (a_to_b, at_b, back_at_a) = (
            std::sync::atomic::AtomicUsize::new(0),
            std::sync::atomic::AtomicUsize::new(0),
            std::sync::atomic::AtomicUsize::new(0),
        );
        use std::sync::atomic::Ordering::Relaxed;
        let _ = tokio::time::timeout(Duration::from_secs(20), async {
            let sender = async {
                for _ in 0..156 {
                    input.send(vec![0x5a; 1024]).await.unwrap();
                }
            };
            let drain_out = async { while output.next().await.is_some() {} };
            let probe_a = async {
                while let Some(l) = a_out.recv().await {
                    if l.contains("AtoB") { a_to_b.fetch_add(1, Relaxed); }
                    if l.contains("backAtA") { back_at_a.fetch_add(1, Relaxed); }
                }
            };
            let probe_b = async {
                while let Some(l) = b_out.recv().await {
                    if l.contains("atB") { at_b.fetch_add(1, Relaxed); }
                }
            };
            // Sample at 5 s and 15 s; for a deadlock these are identical (flat).
            let sampler = async {
                for dt in [5u64, 10] {
                    tokio::time::sleep(Duration::from_secs(dt)).await;
                    eprintln!(
                        "DIAG probe: AtoB={} atB={} backAtA={}",
                        a_to_b.load(Relaxed), at_b.load(Relaxed), back_at_a.load(Relaxed),
                    );
                }
            };
            tokio::join!(sender, drain_out, probe_a, probe_b, sampler);
        })
        .await;
        eprintln!(
            "DIAG probe: AtoB={} atB={} backAtA={} (of 156)",
            a_to_b.load(Relaxed),
            at_b.load(Relaxed),
            back_at_a.load(Relaxed),
        );
        deployment.stop().await.unwrap();
    }
}
