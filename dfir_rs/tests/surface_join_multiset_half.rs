use dfir_rs::dfir_syntax;
use dfir_rs::util::{collect_ready, unbounded_channel};
use multiplatform_test::multiplatform_test;

#[multiplatform_test]
pub fn test_join_multiset_half_basic() {
    let (build_send, build_recv) = unbounded_channel::<(&str, char)>();
    let (probe_send, probe_recv) = unbounded_channel::<(&str, i32)>();
    let (out_send, mut out_recv) = unbounded_channel::<(&str, (i32, char))>();
    let mut flow = dfir_syntax! {
        source_stream(build_recv) -> [build]my_join;
        source_stream(probe_recv) -> [probe]my_join;
        my_join = join_multiset_half() -> for_each(|x| out_send.send(x).unwrap());
    };

    // Build side: cat->x, dog->y
    build_send.send(("cat", 'x')).unwrap();
    build_send.send(("dog", 'y')).unwrap();
    // Probe side: cat->1, dog->2, cat->3
    probe_send.send(("cat", 1)).unwrap();
    probe_send.send(("dog", 2)).unwrap();
    probe_send.send(("cat", 3)).unwrap();

    flow.run_tick_sync();
    let mut out: Vec<_> = collect_ready(&mut out_recv);
    out.sort();
    assert_eq!(
        out,
        vec![("cat", (1, 'x')), ("cat", (3, 'x')), ("dog", (2, 'y'))]
    );
}

#[multiplatform_test]
pub fn test_join_multiset_half_preserves_probe_order() {
    let (build_send, build_recv) = unbounded_channel::<(i32, char)>();
    let (probe_send, probe_recv) = unbounded_channel::<(i32, i32)>();
    let (out_send, mut out_recv) = unbounded_channel::<(i32, (i32, char))>();
    let mut flow = dfir_syntax! {
        source_stream(build_recv) -> [build]my_join;
        source_stream(probe_recv) -> [probe]my_join;
        my_join = join_multiset_half() -> for_each(|x| out_send.send(x).unwrap());
    };

    // Build side available first (stratum-delayed)
    build_send.send((1, 'a')).unwrap();
    build_send.send((2, 'b')).unwrap();

    // Probe side arrives in specific order
    probe_send.send((2, 20)).unwrap();
    probe_send.send((1, 10)).unwrap();

    flow.run_tick_sync();
    let out: Vec<_> = collect_ready(&mut out_recv);
    // Output should follow probe order: key 2 first, then key 1
    assert_eq!(out, vec![(2, (20, 'b')), (1, (10, 'a'))]);
}

#[multiplatform_test]
pub fn test_join_multiset_half_no_match() {
    let (build_send, build_recv) = unbounded_channel::<(i32, char)>();
    let (probe_send, probe_recv) = unbounded_channel::<(i32, i32)>();
    let (out_send, mut out_recv) = unbounded_channel::<(i32, (i32, char))>();
    let mut flow = dfir_syntax! {
        source_stream(build_recv) -> [build]my_join;
        source_stream(probe_recv) -> [probe]my_join;
        my_join = join_multiset_half() -> for_each(|x| out_send.send(x).unwrap());
    };

    build_send.send((1, 'a')).unwrap();
    probe_send.send((2, 20)).unwrap(); // no match

    flow.run_tick_sync();
    let out: Vec<_> = collect_ready(&mut out_recv);
    assert_eq!(out, vec![]);
}

#[multiplatform_test]
pub fn test_join_multiset_half_multiple_build_values() {
    let (build_send, build_recv) = unbounded_channel::<(i32, char)>();
    let (probe_send, probe_recv) = unbounded_channel::<(i32, i32)>();
    let (out_send, mut out_recv) = unbounded_channel::<(i32, (i32, char))>();
    let mut flow = dfir_syntax! {
        source_stream(build_recv) -> [build]my_join;
        source_stream(probe_recv) -> [probe]my_join;
        my_join = join_multiset_half() -> for_each(|x| out_send.send(x).unwrap());
    };

    // Multiple values for same key on build side
    build_send.send((1, 'a')).unwrap();
    build_send.send((1, 'b')).unwrap();
    probe_send.send((1, 10)).unwrap();

    flow.run_tick_sync();
    let mut out: Vec<_> = collect_ready(&mut out_recv);
    out.sort();
    assert_eq!(out, vec![(1, (10, 'a')), (1, (10, 'b'))]);
}

#[multiplatform_test]
pub fn test_join_multiset_half_tick_static() {
    let (build_send, build_recv) = unbounded_channel::<(i32, char)>();
    let (probe_send, probe_recv) = unbounded_channel::<(i32, i32)>();
    let (out_send, mut out_recv) = unbounded_channel::<(i32, (i32, char))>();
    let mut flow = dfir_syntax! {
        source_stream(build_recv) -> [build]my_join;
        source_stream(probe_recv) -> [probe]my_join;
        my_join = join_multiset_half::<'static, 'tick>() -> for_each(|x| out_send.send(x).unwrap());
    };

    // Tick 1
    build_send.send((1, 'a')).unwrap();
    probe_send.send((1, 10)).unwrap();
    flow.run_tick_sync();
    let mut out: Vec<_> = collect_ready(&mut out_recv);
    out.sort();
    assert_eq!(out, vec![(1, (10, 'a'))]);

    // Tick 2: build side persists ('static), new probe items
    probe_send.send((1, 20)).unwrap();
    flow.run_tick_sync();
    let mut out: Vec<_> = collect_ready(&mut out_recv);
    out.sort();
    assert_eq!(out, vec![(1, (20, 'a'))]);
}

#[multiplatform_test]
pub fn test_join_multiset_half_probe_does_not_persist() {
    // Regression: with swapped persistence args, the probe side would
    // incorrectly get 'static persistence and replay old probe items.
    let (build_send, build_recv) = unbounded_channel::<(i32, char)>();
    let (probe_send, probe_recv) = unbounded_channel::<(i32, i32)>();
    let (out_send, mut out_recv) = unbounded_channel::<(i32, (i32, char))>();
    let mut flow = dfir_syntax! {
        source_stream(build_recv) -> [build]my_join;
        source_stream(probe_recv) -> [probe]my_join;
        my_join = join_multiset_half::<'static, 'tick>() -> for_each(|x| out_send.send(x).unwrap());
    };

    // Tick 1
    build_send.send((1, 'a')).unwrap();
    probe_send.send((1, 10)).unwrap();
    flow.run_tick_sync();
    let out: Vec<_> = collect_ready(&mut out_recv);
    assert_eq!(out, vec![(1, (10, 'a'))]);

    // Tick 2: only new build data, no new probe data.
    // Probe is 'tick so old probe item (1, 10) should NOT replay.
    build_send.send((1, 'b')).unwrap();
    flow.run_tick_sync();
    let out: Vec<_> = collect_ready(&mut out_recv);
    assert_eq!(out, vec![]);
}
