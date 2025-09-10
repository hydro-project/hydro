use dfir_rs::scheduled::ticks::TickInstant;
use dfir_rs::util::collect_ready;
use dfir_rs::{assert_graphvis_snapshots, dfir_syntax};
use multiplatform_test::multiplatform_test;

#[multiplatform_test]
pub fn test_reduce_tick() {
    let (items_send, items_recv) = dfir_rs::util::unbounded_channel::<u32>();
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<u32>();

    let mut df = dfir_rs::dfir_syntax! {
        source_stream(items_recv)
            -> reduce::<'tick>(|acc: &mut u32, next: u32| *acc += next)
            -> for_each(|v| result_send.send(v).unwrap());
    };
    assert_graphvis_snapshots!(df);
    assert_eq!(
        (TickInstant::new(0), 0),
        (df.current_tick(), df.current_stratum())
    );

    items_send.send(1).unwrap();
    items_send.send(2).unwrap();
    df.run_tick_sync();

    assert_eq!(
        (TickInstant::new(1), 0),
        (df.current_tick(), df.current_stratum())
    );
    assert_eq!(&[3], &*collect_ready::<Vec<_>, _>(&mut result_recv));

    items_send.send(3).unwrap();
    items_send.send(4).unwrap();
    df.run_tick_sync();

    assert_eq!(
        (TickInstant::new(2), 0),
        (df.current_tick(), df.current_stratum())
    );
    assert_eq!(&[7], &*collect_ready::<Vec<_>, _>(&mut result_recv));
}

#[multiplatform_test]
pub fn test_reduce_static() {
    let (items_send, items_recv) = dfir_rs::util::unbounded_channel::<u32>();
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<u32>();

    let mut df = dfir_rs::dfir_syntax! {
        source_stream(items_recv)
            -> reduce::<'static>(|acc: &mut u32, next: u32| *acc += next)
            -> for_each(|v| result_send.send(v).unwrap());
    };
    assert_graphvis_snapshots!(df);
    assert_eq!(
        (TickInstant::new(0), 0),
        (df.current_tick(), df.current_stratum())
    );

    items_send.send(1).unwrap();
    items_send.send(2).unwrap();
    df.run_tick_sync();

    assert_eq!(
        (TickInstant::new(1), 0),
        (df.current_tick(), df.current_stratum())
    );
    assert_eq!(&[3], &*collect_ready::<Vec<_>, _>(&mut result_recv));

    items_send.send(3).unwrap();
    items_send.send(4).unwrap();
    df.run_tick_sync();

    assert_eq!(
        (TickInstant::new(2), 0),
        (df.current_tick(), df.current_stratum())
    );
    assert_eq!(&[10], &*collect_ready::<Vec<_>, _>(&mut result_recv));
}

#[multiplatform_test]
pub fn test_reduce_sum() {
    let (items_send, items_recv) = dfir_rs::util::unbounded_channel::<usize>();

    let mut df = dfir_syntax! {
        source_stream(items_recv)
            -> reduce(|a: &mut _, b| *a += b)
            -> for_each(|v| print!("{:?}", v));
    };
    assert_graphvis_snapshots!(df);
    assert_eq!(
        (TickInstant::new(0), 0),
        (df.current_tick(), df.current_stratum())
    );
    df.run_tick_sync();
    assert_eq!(
        (TickInstant::new(1), 0),
        (df.current_tick(), df.current_stratum())
    );

    print!("\nA: ");

    items_send.send(9).unwrap();
    items_send.send(2).unwrap();
    items_send.send(5).unwrap();
    df.run_tick_sync();
    assert_eq!(
        (TickInstant::new(2), 0),
        (df.current_tick(), df.current_stratum())
    );

    print!("\nB: ");

    items_send.send(9).unwrap();
    items_send.send(5).unwrap();
    items_send.send(2).unwrap();
    items_send.send(0).unwrap();
    items_send.send(3).unwrap();
    df.run_tick_sync();
    assert_eq!(
        (TickInstant::new(3), 0),
        (df.current_tick(), df.current_stratum())
    );

    println!();
}

/// This tests graph reachability along with an accumulation (in this case sum of vertex ids).
/// This is to test fixed-point being reched before the accumulation running.
#[multiplatform_test]
pub fn test_reduce() {
    // An edge in the input data = a pair of `usize` vertex IDs.
    let (pairs_send, pairs_recv) = dfir_rs::util::unbounded_channel::<(usize, usize)>();

    let mut df = dfir_syntax! {
        reached_vertices = union() -> map(|v| (v, ()));
        source_iter(vec![0]) -> [0]reached_vertices;

        my_join_tee = join() -> map(|(_src, ((), dst))| dst) -> tee();
        reached_vertices -> [0]my_join_tee;
        source_stream(pairs_recv) -> [1]my_join_tee;

        my_join_tee[0] -> [1]reached_vertices;
        my_join_tee[1] -> reduce(|a: &mut _, b| *a += b) -> for_each(|sum| println!("{}", sum));
    };
    assert_graphvis_snapshots!(df);
    assert_eq!(
        (TickInstant::new(0), 0),
        (df.current_tick(), df.current_stratum())
    );
    df.run_tick_sync();
    assert_eq!(
        (TickInstant::new(1), 0),
        (df.current_tick(), df.current_stratum())
    );

    println!("A");

    pairs_send.send((0, 1)).unwrap();
    pairs_send.send((2, 4)).unwrap();
    pairs_send.send((3, 4)).unwrap();
    pairs_send.send((1, 2)).unwrap();
    df.run_tick_sync();
    assert_eq!(
        (TickInstant::new(2), 0),
        (df.current_tick(), df.current_stratum())
    );

    println!("B");

    pairs_send.send((0, 3)).unwrap();
    pairs_send.send((0, 3)).unwrap();
    df.run_tick_sync();
    assert_eq!(
        (TickInstant::new(3), 0),
        (df.current_tick(), df.current_stratum())
    );
}

#[test]
fn test_reduce_loop_lifetime() {
    let (result1_send, mut result1_recv) = dfir_rs::util::unbounded_channel::<_>();
    let (result2_send, mut result2_recv) = dfir_rs::util::unbounded_channel::<_>();

    let mut df = dfir_syntax! {
        a = source_iter(0..10);
        loop {
            b = a -> batch() -> tee();
            loop {
                b -> repeat_n(5)
                    -> reduce::<'none>(|old: &mut _, val| {
                        *old += val;
                    })
                    -> for_each(|v| result1_send.send(v).unwrap());

                b -> repeat_n(5)
                    -> reduce::<'loop>(|old: &mut _, val| {
                        *old += val;
                    })
                    -> for_each(|v| result2_send.send(v).unwrap());
            };
        };
    };
    df.run_available_sync();

    // `'none` resets each iteration.
    assert_eq!(
        &[45, 45, 45, 45, 45],
        &*collect_ready::<Vec<_>, _>(&mut result1_recv)
    );
    // `'loop` accumulates across iterations.
    assert_eq!(
        &[45, 90, 135, 180, 225],
        &*collect_ready::<Vec<_>, _>(&mut result2_recv)
    );
}
