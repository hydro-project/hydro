use std::collections::BTreeSet;

use dfir_rs::assert_graphvis_snapshots;
use dfir_rs::scheduled::ticks::TickInstant;
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
        &*dfir_rs::util::collect_ready::<Vec<_>, _>(&mut result_recv)
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
        dfir_rs::util::collect_ready::<BTreeSet<_>, _>(&mut result_recv)
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
        dfir_rs::util::collect_ready::<BTreeSet<_>, _>(&mut result_recv)
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
        dfir_rs::util::collect_ready::<BTreeSet<_>, _>(&mut result_recv)
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
        dfir_rs::util::collect_ready::<BTreeSet<_>, _>(&mut result_recv)
    );

    df.run_available(); // Should return quickly and not hang
}
