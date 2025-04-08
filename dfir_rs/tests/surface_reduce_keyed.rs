use std::collections::BTreeSet;

use dfir_rs::assert_graphvis_snapshots;
use dfir_rs::scheduled::ticks::TickInstant;
use dfir_rs::util::collect_ready;
use multiplatform_test::multiplatform_test;

#[multiplatform_test]
pub fn test_reduce_keyed_infer_basic() {
    pub struct SubordResponse {
        pub xid: &'static str,
        pub mtype: u32,
    }
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<(&'static str, u32)>();

    let mut df = dfir_rs::dfir_syntax! {
        source_iter([
            SubordResponse { xid: "123", mtype: 33 },
            SubordResponse { xid: "123", mtype: 52 },
            SubordResponse { xid: "123", mtype: 72 },
            SubordResponse { xid: "123", mtype: 83 },
            SubordResponse { xid: "123", mtype: 78 },
        ])
            -> map(|m: SubordResponse| (m.xid, m.mtype))
            -> reduce_keyed::<'static>(|old: &mut u32, val: u32| *old += val)
            -> for_each(|kv| result_send.send(kv).unwrap());
    };
    assert_graphvis_snapshots!(df);
    assert_eq!(
        (TickInstant::new(0), 0),
        (df.current_tick(), df.current_stratum())
    );
    df.run_tick();
    assert_eq!(
        (TickInstant::new(1), 0),
        (df.current_tick(), df.current_stratum())
    );

    df.run_available(); // Should return quickly and not hang

    assert_eq!(
        &[("123", 318), ("123", 318)],
        &*collect_ready::<Vec<_>, _>(&mut result_recv)
    );
}

#[multiplatform_test]
pub fn test_reduce_keyed_tick() {
    let (items_send, items_recv) = dfir_rs::util::unbounded_channel::<(u32, Vec<u32>)>();
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<(u32, Vec<u32>)>();

    let mut df = dfir_rs::dfir_syntax! {
        source_stream(items_recv)
            -> reduce_keyed::<'tick>(|old: &mut Vec<u32>, mut x: Vec<u32>| old.append(&mut x))
            -> for_each(|v| result_send.send(v).unwrap());
    };
    assert_graphvis_snapshots!(df);
    assert_eq!(
        (TickInstant::new(0), 0),
        (df.current_tick(), df.current_stratum())
    );
    df.run_tick();
    assert_eq!(
        (TickInstant::new(1), 0),
        (df.current_tick(), df.current_stratum())
    );

    items_send.send((0, vec![1, 2])).unwrap();
    items_send.send((0, vec![3, 4])).unwrap();
    items_send.send((1, vec![1])).unwrap();
    items_send.send((1, vec![1, 2])).unwrap();
    df.run_tick();

    assert_eq!(
        (TickInstant::new(2), 0),
        (df.current_tick(), df.current_stratum())
    );
    assert_eq!(
        [(0, vec![1, 2, 3, 4]), (1, vec![1, 1, 2])]
            .into_iter()
            .collect::<BTreeSet<_>>(),
        collect_ready::<BTreeSet<_>, _>(&mut result_recv)
    );

    items_send.send((0, vec![5, 6])).unwrap();
    items_send.send((0, vec![7, 8])).unwrap();
    items_send.send((1, vec![10])).unwrap();
    items_send.send((1, vec![11, 12])).unwrap();
    df.run_tick();

    assert_eq!(
        (TickInstant::new(3), 0),
        (df.current_tick(), df.current_stratum())
    );
    assert_eq!(
        [(0, vec![5, 6, 7, 8]), (1, vec![10, 11, 12])]
            .into_iter()
            .collect::<BTreeSet<_>>(),
        collect_ready::<BTreeSet<_>, _>(&mut result_recv)
    );

    df.run_available(); // Should return quickly and not hang
}

#[multiplatform_test]
pub fn test_reduce_keyed_static() {
    let (items_send, items_recv) = dfir_rs::util::unbounded_channel::<(u32, Vec<u32>)>();
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<(u32, Vec<u32>)>();

    let mut df = dfir_rs::dfir_syntax! {
        source_stream(items_recv)
            -> reduce_keyed::<'static>(|old: &mut Vec<u32>, mut x: Vec<u32>| old.append(&mut x))
            -> for_each(|v| result_send.send(v).unwrap());
    };
    assert_graphvis_snapshots!(df);
    assert_eq!(
        (TickInstant::new(0), 0),
        (df.current_tick(), df.current_stratum())
    );
    df.run_tick();
    assert_eq!(
        (TickInstant::new(1), 0),
        (df.current_tick(), df.current_stratum())
    );

    items_send.send((0, vec![1, 2])).unwrap();
    items_send.send((0, vec![3, 4])).unwrap();
    items_send.send((1, vec![1])).unwrap();
    items_send.send((1, vec![1, 2])).unwrap();
    df.run_tick();

    assert_eq!(
        (TickInstant::new(2), 0),
        (df.current_tick(), df.current_stratum())
    );
    assert_eq!(
        [(0, vec![1, 2, 3, 4]), (1, vec![1, 1, 2])]
            .into_iter()
            .collect::<BTreeSet<_>>(),
        collect_ready::<BTreeSet<_>, _>(&mut result_recv)
    );

    items_send.send((0, vec![5, 6])).unwrap();
    items_send.send((0, vec![7, 8])).unwrap();
    items_send.send((1, vec![10])).unwrap();
    items_send.send((1, vec![11, 12])).unwrap();
    df.run_tick();

    assert_eq!(
        (TickInstant::new(3), 0),
        (df.current_tick(), df.current_stratum())
    );
    assert_eq!(
        [
            (0, vec![1, 2, 3, 4, 5, 6, 7, 8]),
            (1, vec![1, 1, 2, 10, 11, 12])
        ]
        .into_iter()
        .collect::<BTreeSet<_>>(),
        collect_ready::<BTreeSet<_>, _>(&mut result_recv)
    );

    df.run_available(); // Should return quickly and not hang
}

#[multiplatform_test]
pub fn test_reduce_keyed_loop_lifetime() {
    let (result1_send, mut result1_recv) = dfir_rs::util::unbounded_channel::<_>();
    let (result2_send, mut result2_recv) = dfir_rs::util::unbounded_channel::<_>();

    let mut df = dfir_rs::dfir_syntax! {
        a = source_iter([
            ("foo", 0),
            ("foo", 1),
            ("foo", 2),
            ("foo", 3),
            ("foo", 4),
            ("bar", 0),
            ("bar", 1),
            ("bar", 2),
            ("bar", 3),
            ("foo", 5),
            ("foo", 6),
            ("foo", 7),
            ("foo", 8),
            ("foo", 9),
            ("bar", 4),
            ("bar", 5),
            ("bar", 6),
            ("bar", 7),
            ("bar", 8),
            ("bar", 9),
        ]);

        loop {
            b = a -> batch() -> tee();
            loop {
                b -> repeat_n(5)
                    -> reduce_keyed::<'none>(|old: &mut u32, val: u32| *old += val)
                    -> for_each(|v| result1_send.send(v).unwrap());

                b -> repeat_n(5)
                    -> reduce_keyed::<'loop>(|old: &mut u32, val: u32| *old += val)
                    -> for_each(|v| result2_send.send(v).unwrap());
            };
        };
    };
    df.run_available();

    // `'none` resets each iteration.
    assert_eq!(
        BTreeSet::from_iter([
            ("bar", 45),
            ("foo", 45),
            ("bar", 45),
            ("foo", 45),
            ("bar", 45),
            ("foo", 45),
            ("bar", 45),
            ("foo", 45),
            ("bar", 45),
            ("foo", 45),
        ]),
        collect_ready::<BTreeSet<_>, _>(&mut result1_recv)
    );
    // `'loop` accumulates across iterations.
    assert_eq!(
        BTreeSet::from_iter([
            ("bar", 45),
            ("foo", 45),
            ("bar", 90),
            ("foo", 90),
            ("bar", 135),
            ("foo", 135),
            ("bar", 180),
            ("foo", 180),
            ("bar", 225),
            ("foo", 225),
        ]),
        collect_ready::<BTreeSet<_>, _>(&mut result2_recv)
    );
}
