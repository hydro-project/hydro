use hydroflow::{assert_graphvis_snapshots, hydroflow_syntax};
use multiplatform_test::multiplatform_test;

#[multiplatform_test]
pub fn test_reduce_tick() {
    let (items_send, items_recv) = hydroflow::util::unbounded_channel::<u32>();
    let (result_send, mut result_recv) = hydroflow::util::unbounded_channel::<u32>();

    let mut df = hydroflow::hydroflow_syntax! {
        source_stream(items_recv)
            -> reduce::<'tick>(|acc: u32, next: u32| acc + next)
            -> for_each(|v| result_send.send(v).unwrap());
    };
    assert_graphvis_snapshots!(df);
    assert_eq!((0, 0), (df.current_tick(), df.current_stratum()));

    items_send.send(1).unwrap();
    items_send.send(2).unwrap();
    df.run_tick();

    assert_eq!((1, 0), (df.current_tick(), df.current_stratum()));
    assert_eq!(
        &[3],
        &*hydroflow::util::collect_ready::<Vec<_>, _>(&mut result_recv)
    );

    items_send.send(3).unwrap();
    items_send.send(4).unwrap();
    df.run_tick();

    assert_eq!((2, 0), (df.current_tick(), df.current_stratum()));
    assert_eq!(
        &[7],
        &*hydroflow::util::collect_ready::<Vec<_>, _>(&mut result_recv)
    );
}

#[multiplatform_test]
pub fn test_reduce_static() {
    let (items_send, items_recv) = hydroflow::util::unbounded_channel::<u32>();
    let (result_send, mut result_recv) = hydroflow::util::unbounded_channel::<u32>();

    let mut df = hydroflow::hydroflow_syntax! {
        source_stream(items_recv)
            -> reduce::<'static>(|acc: u32, next: u32| acc + next)
            -> for_each(|v| result_send.send(v).unwrap());
    };
    assert_graphvis_snapshots!(df);
    assert_eq!((0, 0), (df.current_tick(), df.current_stratum()));

    items_send.send(1).unwrap();
    items_send.send(2).unwrap();
    df.run_tick();

    assert_eq!((1, 0), (df.current_tick(), df.current_stratum()));
    assert_eq!(
        &[3],
        &*hydroflow::util::collect_ready::<Vec<_>, _>(&mut result_recv)
    );

    items_send.send(3).unwrap();
    items_send.send(4).unwrap();
    df.run_tick();

    assert_eq!((2, 0), (df.current_tick(), df.current_stratum()));
    assert_eq!(
        &[10],
        &*hydroflow::util::collect_ready::<Vec<_>, _>(&mut result_recv)
    );
}

#[multiplatform_test]
pub fn test_reduce_sum() {
    let (items_send, items_recv) = hydroflow::util::unbounded_channel::<usize>();

    let mut df = hydroflow_syntax! {
        source_stream(items_recv)
            -> reduce(|a, b| a + b)
            -> for_each(|v| print!("{:?}", v));
    };
    assert_graphvis_snapshots!(df);
    assert_eq!((0, 0), (df.current_tick(), df.current_stratum()));
    df.run_tick();
    assert_eq!((1, 0), (df.current_tick(), df.current_stratum()));

    print!("\nA: ");

    items_send.send(9).unwrap();
    items_send.send(2).unwrap();
    items_send.send(5).unwrap();
    df.run_tick();
    assert_eq!((2, 0), (df.current_tick(), df.current_stratum()));

    print!("\nB: ");

    items_send.send(9).unwrap();
    items_send.send(5).unwrap();
    items_send.send(2).unwrap();
    items_send.send(0).unwrap();
    items_send.send(3).unwrap();
    df.run_tick();
    assert_eq!((3, 0), (df.current_tick(), df.current_stratum()));

    println!();
}

/// This tests graph reachability along with an accumulation (in this case sum of vertex ids).
/// This is to test fixed-point being reched before the accumulation running.
#[multiplatform_test]
pub fn test_reduce() {
    // An edge in the input data = a pair of `usize` vertex IDs.
    let (pairs_send, pairs_recv) = hydroflow::util::unbounded_channel::<(usize, usize)>();

    let mut df = hydroflow_syntax! {
        reached_vertices = merge() -> map(|v| (v, ()));
        source_iter(vec![0]) -> [0]reached_vertices;

        my_join_tee = join() -> map(|(_src, ((), dst))| dst) -> tee();
        reached_vertices -> [0]my_join_tee;
        source_stream(pairs_recv) -> [1]my_join_tee;

        my_join_tee[0] -> [1]reached_vertices;
        my_join_tee[1] -> reduce(|a, b| a + b) -> for_each(|sum| println!("{}", sum));
    };
    assert_graphvis_snapshots!(df);
    assert_eq!((0, 0), (df.current_tick(), df.current_stratum()));
    df.run_tick();
    assert_eq!((1, 0), (df.current_tick(), df.current_stratum()));

    println!("A");

    pairs_send.send((0, 1)).unwrap();
    pairs_send.send((2, 4)).unwrap();
    pairs_send.send((3, 4)).unwrap();
    pairs_send.send((1, 2)).unwrap();
    df.run_tick();
    assert_eq!((2, 0), (df.current_tick(), df.current_stratum()));

    println!("B");

    pairs_send.send((0, 3)).unwrap();
    pairs_send.send((0, 3)).unwrap();
    df.run_tick();
    assert_eq!((3, 0), (df.current_tick(), df.current_stratum()));
}
