use dfir_rs::util::{collect_ready, iter_batches_stream};
use dfir_rs::{assert_graphvis_snapshots, dfir_syntax};
use multiplatform_test::multiplatform_test;

#[multiplatform_test]
pub fn test_flo_syntax() {
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<_>();

    let mut df = dfir_syntax! {
        users = source_iter(["alice", "bob"]);
        messages = source_stream(iter_batches_stream(0..12, 3));
        loop {
            users -> prefix() -> [0]cp;
            messages -> batch() -> [1]cp;
            cp = cross_join()
                -> map(|item| (context.current_tick().0, item))
                -> for_each(|x| result_send.send(x).unwrap());
        }
    };
    assert_graphvis_snapshots!(df);
    df.run_available();

    assert_eq!(
        &[
            (0, ("alice", 0)),
            (0, ("alice", 1)),
            (0, ("alice", 2)),
            (0, ("bob", 0)),
            (0, ("bob", 1)),
            (0, ("bob", 2)),
            (1, ("alice", 3)),
            (1, ("alice", 4)),
            (1, ("alice", 5)),
            (1, ("bob", 3)),
            (1, ("bob", 4)),
            (1, ("bob", 5)),
            (2, ("alice", 6)),
            (2, ("alice", 7)),
            (2, ("alice", 8)),
            (2, ("bob", 6)),
            (2, ("bob", 7)),
            (2, ("bob", 8)),
            (3, ("alice", 9)),
            (3, ("alice", 10)),
            (3, ("alice", 11)),
            (3, ("bob", 9)),
            (3, ("bob", 10)),
            (3, ("bob", 11)),
        ],
        &*collect_ready::<Vec<_>, _>(&mut result_recv)
    );
}

#[multiplatform_test]
pub fn test_flo_nested() {
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<_>();

    let mut df = dfir_syntax! {
        users = source_iter(["alice", "bob"]);
        messages = source_stream(iter_batches_stream(0..12, 3));
        loop {
            users -> prefix() -> [0]cp;
            messages -> batch() -> [1]cp;
            cp = cross_join();
            loop {
                cp
                    -> all_once()
                    -> map(|vec| (context.current_tick().0, vec))
                    -> for_each(|x| result_send.send(x).unwrap());
            }
        }
    };
    assert_graphvis_snapshots!(df);
    df.run_available();

    assert_eq!(
        &[
            (0, ("alice", 0)),
            (0, ("alice", 1)),
            (0, ("alice", 2)),
            (0, ("bob", 0)),
            (0, ("bob", 1)),
            (0, ("bob", 2)),
            (1, ("alice", 3)),
            (1, ("alice", 4)),
            (1, ("alice", 5)),
            (1, ("bob", 3)),
            (1, ("bob", 4)),
            (1, ("bob", 5)),
            (2, ("alice", 6)),
            (2, ("alice", 7)),
            (2, ("alice", 8)),
            (2, ("bob", 6)),
            (2, ("bob", 7)),
            (2, ("bob", 8)),
            (3, ("alice", 9)),
            (3, ("alice", 10)),
            (3, ("alice", 11)),
            (3, ("bob", 9)),
            (3, ("bob", 10)),
            (3, ("bob", 11)),
        ],
        &*collect_ready::<Vec<_>, _>(&mut result_recv)
    );
}

#[multiplatform_test(test, env_tracing)]
pub fn test_flo_repeat_n() {
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<_>();

    let mut df = dfir_syntax! {
        users = source_iter(["alice", "bob"]);
        messages = source_stream(iter_batches_stream(0..9, 3));
        loop {
            users -> prefix() -> [0]cp;
            messages -> batch() -> [1]cp;
            cp = cross_join();
            loop {
                cp -> repeat_n(2) -> for_each(|x| result_send.send(x).unwrap());
            }
        }
    };
    assert_graphvis_snapshots!(df);
    df.run_available();

    assert_eq!(
        &[
            ("alice", 0),
            ("alice", 1),
            ("alice", 2),
            ("bob", 0),
            ("bob", 1),
            ("bob", 2),
            ("alice", 0),
            ("alice", 1),
            ("alice", 2),
            ("bob", 0),
            ("bob", 1),
            ("bob", 2),
            ("alice", 3),
            ("alice", 4),
            ("alice", 5),
            ("bob", 3),
            ("bob", 4),
            ("bob", 5),
            ("alice", 3),
            ("alice", 4),
            ("alice", 5),
            ("bob", 3),
            ("bob", 4),
            ("bob", 5),
            ("alice", 6),
            ("alice", 7),
            ("alice", 8),
            ("bob", 6),
            ("bob", 7),
            ("bob", 8),
            ("alice", 6),
            ("alice", 7),
            ("alice", 8),
            ("bob", 6),
            ("bob", 7),
            ("bob", 8),
        ],
        &*collect_ready::<Vec<_>, _>(&mut result_recv)
    );
}

#[multiplatform_test(test, wasm, env_tracing)]
pub fn test_flo_repeat_n_nested() {
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<_>();

    let mut df = dfir_syntax! {
        usrs1 = source_iter(["alice", "bob"]);
        loop {
            usrs2 = usrs1 -> batch();
            loop {
                usrs3 = usrs2 -> repeat_n(3) -> inspect(|x| println!("A {:?} {}", x, context.is_first_loop_iteration()));
                loop {
                    usrs3 -> repeat_n(3)
                        -> inspect(|x| println!("B {:?} {}", x, context.is_first_loop_iteration()))
                        -> for_each(|x| result_send.send(x).unwrap());
                }
            }
        }
    };
    assert_graphvis_snapshots!(df);
    df.run_available();

    assert_eq!(
        &[
            "alice", "bob", "alice", "bob", "alice", "bob", "alice", "bob", "alice", "bob",
            "alice", "bob", "alice", "bob", "alice", "bob", "alice", "bob",
        ],
        &*collect_ready::<Vec<_>, _>(&mut result_recv)
    );
}

// #[multiplatform_test]
// pub fn test_flo_repeat_n_multiple_nested() {
// let mut df = dfir_syntax! {
// usrs1 = source_iter(["alice", "bob"]);
// loop {
// usrs2 = usrs1 -> batch() -> flatten();
// loop {
// usrs3 = usrs2 -> repeat_n(3) -> flatten()
// -> inspect(|x| println!("{:?} {}", x, context.is_first_loop_iteration()))
// -> tee();
// loop {
// usrs3 -> repeat_n(3)
// -> inspect(|x| println!("{} {:?} {}", line!(), x, context.is_first_loop_iteration()))
// -> assert_eq([
// vec!["alice", "bob"],
// vec!["alice", "bob"],
// vec!["alice", "bob"],
// vec!["alice", "bob"],
// vec!["alice", "bob"],
// vec!["alice", "bob"],
// vec!["alice", "bob"],
// vec!["alice", "bob"],
// vec!["alice", "bob"],
// ]);
// }
// loop {
// usrs3 -> repeat_n(3)
// -> inspect(|x| println!("{} {:?} {}", line!(), x, context.is_first_loop_iteration()))
// -> assert_eq([
// vec!["alice", "bob"],
// vec!["alice", "bob"],
// vec!["alice", "bob"],
// vec!["alice", "bob"],
// vec!["alice", "bob"],
// vec!["alice", "bob"],
// vec!["alice", "bob"],
// vec!["alice", "bob"],
// vec!["alice", "bob"],
// ]);
// }
// }
// }
// };
// assert_graphvis_snapshots!(df);
// df.run_available();
// }
