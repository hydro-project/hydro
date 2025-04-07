use std::collections::{HashMap, HashSet};

use dfir_rs::scheduled::ticks::TickInstant;
use dfir_rs::util::collect_ready;
use dfir_rs::{assert_graphvis_snapshots, dfir_syntax};
use multiplatform_test::multiplatform_test;

#[multiplatform_test]
pub fn test_fold_tick() {
    let (items_send, items_recv) = dfir_rs::util::unbounded_channel::<Vec<u32>>();
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<Vec<u32>>();

    let mut df = dfir_rs::dfir_syntax! {
        source_stream(items_recv)
            -> fold::<'tick>(Vec::new, |old: &mut Vec<u32>, mut x: Vec<u32>| { old.append(&mut x); })
            -> for_each(|v| result_send.send(v).unwrap());
    };
    assert_graphvis_snapshots!(df);

    assert_eq!(
        (TickInstant::new(0), 0),
        (df.current_tick(), df.current_stratum())
    );

    items_send.send(vec![1, 2]).unwrap();
    items_send.send(vec![3, 4]).unwrap();
    df.run_tick();

    assert_eq!(
        (TickInstant::new(1), 0),
        (df.current_tick(), df.current_stratum())
    );
    assert_eq!(
        &[vec![1, 2, 3, 4]],
        &*collect_ready::<Vec<_>, _>(&mut result_recv)
    );

    items_send.send(vec![5, 6]).unwrap();
    items_send.send(vec![7, 8]).unwrap();
    df.run_tick();

    assert_eq!(
        (TickInstant::new(2), 0),
        (df.current_tick(), df.current_stratum())
    );
    assert_eq!(
        &[vec![5, 6, 7, 8]],
        &*collect_ready::<Vec<_>, _>(&mut result_recv)
    );

    df.run_available(); // Should return quickly and not hang
}

#[multiplatform_test]
pub fn test_fold_static() {
    let (items_send, items_recv) = dfir_rs::util::unbounded_channel::<Vec<u32>>();
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<Vec<u32>>();

    let mut df = dfir_rs::dfir_syntax! {
        source_stream(items_recv)
            -> fold::<'static>(Vec::new, |old: &mut Vec<u32>, mut x: Vec<u32>| { old.append(&mut x); })
            -> for_each(|v| result_send.send(v).unwrap());
    };
    assert_graphvis_snapshots!(df);

    assert_eq!(
        (TickInstant::new(0), 0),
        (df.current_tick(), df.current_stratum())
    );

    items_send.send(vec![1, 2]).unwrap();
    items_send.send(vec![3, 4]).unwrap();
    df.run_tick();

    assert_eq!(
        (TickInstant::new(1), 0),
        (df.current_tick(), df.current_stratum())
    );
    assert_eq!(
        &[vec![1, 2, 3, 4]],
        &*collect_ready::<Vec<_>, _>(&mut result_recv)
    );

    items_send.send(vec![5, 6]).unwrap();
    items_send.send(vec![7, 8]).unwrap();
    df.run_tick();

    assert_eq!(
        (TickInstant::new(2), 0),
        (df.current_tick(), df.current_stratum())
    );
    assert_eq!(
        &[vec![1, 2, 3, 4, 5, 6, 7, 8]],
        &*collect_ready::<Vec<_>, _>(&mut result_recv)
    );

    df.run_available(); // Should return quickly and not hang
}

#[multiplatform_test]
pub fn test_fold_static_join() {
    let (items_send, items_recv) = dfir_rs::util::unbounded_channel::<usize>();
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<(usize, usize)>();

    let mut df = dfir_rs::dfir_syntax! {
        teed_fold = source_iter([])
            -> fold::<'tick>(|| 0, |old: &mut usize, _: usize| { *old += 1; })
            -> tee();
        teed_fold -> for_each(|_| {});
        teed_fold -> [1]join_node;

        source_stream(items_recv) -> [0]join_node;

        join_node = cross_join_multiset();
        join_node -> for_each(|v| result_send.send(v).unwrap());
    };
    assert_graphvis_snapshots!(df);

    assert_eq!(
        (TickInstant::new(0), 0),
        (df.current_tick(), df.current_stratum())
    );

    items_send.send(0).unwrap();
    df.run_available();

    assert_eq!(
        (TickInstant::new(1), 0),
        (df.current_tick(), df.current_stratum())
    );
    assert_eq!(&[(0, 0)], &*collect_ready::<Vec<_>, _>(&mut result_recv));

    items_send.send(1).unwrap();
    df.run_available();

    assert_eq!(
        (TickInstant::new(2), 0),
        (df.current_tick(), df.current_stratum())
    );
    assert_eq!(&[(1, 0)], &*collect_ready::<Vec<_>, _>(&mut result_recv));
}

#[multiplatform_test]
pub fn test_fold_flatten() {
    // test pull
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<(u8, u8)>();
    let mut df_pull = dfir_syntax! {
        source_iter([(1,1), (1,2), (2,3), (2,4)])
            -> fold::<'tick>(HashMap::<u8, u8>::new, |ht: &mut HashMap<u8, u8>, t: (u8,u8)| {
                    let e = ht.entry(t.0).or_insert(0);
                    *e += t.1;
                })
            -> flatten()
            -> for_each(|(k,v)| out_send.send((k,v)).unwrap());
    };

    assert_eq!(
        (TickInstant::new(0), 0),
        (df_pull.current_tick(), df_pull.current_stratum())
    );
    df_pull.run_tick();
    assert_eq!(
        (TickInstant::new(1), 0),
        (df_pull.current_tick(), df_pull.current_stratum())
    );

    let out: HashSet<_> = collect_ready(&mut out_recv);
    for pair in [(1, 3), (2, 7)] {
        assert!(out.contains(&pair));
    }

    // test push
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<(u8, u8)>();
    let mut df_push = dfir_syntax! {
        datagen = source_iter([(1,2), (1,2), (2,4), (2,4)]) -> tee();
        datagen[0] -> fold::<'tick>(HashMap::<u8, u8>::new, |ht: &mut HashMap<u8, u8>, t:(u8,u8)| {
                let e = ht.entry(t.0).or_insert(0);
                *e += t.1;
            })
            -> flatten()
            -> for_each(|(k,v)| out_send.send((k,v)).unwrap());
        datagen[1] -> null();
    };
    assert_eq!(
        (TickInstant::new(0), 0),
        (df_push.current_tick(), df_push.current_stratum())
    );
    df_push.run_tick();
    assert_eq!(
        (TickInstant::new(1), 0),
        (df_push.current_tick(), df_push.current_stratum())
    );

    let out: HashSet<_> = collect_ready(&mut out_recv);
    for pair in [(1, 4), (2, 8)] {
        assert!(out.contains(&pair));
    }

    df_push.run_available(); // Should return quickly and not hang
    df_pull.run_available(); // Should return quickly and not hang
}

#[multiplatform_test]
pub fn test_fold_sort() {
    let (items_send, items_recv) = dfir_rs::util::unbounded_channel::<usize>();

    let mut df = dfir_syntax! {
        source_stream(items_recv)
            -> fold::<'tick>(Vec::new, Vec::push)
            -> flat_map(|mut vec| { vec.sort(); vec })
            -> for_each(|v| print!("{:?}, ", v));
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

    print!("\nA: ");

    items_send.send(9).unwrap();
    items_send.send(2).unwrap();
    items_send.send(5).unwrap();
    df.run_tick();
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
    df.run_tick();
    assert_eq!(
        (TickInstant::new(3), 0),
        (df.current_tick(), df.current_stratum())
    );

    println!();

    df.run_available(); // Should return quickly and not hang
}

#[multiplatform_test]
pub fn test_fold_inference() {
    let (_items_send, items_recv) = dfir_rs::util::unbounded_channel::<String>();

    let _ = dfir_rs::dfir_syntax! {
        source_stream(items_recv)
            -> fold::<'tick>(|| 0, |old, s| { *old += s.len() })
            -> for_each(|_| {});
    };
}
