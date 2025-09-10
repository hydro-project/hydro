use std::cell::RefCell;
use std::rc::Rc;

use dfir_rs::scheduled::graph::Dfir;
use dfir_rs::util::multiset::HashMultiSet;
use dfir_rs::{assert_graphvis_snapshots, dfir_syntax};
use multiplatform_test::multiplatform_test;
use tokio::sync::mpsc::error::SendError;

// /// Testing an interesting topology: a self-loop which does nothing.
// /// Doesn't compile due to not knowing what type flows through the empty loop.
// #[test]
// pub fn test_loop() {
//     let mut df = dfir_syntax! {
//         a = identity::<usize>();
//         a -> a;
//     };
// }

/// Basic difference test, test difference between two one-off iterators.
#[multiplatform_test]
pub fn test_difference_a() {
    let output = <Rc<RefCell<HashMultiSet<usize>>>>::default();
    let output_inner = Rc::clone(&output);

    let mut df: Dfir = dfir_syntax! {
        a = difference();
        source_iter([1, 2, 3, 4]) -> [pos]a;
        source_iter([1, 3, 5, 7]) -> [neg]a;
        a -> for_each(|x| output_inner.borrow_mut().insert(x));
    };
    assert_graphvis_snapshots!(df);
    df.run_available_sync();

    assert_eq!(HashMultiSet::from_iter([2, 4]), output.take());
}

/// More complex different test.
/// Take the difference of each tick of items and subtract the previous tick's items.
#[multiplatform_test]
pub fn test_difference_b() -> Result<(), SendError<&'static str>> {
    let (inp_send, inp_recv) = dfir_rs::util::unbounded_channel::<&'static str>();

    let output = <Rc<RefCell<HashMultiSet<&'static str>>>>::default();
    let output_inner = Rc::clone(&output);

    let mut df: Dfir = dfir_syntax! {
        a = difference();
        source_stream(inp_recv) -> [pos]a;
        b = a -> tee();
        b[0] -> defer_tick() -> [neg]a;
        b[1] -> for_each(|x| output_inner.borrow_mut().insert(x));
    };
    assert_graphvis_snapshots!(df);

    inp_send.send("01")?;
    inp_send.send("02")?;
    inp_send.send("03")?;
    df.run_tick_sync();
    assert_eq!(HashMultiSet::from_iter(["01", "02", "03"]), output.take());

    inp_send.send("02")?;
    inp_send.send("11")?;
    inp_send.send("12")?;
    df.run_tick_sync();
    assert_eq!(HashMultiSet::from_iter(["11", "12"]), output.take());

    inp_send.send("02")?;
    inp_send.send("11")?;
    inp_send.send("12")?;
    df.run_tick_sync();
    assert_eq!(HashMultiSet::from_iter(["02"]), output.take());

    Ok(())
}

#[multiplatform_test]
pub fn test_tick_loop_1() {
    let output = <Rc<RefCell<Vec<usize>>>>::default();
    let output_inner = Rc::clone(&output);

    // Without `defer_tick()` this would be "unsafe" although legal.
    // E.g. it would spin forever in a single infinite tick/tick.
    let mut df: Dfir = dfir_syntax! {
        a = union() -> tee();
        source_iter([1, 3]) -> [0]a;
        a[0] -> defer_tick() -> map(|x| 2 * x) -> [1]a;
        a[1] -> for_each(|x| output_inner.borrow_mut().push(x));
    };
    assert_graphvis_snapshots!(df);

    df.run_tick_sync();
    assert_eq!(&[1, 3], &*output.take());

    df.run_tick_sync();
    assert_eq!(&[2, 6], &*output.take());

    df.run_tick_sync();
    assert_eq!(&[4, 12], &*output.take());

    df.run_tick_sync();
    assert_eq!(&[8, 24], &*output.take());
}

#[multiplatform_test]
pub fn test_tick_loop_2() {
    let output = <Rc<RefCell<Vec<usize>>>>::default();
    let output_inner = Rc::clone(&output);

    let mut df: Dfir = dfir_syntax! {
        a = union() -> tee();
        source_iter([1, 3]) -> [0]a;
        a[0] -> defer_tick() -> defer_tick() -> map(|x| 2 * x) -> [1]a;
        a[1] -> for_each(|x| output_inner.borrow_mut().push(x));
    };
    assert_graphvis_snapshots!(df);

    df.run_tick_sync();
    assert_eq!(&[1, 3], &*output.take());

    df.run_tick_sync();
    assert!(output.take().is_empty());

    df.run_tick_sync();
    assert_eq!(&[2, 6], &*output.take());

    df.run_tick_sync();
    assert!(output.take().is_empty());

    df.run_tick_sync();
    assert_eq!(&[4, 12], &*output.take());
}

#[multiplatform_test]
pub fn test_tick_loop_3() {
    let output = <Rc<RefCell<Vec<usize>>>>::default();
    let output_inner = Rc::clone(&output);

    let mut df: Dfir = dfir_syntax! {
        a = union() -> tee();
        source_iter([1, 3]) -> [0]a;
        a[0] -> defer_tick() -> defer_tick() -> defer_tick() -> map(|x| 2 * x) -> [1]a;
        a[1] -> for_each(|x| output_inner.borrow_mut().push(x));
    };
    assert_graphvis_snapshots!(df);

    df.run_tick_sync();
    assert_eq!(&[1, 3], &*output.take());

    df.run_tick_sync();
    assert!(output.take().is_empty());

    df.run_tick_sync();
    assert!(output.take().is_empty());

    df.run_tick_sync();
    assert_eq!(&[2, 6], &*output.take());

    df.run_tick_sync();
    assert!(output.take().is_empty());
}

#[multiplatform_test]
pub fn test_surface_syntax_graph_unreachability() {
    // TODO(mingwei): may need persistence if we want this to make easier to eyeball.

    // An edge in the input data = a pair of `usize` vertex IDs.
    let (pairs_send, pairs_recv) = dfir_rs::util::unbounded_channel::<(usize, usize)>();

    #[expect(clippy::map_identity, reason = "stratification topology testing")]
    let mut df = dfir_syntax! {
        reached_vertices = union() -> map(|v| (v, ()));
        source_iter(vec![0]) -> [0]reached_vertices;

        edges = source_stream(pairs_recv) -> tee();

        my_join_tee = join() -> map(|(_src, ((), dst))| dst) -> map(|x| x) -> map(|x| x) -> tee();
        reached_vertices -> [0]my_join_tee;
        edges[1] -> [1]my_join_tee;

        my_join_tee[0] -> [1]reached_vertices;

        diff = difference() -> for_each(|x| println!("Not reached: {}", x));

        edges[0] -> flat_map(|(a, b)| [a, b]) -> [pos]diff;
        my_join_tee[1] -> [neg]diff;
    };
    assert_graphvis_snapshots!(df);
    df.run_available_sync();

    println!("A");

    pairs_send.send((0, 1)).unwrap();
    pairs_send.send((2, 4)).unwrap();
    pairs_send.send((3, 5)).unwrap();
    pairs_send.send((1, 2)).unwrap();
    df.run_available_sync();

    // println!("B");

    // pairs_send.send((0, 3)).unwrap();
    // df.run_available_sync();
}

/// Test that subgraphs are in the same stratum when possible.
#[multiplatform_test]
pub fn test_subgraph_stratum_consolidation() {
    let output = <Rc<RefCell<Vec<usize>>>>::default();
    let output_inner = Rc::clone(&output);

    // Bunch of triangles generate consecutive subgraphs, but since there are
    // no negative edges they can all be in the same stratum.
    let mut df: Dfir = dfir_syntax! {
        a = union() -> tee();
        b = union() -> tee();
        c = union() -> tee();
        d = union() -> for_each(|x| output_inner.borrow_mut().push(x));
        source_iter([0]) -> [0]a[0] -> [0]b[0] -> [0]c[0] -> [0]d;
        source_iter([1]) -> [1]a[1] -> [1]b[1] -> [1]c[1] -> [1]d;
    };
    assert_graphvis_snapshots!(df);

    df.run_available_sync();
    assert_eq!(2 * usize::pow(2, 3), output.take().len());

    df.run_available_sync();
    assert!(output.take().is_empty());
}

#[multiplatform_test]
pub fn test_defer_lazy() {
    let output = <Rc<RefCell<Vec<usize>>>>::default();
    let output_inner = Rc::clone(&output);

    // Without `defer()` this would spin forever with run_available().
    let mut df: Dfir = dfir_syntax! {
        a = union() -> tee();
        source_iter([1, 3]) -> [0]a;
        a[0] -> defer_tick_lazy() -> map(|x| 2 * x) -> [1]a;
        a[1] -> for_each(|x| output_inner.borrow_mut().push(x));
    };
    println!(
        "{}",
        df.meta_graph().unwrap().to_mermaid(&Default::default())
    );

    assert_graphvis_snapshots!(df);

    df.run_available_sync();
    assert_eq!(&[1, 3], &*output.take());

    df.run_available_sync();
    assert_eq!(&[2, 6], &*output.take());

    df.run_available_sync();
    assert_eq!(&[4, 12], &*output.take());

    df.run_available_sync();
    assert_eq!(&[8, 24], &*output.take());
}
