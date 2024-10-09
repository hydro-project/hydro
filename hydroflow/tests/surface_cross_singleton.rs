use hydroflow::util::collect_ready;
use hydroflow::{assert_graphvis_snapshots, hydroflow_syntax};
use multiplatform_test::multiplatform_test;

#[multiplatform_test]
pub fn test_basic() {
    let (single_tx, single_rx) = hydroflow::util::unbounded_channel::<()>();
    let (egress_tx, mut egress_rx) = hydroflow::util::unbounded_channel();

    let mut df = hydroflow_syntax! {
        join = cross_singleton();
        source_iter([1, 2, 3]) -> persist::<'static>() -> [input]join;
        source_stream(single_rx) -> [single]join;

        join -> for_each(|x| egress_tx.send(x).unwrap());
    };
    assert_graphvis_snapshots!(df);

    df.run_available();
    let out: Vec<_> = collect_ready(&mut egress_rx);
    assert_eq!(out, []);

    single_tx.send(()).unwrap();
    df.run_available();

    let out: Vec<_> = collect_ready(&mut egress_rx);
    assert_eq!(out, vec![(1, ()), (2, ()), (3, ())]);
}
