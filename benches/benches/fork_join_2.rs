use std::sync::OnceLock;

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use dfir_rs::dfir_syntax;
use dfir_rs::scheduled::graph::Dfir;
use dfir_rs::scheduled::graph_ext::GraphExt;
use dfir_rs::scheduled::handoff::{Iter, VecHandoff};
use timely::dataflow::operators::{Concatenate, Filter, Inspect, ToStream};

const NUM_OPS: usize = 20;
const NUM_INTS: usize = 100_000;

fn vals() -> impl Iterator<Item = usize> {
    static VALS: OnceLock<Vec<usize>> = OnceLock::new();
    VALS.get_or_init(|| {
        let mut rng = <rand::rngs::SmallRng as rand::SeedableRng>::seed_from_u64(5938450);
        let mut vals = Vec::with_capacity(NUM_INTS);
        for _ in 0..NUM_INTS {
            vals.push(rand::Rng::gen_range(&mut rng, 0..(1 << (NUM_OPS + 3))));
        }
        vals
    })
    .iter()
    .copied()
}

fn benchmark_hydroflow(c: &mut Criterion) {
    c.bench_function("fork_join_2/dfir_rs", |b| {
        let _init = vals();

        b.iter(|| {
            let mut df = Dfir::new();

            let (start_send, start_recv) = df.make_edge::<_, VecHandoff<usize>>("start");

            let mut sent = false;
            df.add_subgraph_source("source", start_send, move |_ctx, send| {
                if !sent {
                    sent = true;
                    send.give(Iter(vals()));
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

            for i in 1..=NUM_OPS {
                let (send1, next_recv1) = df.make_edge("1");
                let (send2, next_recv2) = df.make_edge("2");

                df.add_subgraph_2in_2out(
                    "join-fork",
                    recv1,
                    recv2,
                    send1,
                    send2,
                    move |_ctx, recv1, recv2, send1, send2| {
                        for v in recv1.take_inner().into_iter().chain(recv2.take_inner()) {
                            if (v >> i) & 0b1 == 0 {
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
    c.bench_function("fork_join_2/dfir_rs/surface", |b| {
        let _init = vals();

        b.iter(|| {
            let mut hf = include!("fork_join_2.hf");
            hf.run_available_sync();
        })
    });
}

fn benchmark_hydroflow_surface_tracing(c: &mut Criterion) {
    c.bench_function("fork_join_2/dfir_rs/surface/tracing", |b| {
        let _init = vals();

        #[derive(Debug, Copy, Clone)]
        #[expect(dead_code, reason = "id for testing")]
        struct SpanId(u64);

        b.iter(|| {
            use std::cell::{Cell, RefCell};
            use std::rc::Rc;

            let span_id = Rc::new(Cell::new(3));
            let follows_from = Rc::new(RefCell::new(Vec::new()));

            let span_follows = {
                let span_id = Rc::clone(&span_id);
                let follows_from = Rc::clone(&follows_from);
                move |a: SpanId| -> SpanId {
                    let out = span_id.get();
                    span_id.set(out + 1);
                    follows_from.borrow_mut().push((out, a));
                    SpanId(out)
                }
            };
            let span_follows = &span_follows;

            let mut hf = include!("fork_join_2_tracing.hfx");
            hf.run_available_sync();
        })
    });
}

fn benchmark_raw(c: &mut Criterion) {
    c.bench_function("fork_join_2/raw", |b| {
        let _init = vals();

        b.iter(|| {
            let mut parts = [Vec::new(), Vec::new()];
            let mut data: Vec<_> = vals().collect();

            for i in 1..=NUM_OPS {
                for x in data.drain(..) {
                    parts[(x >> i) & 0b1].push(i);
                }

                for part in parts.iter_mut() {
                    data.append(part);
                }
            }
        })
    });
}

fn benchmark_timely(c: &mut Criterion) {
    c.bench_function("fork_join_2/timely", |b| {
        let _init = vals();

        b.iter(|| {
            timely::example(|scope| {
                let mut op = vals().to_stream(scope);
                for i in 1..=NUM_OPS {
                    let mut ops = Vec::new();

                    ops.push(op.filter(move |x| (x >> i) & 0b1 == 0));
                    ops.push(op.filter(move |x| (x >> i) & 0b1 == 1));

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
    fork_join_2,
    benchmark_hydroflow,
    benchmark_hydroflow_surface,
    benchmark_hydroflow_surface_tracing,
    benchmark_timely,
    benchmark_raw,
);
criterion_main!(fork_join_2);
