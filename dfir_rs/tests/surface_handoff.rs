//! Tests for the `handoff()` pseudo-operator.

use dfir_rs::assert_graphvis_snapshots;

/// Test: handoff() pseudo-operator forces a subgraph boundary.
#[dfir_rs::test]
pub async fn test_handoff_basic() {
    let mut output = Vec::<i32>::new();
    let out = &mut output;
    let mut flow = dfir_rs::dfir_syntax! {
        source_iter(0..5_i32) -> handoff() -> for_each(|v: i32| out.push(v));
    };
    assert_graphvis_snapshots!(flow);
    flow.run_tick().await;
    drop(flow);
    assert_eq!(vec![0, 1, 2, 3, 4], output);
}

/// Test: handoff() in the middle of a pipeline with transforms on both sides.
#[dfir_rs::test]
pub async fn test_handoff_mid_pipeline() {
    let mut output = Vec::<i32>::new();
    let out = &mut output;
    let mut flow = dfir_rs::dfir_syntax! {
        source_iter(0..5_i32)
            -> map(|x| x * 2)
            -> handoff()
            -> filter(|x: &i32| *x > 4)
            -> for_each(|v: i32| out.push(v));
    };
    assert_graphvis_snapshots!(flow);
    flow.run_tick().await;
    drop(flow);
    assert_eq!(vec![6, 8], output);
}
