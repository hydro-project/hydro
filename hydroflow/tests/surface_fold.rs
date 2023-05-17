use std::collections::{HashMap, HashSet};

use hydroflow::util::collect_ready;
use hydroflow::{assert_graphvis_snapshots, hydroflow_syntax};
use multiplatform_test::multiplatform_test;

#[multiplatform_test]
pub fn test_fold_tick() {
    let (items_send, items_recv) = hydroflow::util::unbounded_channel::<Vec<u32>>();
    let (result_send, mut result_recv) = hydroflow::util::unbounded_channel::<Vec<u32>>();

    let mut df = hydroflow::hydroflow_syntax! {
        source_stream(items_recv)
            -> fold::<'tick>(Vec::new(), |mut old: Vec<u32>, mut x: Vec<u32>| { old.append(&mut x); old })
            -> for_each(|v| result_send.send(v).unwrap());
    };
    assert_graphvis_snapshots!(df);

    assert_eq!((0, 0), (df.current_tick(), df.current_stratum()));

    items_send.send(vec![1, 2]).unwrap();
    items_send.send(vec![3, 4]).unwrap();
    df.run_tick();

    assert_eq!((1, 0), (df.current_tick(), df.current_stratum()));
    assert_eq!(
        &[vec![1, 2, 3, 4]],
        &*hydroflow::util::collect_ready::<Vec<_>, _>(&mut result_recv)
    );

    items_send.send(vec![5, 6]).unwrap();
    items_send.send(vec![7, 8]).unwrap();
    df.run_tick();

    assert_eq!((2, 0), (df.current_tick(), df.current_stratum()));
    assert_eq!(
        &[vec![5, 6, 7, 8]],
        &*hydroflow::util::collect_ready::<Vec<_>, _>(&mut result_recv)
    );

    df.run_available(); // Should return quickly and not hang
}

#[multiplatform_test]
pub fn test_fold_static() {
    let (items_send, items_recv) = hydroflow::util::unbounded_channel::<Vec<u32>>();
    let (result_send, mut result_recv) = hydroflow::util::unbounded_channel::<Vec<u32>>();

    let mut df = hydroflow::hydroflow_syntax! {
        source_stream(items_recv)
            -> fold::<'static>(Vec::new(), |mut old: Vec<u32>, mut x: Vec<u32>| { old.append(&mut x); old })
            -> for_each(|v| result_send.send(v).unwrap());
    };
    assert_graphvis_snapshots!(df);

    assert_eq!((0, 0), (df.current_tick(), df.current_stratum()));

    items_send.send(vec![1, 2]).unwrap();
    items_send.send(vec![3, 4]).unwrap();
    df.run_tick();

    assert_eq!((1, 0), (df.current_tick(), df.current_stratum()));
    assert_eq!(
        &[vec![1, 2, 3, 4]],
        &*hydroflow::util::collect_ready::<Vec<_>, _>(&mut result_recv)
    );

    items_send.send(vec![5, 6]).unwrap();
    items_send.send(vec![7, 8]).unwrap();
    df.run_tick();

    assert_eq!((2, 0), (df.current_tick(), df.current_stratum()));
    assert_eq!(
        &[vec![1, 2, 3, 4, 5, 6, 7, 8]],
        &*hydroflow::util::collect_ready::<Vec<_>, _>(&mut result_recv)
    );

    df.run_available(); // Should return quickly and not hang
}

#[multiplatform_test]
pub fn test_fold_flatten() {
    // test pull
    let (out_send, mut out_recv) = hydroflow::util::unbounded_channel::<(u8, u8)>();
    let mut df_pull = hydroflow_syntax! {
        source_iter([(1,1), (1,2), (2,3), (2,4)])
            -> fold::<'tick>(HashMap::<u8,u8>::new(), |mut ht, t: (u8,u8)| {
                    let e = ht.entry(t.0).or_insert(0);
                    *e += t.1;
                    ht})
            -> flatten()
            -> for_each(|(k,v)| out_send.send((k,v)).unwrap());
    };

    assert_eq!((0, 0), (df_pull.current_tick(), df_pull.current_stratum()));
    df_pull.run_tick();
    assert_eq!((1, 0), (df_pull.current_tick(), df_pull.current_stratum()));

    let out: HashSet<_> = collect_ready(&mut out_recv);
    for pair in [(1, 3), (2, 7)] {
        assert!(out.contains(&pair));
    }

    // test push
    let (out_send, mut out_recv) = hydroflow::util::unbounded_channel::<(u8, u8)>();
    let mut df_push = hydroflow_syntax! {
        datagen = source_iter([(1,2), (1,2), (2,4), (2,4)]) -> tee();
        datagen[0] -> fold::<'tick>(HashMap::<u8,u8>::new(), |mut ht, t:(u8,u8)| {
                let e = ht.entry(t.0).or_insert(0);
                *e += t.1;
                ht})
            -> flatten()
            -> for_each(|(k,v)| out_send.send((k,v)).unwrap());
        datagen[1] -> null();
    };
    assert_eq!((0, 0), (df_push.current_tick(), df_push.current_stratum()));
    df_push.run_tick();
    assert_eq!((1, 0), (df_push.current_tick(), df_push.current_stratum()));

    let out: HashSet<_> = collect_ready(&mut out_recv);
    for pair in [(1, 4), (2, 8)] {
        assert!(out.contains(&pair));
    }

    df_push.run_available(); // Should return quickly and not hang
    df_pull.run_available(); // Should return quickly and not hang
}

#[multiplatform_test]
pub fn test_fold_sort() {
    let (items_send, items_recv) = hydroflow::util::unbounded_channel::<usize>();

    let mut df = hydroflow_syntax! {
        source_stream(items_recv)
            -> fold::<'tick>(Vec::new(), |mut v, x| {
                v.push(x);
                v
            })
            -> flat_map(|mut vec| { vec.sort(); vec })
            -> for_each(|v| print!("{:?}, ", v));
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

    df.run_available(); // Should return quickly and not hang
}
