use criterion::{Criterion, black_box, criterion_group, criterion_main};
use dfir_rs::dfir_syntax;
use dfir_rs::scheduled::graph::Dfir;
use dfir_rs::scheduled::graph_ext::GraphExt;
use dfir_rs::scheduled::handoff::{Iter, VecHandoff};
use timely::dataflow::operators::{Concatenate, Filter, Inspect, ToStream};

const NUM_OPS: usize = 20;
const NUM_INTS: usize = 100_000;
const BRANCH_FACTOR: usize = 2;

fn benchmark_hydroflow(c: &mut Criterion) {
    c.bench_function("fork_join/dfir_rs", |b| {
        b.iter(|| {
            let mut df = Dfir::new();

            let (start_send, start_recv) = df.make_edge::<_, VecHandoff<usize>>("start");

            let mut sent = false;
            df.add_subgraph_source("source", start_send, move |_ctx, send| {
                if !sent {
                    sent = true;
                    send.give(Iter(0..NUM_INTS));
                }
            });

            let (send1, mut recv1) = df.make_edge::<_, VecHandoff<_>>("1");
            let (send2, mut recv2) = df.make_edge::<_, VecHandoff<_>>("2");

            df.add_subgraph_in_2out(
                "fork",
                start_recv,
                send1,
                send2,
                |_ctx, recv, send1, send2| {
                    for v in recv.take_inner() {
                        if v % 2 == 0 {
                            send1.give(Some(v));
                        } else {
                            send2.give(Some(v));
                        }
                    }
                },
            );

            for _ in 0..NUM_OPS {
                let (send1, next_recv1) = df.make_edge("1");
                let (send2, next_recv2) = df.make_edge("2");

                df.add_subgraph_2in_2out(
                    "join-fork",
                    recv1,
                    recv2,
                    send1,
                    send2,
                    |_ctx, recv1, recv2, send1, send2| {
                        for v in recv1.take_inner().into_iter().chain(recv2.take_inner()) {
                            if v % 2 == 0 {
                                send1.give(Some(v));
                            } else {
                                send2.give(Some(v));
                            }
                        }
                    },
                );

                recv1 = next_recv1;
                recv2 = next_recv2;
            }

            df.add_subgraph_2sink("join (union)", recv1, recv2, |_ctx, recv1, recv2| {
                for x in recv1.take_inner() {
                    black_box(x);
                }
                for x in recv2.take_inner() {
                    black_box(x);
                }
            });

            df.run_available_sync();
        })
    });
}

fn benchmark_hydroflow_surface(c: &mut Criterion) {
    c.bench_function("fork_join/dfir_rs/surface", |b| {
        b.iter(|| {
            let mut hf = include!("fork_join_20.hf");
            hf.run_available_sync();
        })
    });
}

fn benchmark_raw(c: &mut Criterion) {
    c.bench_function("fork_join/raw", |b| {
        b.iter(|| {
            let mut parts = [(); BRANCH_FACTOR].map(|_| Vec::new());
            let mut data: Vec<_> = (0..NUM_INTS).collect();

            for _ in 0..NUM_OPS {
                for i in data.drain(..) {
                    parts[i % BRANCH_FACTOR].push(i);
                }

                for part in parts.iter_mut() {
                    data.append(part);
                }
            }
        })
    });
}

fn benchmark_timely(c: &mut Criterion) {
    c.bench_function("fork_join/timely", |b| {
        b.iter(|| {
            timely::example(|scope| {
                let mut op = (0..NUM_INTS).to_stream(scope);
                for _ in 0..NUM_OPS {
                    let mut ops = Vec::new();

                    for i in 0..BRANCH_FACTOR {
                        ops.push(op.filter(move |x| x % BRANCH_FACTOR == i))
                    }

                    op = scope.concatenate(ops);
                }

                op.inspect(|i| {
                    black_box(i);
                });
            });
        })
    });
}

criterion_group!(
    fork_join_dataflow,
    benchmark_hydroflow,
    benchmark_hydroflow_surface,
    benchmark_timely,
    benchmark_raw,
);
criterion_main!(fork_join_dataflow);
