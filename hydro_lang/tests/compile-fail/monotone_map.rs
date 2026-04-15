use hydro_lang::prelude::*;

struct P1 {}

fn test<'a>(p1: &Process<'a, P1>) {
    let unbounded: Stream<_, _> = p1.source_iter(q!(0..10)).into();
    let is_monotone = unbounded.fold(
        q!(|| 0),
        q!(
            |sum, v| {
                *sum += v;
            },
            monotone = manual_proof!(/** test */)
        ),
    );

    let no_longer_monotone = is_monotone.map(q!(|x| -x));
    let _ = no_longer_monotone.threshold_greater_or_equal(p1.singleton(q!(0)));
}

fn test_keyed<'a>(p1: &Process<'a, P1>) {
    use hydro_lang::live_collections::keyed_singleton::MonotonicValue;

    let unbounded: KeyedStream<_, _, _> = p1.source_iter(q!([(0, 1), (1, 2)])).into_keyed().into();
    let is_monotone: KeyedSingleton<_, _, _, MonotonicValue> = unbounded.fold(
        q!(|| 0),
        q!(
            |sum, v| {
                *sum += v;
            },
            monotone = manual_proof!(/** test */)
        ),
    );

    let _: KeyedSingleton<_, _, _, MonotonicValue> = is_monotone.map(q!(|x| -x));
}

fn main() {}
