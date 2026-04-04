use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};
use dfir_rs::dfir_syntax;
use rand::SeedableRng;
use rand::distributions::{Distribution, Uniform};
use rand::rngs::StdRng;

fn ops(c: &mut Criterion) {
    let mut rng = StdRng::from_entropy();

    c.bench_function("micro/ops/identity", |b| {
        b.iter_batched_ref(
            || {
                const NUM_INTS: usize = 10_000;
                let dist = Uniform::new(0, 100);
                let data: Vec<usize> = (0..NUM_INTS).map(|_| dist.sample(&mut rng)).collect();

                dfir_syntax! {
                    source_iter(black_box(data)) -> identity() -> for_each(|x| { black_box(x); });
                }
            },
            |df| {
                df.run_available_sync();
            },
            BatchSize::LargeInput,
        )
    });

    c.bench_function("micro/ops/unique", |b| {
        b.iter_batched_ref(
            || {
                const NUM_INTS: usize = 10_000;
                let dist = Uniform::new(0, 100);
                let data: Vec<usize> = (0..NUM_INTS).map(|_| dist.sample(&mut rng)).collect();

                dfir_syntax! {
                    source_iter(data) -> unique() -> for_each(|x| { black_box(x); });
                }
            },
            |df| {
                df.run_available_sync();
            },
            BatchSize::LargeInput,
        )
    });

    c.bench_function("micro/ops/map", |b| {
        b.iter_batched_ref(
            || {
                const NUM_INTS: usize = 10_000;
                let dist = Uniform::new(0, 100);
                let data: Vec<usize> = (0..NUM_INTS).map(|_| dist.sample(&mut rng)).collect();

                dfir_syntax! {
                    source_iter(black_box(data)) -> map(|x| x + 1) -> for_each(|x| { black_box(x); });
                }
            },
            |df| {
                df.run_available_sync();
            },
            BatchSize::LargeInput,
        )
    });

    c.bench_function("micro/ops/flat_map", |b| {
        b.iter_batched_ref(
            || {
                const NUM_INTS: usize = 10_000;
                let dist = Uniform::new(0, 100);
                let data: Vec<usize> = (0..NUM_INTS).map(|_| dist.sample(&mut rng)).collect();

                dfir_syntax! {
                    source_iter(black_box(data)) -> flat_map(|x| [x]) -> for_each(|x| { black_box(x); });
                }
            },
            |df| {
                df.run_available_sync();
            },
            BatchSize::LargeInput,
        )
    });
    c.bench_function("micro/ops/flat_map/tracing", |b| {
        b.iter_batched_ref(
            || {
                #[derive(Debug, Copy, Clone, Eq)]
                #[expect(dead_code, reason = "id for testing")]
                struct SpanId(u64);
                // Hack to make the flow reach fixpoint.
                impl PartialOrd for SpanId {
                    fn partial_cmp(&self, _: &Self) -> Option<std::cmp::Ordering> {
                        Some(std::cmp::Ordering::Equal)
                    }
                }
                impl Ord for SpanId {
                    fn cmp(&self, _: &Self) -> std::cmp::Ordering {
                        std::cmp::Ordering::Equal
                    }
                }
                impl PartialEq for SpanId {
                    fn eq(&self, _: &Self) -> bool {
                        true
                    }
                }

                const NUM_INTS: usize = 10_000;
                let dist = Uniform::new(0, 100);
                let data: Vec<(usize, SpanId)> = (0..NUM_INTS).map(|_| dist.sample(&mut rng)).map(|x| (x, SpanId(1))).collect();

                dfir_syntax! {
                    source_iter(black_box(data)) -> flat_map(|(x, span_id)| [(x, span_id)]) -> for_each(|(x, span_id)| { black_box((x, span_id)); });
                }
            },
            |df| {
                df.run_available_sync();
            },
            BatchSize::LargeInput,
        )
    });

    c.bench_function("micro/ops/flat_map2", |b| {
        b.iter_batched_ref(
            || {
                const NUM_INTS: usize = 10_000;
                let dist = Uniform::new(0, 100);
                let data: Vec<usize> = (0..NUM_INTS).map(|_| dist.sample(&mut rng)).collect();

                dfir_syntax! {
                    source_iter(black_box(data)) -> flat_map(|x| 0..x) -> for_each(|x| { black_box(x); });
                }
            },
            |df| {
                df.run_available_sync();
            },
            BatchSize::LargeInput,
        )
    });

    c.bench_function("micro/ops/join", |b| {
        b.iter_batched_ref(
            || {
                const NUM_INTS: usize = 10_000;
                let dist = Uniform::new(0, 100);
                let input0: Vec<(usize, ())> =
                    (0..NUM_INTS).map(|_| (dist.sample(&mut rng), ())).collect();
                let input1: Vec<(usize, ())> =
                    (0..NUM_INTS).map(|_| (dist.sample(&mut rng), ())).collect();

                dfir_syntax! {
                    my_join = join();

                    source_iter(black_box(input0)) -> [0]my_join;
                    source_iter(black_box(input1)) -> [1]my_join;

                    my_join -> for_each(|x| { black_box(x); });
                }
            },
            |df| {
                df.run_available_sync();
            },
            BatchSize::LargeInput,
        )
    });

    c.bench_function("micro/ops/difference", |b| {
        b.iter_batched_ref(
            || {
                const NUM_INTS: usize = 10_000;
                let dist = Uniform::new(0, 100);
                let input0: Vec<(usize, ())> =
                    (0..NUM_INTS).map(|_| (dist.sample(&mut rng), ())).collect();
                let input1: Vec<(usize, ())> =
                    (0..NUM_INTS).map(|_| (dist.sample(&mut rng), ())).collect();

                dfir_syntax! {
                    my_difference = difference();

                    source_iter(black_box(input0)) -> [pos]my_difference;
                    source_iter(black_box(input1)) -> [neg]my_difference;

                    my_difference -> for_each(|x| { black_box(x); });
                }
            },
            |df| {
                df.run_available_sync();
            },
            BatchSize::LargeInput,
        )
    });

    c.bench_function("micro/ops/union", |b| {
        b.iter_batched_ref(
            || {
                const NUM_INTS: usize = 10_000;
                let dist = Uniform::new(0, 100);
                let input0: Vec<usize> = (0..NUM_INTS).map(|_| dist.sample(&mut rng)).collect();
                let input1: Vec<usize> = (0..NUM_INTS).map(|_| dist.sample(&mut rng)).collect();

                dfir_syntax! {
                    my_union = union();

                    source_iter(black_box(input0)) -> my_union;
                    source_iter(black_box(input1)) -> my_union;

                    my_union -> for_each(|x| { black_box(x); });
                }
            },
            |df| {
                df.run_available_sync();
            },
            BatchSize::LargeInput,
        )
    });

    c.bench_function("micro/ops/tee", |b| {
        b.iter_batched_ref(
            || {
                const NUM_INTS: usize = 10_000;
                let dist = Uniform::new(0, 100);
                let input0: Vec<usize> = (0..NUM_INTS).map(|_| dist.sample(&mut rng)).collect();

                dfir_syntax! {
                    my_tee = tee();

                    source_iter(black_box(input0)) -> my_tee;

                    my_tee -> for_each(|x| { black_box(x); });
                    my_tee -> for_each(|x| { black_box(x); });
                }
            },
            |df| {
                df.run_available_sync();
            },
            BatchSize::LargeInput,
        )
    });

    c.bench_function("micro/ops/fold", |b| {
        b.iter_batched_ref(
            || {
                const NUM_INTS: usize = 10_000;
                let dist = Uniform::new(0, 100);
                let input0: Vec<usize> = (0..NUM_INTS).map(|_| dist.sample(&mut rng)).collect();

                {
                    dfir_syntax! {
                        source_iter(black_box(input0)) -> fold::<'tick>(|| 0, |accum: &mut _, elem| { *accum += elem }) -> for_each(|x| { black_box(x); });
                    }
                }
            },
            |df| {
                df.run_available_sync();
            },
            BatchSize::LargeInput,
        )
    });

    c.bench_function("micro/ops/sort", |b| {
        b.iter_batched_ref(
            || {
                const NUM_INTS: usize = 10_000;
                let dist = Uniform::new(0, 100);
                let input0: Vec<usize> = (0..NUM_INTS).map(|_| dist.sample(&mut rng)).collect();

                dfir_syntax! {
                    source_iter(black_box(input0)) -> sort() -> for_each(|x| { black_box(x); });
                }
            },
            |df| {
                df.run_available_sync();
            },
            BatchSize::LargeInput,
        )
    });

    // TODO:
    // This should've been called cross_join to be consistent with the rest of the benchmark names.
    // At some point we will have to edit the benchmark history to give it the correct name.
    c.bench_function("micro/ops/crossjoin", |b| {
        b.iter_batched_ref(
            || {
                const NUM_INTS: usize = 1000;
                let dist = Uniform::new(0, 100);
                let input0: Vec<usize> = (0..NUM_INTS).map(|_| dist.sample(&mut rng)).collect();
                let input1: Vec<usize> = (0..NUM_INTS).map(|_| dist.sample(&mut rng)).collect();

                dfir_syntax! {
                    my_crossjoin = cross_join();

                    source_iter(black_box(input0)) -> [0]my_crossjoin;
                    source_iter(black_box(input1)) -> [1]my_crossjoin;

                    my_crossjoin -> for_each(|x| { black_box(x); });
                }
            },
            |df| {
                df.run_available_sync();
            },
            BatchSize::LargeInput,
        )
    });

    c.bench_function("micro/ops/anti_join", |b| {
        b.iter_batched_ref(
            || {
                const NUM_INTS: usize = 1000;
                let dist = Uniform::new(0, 100);
                let input0: Vec<(usize, ())> =
                    (0..NUM_INTS).map(|_| (dist.sample(&mut rng), ())).collect();
                let input1: Vec<usize> = (0..NUM_INTS).map(|_| dist.sample(&mut rng)).collect();

                dfir_syntax! {
                    my_antijoin = anti_join();

                    source_iter(black_box(input0)) -> [pos]my_antijoin;
                    source_iter(black_box(input1)) -> [neg]my_antijoin;

                    my_antijoin -> for_each(|x| { black_box(x); });
                }
            },
            |df| {
                df.run_available_sync();
            },
            BatchSize::LargeInput,
        )
    });

    c.bench_function("micro/ops/next_tick/small", |b| {
        const DATA: [u64; 1024] = [0; 1024];

        let mut df = dfir_syntax! {
            source_iter(black_box(DATA)) -> persist::<'static>()
                -> map(black_box)
                -> defer_tick()
                -> map(black_box)
                -> defer_tick()
                -> map(black_box)
                -> defer_tick()
                -> map(black_box)
                -> defer_tick()
                -> map(black_box)
                -> defer_tick()
                -> map(black_box)
                -> defer_tick()
                -> map(black_box)
                -> defer_tick()
                -> map(black_box)
                -> defer_tick()
                -> map(black_box)
                -> defer_tick()
                -> map(black_box)
                -> defer_tick()
                -> map(black_box)
                -> for_each(|x| { black_box(x); });
        };

        b.iter(|| {
            df.run_tick_sync();
        })
    });

    c.bench_function("micro/ops/next_tick/big", |b| {
        const DATA: [[u8; 8192]; 1] = [[0; 8192]; 1];

        let mut df = dfir_syntax! {
            source_iter(black_box(DATA)) -> persist::<'static>()
                -> defer_tick()
                -> map(black_box)
                -> defer_tick()
                -> map(black_box)
                -> defer_tick()
                -> map(black_box)
                -> defer_tick()
                -> map(black_box)
                -> defer_tick()
                -> map(black_box)
                -> defer_tick()
                -> map(black_box)
                -> defer_tick()
                -> map(black_box)
                -> defer_tick()
                -> map(black_box)
                -> defer_tick()
                -> map(black_box)
                -> defer_tick()
                -> map(black_box)
                -> for_each(|x| { black_box(x); });
        };

        b.iter(|| {
            df.run_tick_sync();
        })
    });

    // TODO(mingwei): rename to `fold_keyed`
    c.bench_function("micro/ops/group_by", |b| {
        b.iter_batched_ref(
            || {
                const NUM_INTS: usize = 1000;
                let dist = Uniform::new(0, 100);
                let input0: Vec<(usize, usize)> = (0..NUM_INTS)
                    .map(|_| (dist.sample(&mut rng), dist.sample(&mut rng)))
                    .collect();

                dfir_syntax! {
                    source_iter(black_box(input0))
                        -> fold_keyed(|| 0, |x: &mut usize, n: usize| {
                            *x += n;
                        })
                        -> for_each(|x| { black_box(x); });
                }
            },
            |df| {
                df.run_available_sync();
            },
            BatchSize::LargeInput,
        )
    });
}

criterion_group!(micro_ops, ops);
criterion_main!(micro_ops);
