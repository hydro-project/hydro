use std::sync::mpsc::channel;
use std::thread;

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use dfir_rs::dfir_syntax;
use dfir_rs::scheduled::graph_ext::GraphExt;
use static_assertions::const_assert;
use timely::dataflow::operators::{Inspect, Map, ToStream};

const NUM_OPS: usize = 20;
const NUM_INTS: usize = 1_000_000;

fn benchmark_pipeline(c: &mut Criterion) {
    c.bench_function("identity/pipeline", |b| {
        b.iter(|| {
            let (input, mut output) = channel();

            for _ in 0..NUM_OPS {
                let (tx, mut rx) = channel();
                std::mem::swap(&mut output, &mut rx);
                thread::spawn(move || {
                    for elt in rx {
                        tx.send(elt).unwrap();
                    }
                });
            }

            for i in 0..NUM_INTS {
                input.send(i).unwrap();
            }
            drop(input);
            for elt in output {
                black_box(elt);
            }
        });
    });
}

// This benchmark just copies around a bunch of data with basically zero
// overhead, so this should theoretically be the fastest achievable (with a
// single thread).
fn benchmark_raw_copy(c: &mut Criterion) {
    c.bench_function("identity/raw", |b| {
        b.iter(|| {
            let mut data: Vec<_> = (0..NUM_INTS).collect();
            let mut next = Vec::new();

            for _ in 0..NUM_OPS {
                next.append(&mut data);
                std::mem::swap(&mut data, &mut next);
            }

            for elt in data {
                black_box(elt);
            }
        })
    });
}

fn benchmark_iter(c: &mut Criterion) {
    c.bench_function("identity/iter", |b| {
        b.iter(|| {
            let iter = 0..NUM_INTS;

            ///// MAGIC NUMBER!!!!!!!! is NUM_OPS
            seq_macro::seq!(_ in 0..20 {
                let iter = iter.map(black_box);
            });

            let data: Vec<_> = iter.collect();

            for elt in data {
                black_box(elt);
            }
        });
    });
}

fn benchmark_iter_collect(c: &mut Criterion) {
    c.bench_function("identity/iter-collect", |b| {
        b.iter(|| {
            let mut data: Vec<_> = (0..NUM_INTS).collect();

            for _ in 0..NUM_OPS {
                let iter = data.into_iter();
                let iter = iter.map(black_box);
                data = iter.collect();
            }

            for elt in data {
                black_box(elt);
            }
        });
    });
}

fn benchmark_timely(c: &mut Criterion) {
    c.bench_function("identity/timely", |b| {
        b.iter(|| {
            timely::example(|scope| {
                let mut op = (0..NUM_INTS).to_stream(scope);
                for _ in 0..NUM_OPS {
                    op = op.map(black_box)
                }

                op.inspect(|i| {
                    black_box(i);
                });
            });
        })
    });
}

fn benchmark_hydroflow_compiled(c: &mut Criterion) {
    use pusherator::{InputBuild, Pusherator, PusheratorBuild};

    c.bench_function("identity/dfir_rs/compiled", |b| {
        b.iter(|| {
            let mut pusherator = InputBuild::<usize>::new()
                .map(black_box)
                .map(black_box)
                .map(black_box)
                .map(black_box)
                .map(black_box)
                .map(black_box)
                .map(black_box)
                .map(black_box)
                .map(black_box)
                .map(black_box)
                .map(black_box)
                .map(black_box)
                .map(black_box)
                .map(black_box)
                .map(black_box)
                .map(black_box)
                .map(black_box)
                .map(black_box)
                .map(black_box)
                .map(black_box)
                .for_each(|x| {
                    black_box(x);
                });

            for i in 0..NUM_INTS {
                pusherator.give(i);
            }
        });
    });
}

fn benchmark_hydroflow(c: &mut Criterion) {
    use dfir_rs::scheduled::graph::Dfir;
    use dfir_rs::scheduled::handoff::{Iter, VecHandoff};

    c.bench_function("identity/dfir_rs", |b| {
        b.iter(|| {
            let mut df = Dfir::new();

            let (next_send, mut next_recv) = df.make_edge::<_, VecHandoff<usize>>("end");

            let mut sent = false;
            df.add_subgraph_source("source", next_send, move |_ctx, send| {
                if !sent {
                    sent = true;
                    send.give(Iter(0..NUM_INTS));
                }
            });
            for _ in 0..NUM_OPS {
                let (next_send, next_next_recv) = df.make_edge("handoff");

                df.add_subgraph_in_out("identity", next_recv, next_send, |_ctx, recv, send| {
                    send.give(Iter(recv.take_inner().into_iter()));
                });

                next_recv = next_next_recv;
            }

            df.add_subgraph_sink("sink", next_recv, |_ctx, recv| {
                for x in recv.take_inner() {
                    black_box(x);
                }
            });

            df.run_available_sync();
        });
    });
}

fn benchmark_hydroflow_surface(c: &mut Criterion) {
    const_assert!(NUM_OPS == 20); // This benchmark is hardcoded for 20 ops, so assert that NUM_OPS is 20.
    c.bench_function("identity/dfir_rs/surface", |b| {
        b.iter(|| {
            let mut df = dfir_syntax! {
                source_iter(black_box(0..NUM_INTS))

                -> map(black_box)
                -> map(black_box)
                -> map(black_box)
                -> map(black_box)
                -> map(black_box)
                -> map(black_box)
                -> map(black_box)
                -> map(black_box)
                -> map(black_box)
                -> map(black_box)

                -> map(black_box)
                -> map(black_box)
                -> map(black_box)
                -> map(black_box)
                -> map(black_box)
                -> map(black_box)
                -> map(black_box)
                -> map(black_box)
                -> map(black_box)
                -> map(black_box)

                -> for_each(|x| { black_box(x); });
            };

            df.run_available_sync();
        })
    });
}

criterion_group!(
    identity_dataflow,
    benchmark_timely,
    benchmark_pipeline,
    benchmark_iter,
    benchmark_iter_collect,
    benchmark_raw_copy,
    benchmark_hydroflow,
    benchmark_hydroflow_compiled,
    benchmark_hydroflow_surface,
);
criterion_main!(identity_dataflow);
