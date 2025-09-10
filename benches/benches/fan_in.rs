use criterion::{Criterion, black_box, criterion_group, criterion_main};
use dfir_rs::dfir_syntax;
use static_assertions::const_assert;
use timely::dataflow::operators::{Concatenate, Inspect, ToStream};

const NUM_OPS: usize = 20;
const NUM_INTS: usize = 1_000_000;

fn make_ints(i: usize) -> impl Iterator<Item = usize> {
    (i * NUM_INTS)..((i + 1) * NUM_INTS)
}

fn benchmark_hydroflow_surface(c: &mut Criterion) {
    const_assert!(NUM_OPS == 20); // This benchmark is hardcoded for 20 ops, so assert that NUM_OPS is 20.
    c.bench_function("fan_in/dfir_rs/surface", |b| {
        b.iter(|| {
            let mut df = dfir_syntax! {

                my_union = union();

                source_iter(0..NUM_INTS) -> my_union;
                source_iter(0..NUM_INTS) -> my_union;
                source_iter(0..NUM_INTS) -> my_union;
                source_iter(0..NUM_INTS) -> my_union;
                source_iter(0..NUM_INTS) -> my_union;

                source_iter(0..NUM_INTS) -> my_union;
                source_iter(0..NUM_INTS) -> my_union;
                source_iter(0..NUM_INTS) -> my_union;
                source_iter(0..NUM_INTS) -> my_union;
                source_iter(0..NUM_INTS) -> my_union;

                source_iter(0..NUM_INTS) -> my_union;
                source_iter(0..NUM_INTS) -> my_union;
                source_iter(0..NUM_INTS) -> my_union;
                source_iter(0..NUM_INTS) -> my_union;
                source_iter(0..NUM_INTS) -> my_union;

                source_iter(0..NUM_INTS) -> my_union;
                source_iter(0..NUM_INTS) -> my_union;
                source_iter(0..NUM_INTS) -> my_union;
                source_iter(0..NUM_INTS) -> my_union;
                source_iter(0..NUM_INTS) -> my_union;

                my_union -> for_each(|x| { black_box(x); });
            };

            df.run_available_sync();
        })
    });
}

fn benchmark_timely(c: &mut Criterion) {
    c.bench_function("fan_in/timely", |b| {
        b.iter(|| {
            timely::example(move |scope| {
                let sources: Vec<_> = (0..NUM_OPS)
                    .map(|i| make_ints(i).to_stream(scope))
                    .collect();

                let merged = scope.concatenate(sources);

                merged.inspect(|x| {
                    black_box(x);
                });
            });
        })
    });
}

fn benchmark_iters(c: &mut Criterion) {
    c.bench_function("fan_in/iters", |b| {
        b.iter(|| {
            (0..NUM_OPS).flat_map(make_ints).for_each(|x| {
                black_box(x);
            });
        });
    });
}

fn benchmark_for_loops(c: &mut Criterion) {
    c.bench_function("fan_in/loops", |b| {
        b.iter(|| {
            let iters: Vec<_> = (0..NUM_OPS).map(make_ints).collect();
            for iter in iters {
                for x in iter {
                    black_box(x);
                }
            }
        });
    });
}

criterion_group!(
    fan_in_dataflow,
    benchmark_hydroflow_surface,
    benchmark_timely,
    benchmark_iters,
    benchmark_for_loops,
);

// use pprof::criterion::{Output, PProfProfiler};
// criterion_group!(
//     name = fan_in_dataflow;
//     config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
//     targets =
//         benchmark_babyflow,
//         benchmark_hydroflow,
//         benchmark_timely,
//         benchmark_spinachflow,
//         benchmark_iters,
//         benchmark_for_loops,
// );
criterion_main!(fan_in_dataflow);
