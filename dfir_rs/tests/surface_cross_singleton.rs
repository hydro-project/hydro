use dfir_rs::util::collect_ready;
use dfir_rs::{assert_graphvis_snapshots, dfir_syntax};
use multiplatform_test::multiplatform_test;

#[multiplatform_test(test, wasm, env_tracing)]
pub fn test_basic() {
    let (single_tx, single_rx) = dfir_rs::util::unbounded_channel::<()>();
    let (egress_tx, mut egress_rx) = dfir_rs::util::unbounded_channel();

    let mut df = dfir_syntax! {
        join = cross_singleton();
        source_iter([1, 2, 3]) -> persist::<'static>() -> [input]join;
        source_stream(single_rx) -> [single]join;

        join -> for_each(|x| egress_tx.send(x).unwrap());
    };
    assert_graphvis_snapshots!(df);

    df.run_available_sync();
    let out: Vec<_> = collect_ready(&mut egress_rx);
    assert_eq!(out, []);

    single_tx.send(()).unwrap();
    df.run_available_sync();

    let out: Vec<_> = collect_ready(&mut egress_rx);
    assert_eq!(out, vec![(1, ()), (2, ()), (3, ())]);
}

#[multiplatform_test(test, wasm, env_tracing)]
pub fn test_union_defer_tick() {
    let (cross_tx, cross_rx) = dfir_rs::util::unbounded_channel::<i32>();
    let (egress_tx, mut egress_rx) = dfir_rs::util::unbounded_channel();

    let mut df = dfir_syntax! {
        teed_in = source_stream(cross_rx) -> sort() -> tee();
        teed_in -> [input]join;

        deferred_stream -> defer_tick_lazy() -> [0]unioned_stream;

        persisted_stream = source_iter([0]) -> persist::<'static>();
        persisted_stream -> [1]unioned_stream;

        unioned_stream = union();
        unioned_stream -> [single]join;

        join = cross_singleton() -> tee();

        join -> for_each(|x| egress_tx.send(x).unwrap());

        folded_thing = join -> fold(|| 0, |_, _| {});

        teed_in -> [input]joined_folded;
        folded_thing -> [single]joined_folded;
        joined_folded = cross_singleton();
        deferred_stream = joined_folded -> fold(|| 0, |_, _| {}) -> flat_map(|_| []);
    };
    assert_graphvis_snapshots!(df);

    df.run_available_sync();
    let out: Vec<_> = collect_ready(&mut egress_rx);
    assert_eq!(out, vec![]);

    cross_tx.send(1).unwrap();
    cross_tx.send(2).unwrap();
    cross_tx.send(3).unwrap();
    df.run_available_sync();

    let out: Vec<_> = collect_ready(&mut egress_rx);
    assert_eq!(out, vec![(1, 0), (2, 0), (3, 0)]);
}
