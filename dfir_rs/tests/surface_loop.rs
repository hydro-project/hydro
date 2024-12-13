use dfir_rs::util::iter_batches_stream;
use dfir_rs::{assert_graphvis_snapshots, dfir_syntax};
use multiplatform_test::multiplatform_test;

#[multiplatform_test]
pub fn test_flo_syntax() {
    let mut df = dfir_syntax! {
        users = source_iter(["alice", "bob"]);
        messages = source_stream(iter_batches_stream(0..12, 3));
        loop {
            // TODO(mingwei): cross_join type negotion should allow us to eliminate `flatten()`.
            users -> batch() -> flatten() -> [0]cp;
            messages -> batch() -> flatten() -> [1]cp;
            cp = cross_join::<'static, 'tick>()
                -> map(|item| (context.current_tick().0, item))
                -> assert_eq([
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
                ]);
        }
    };
    assert_graphvis_snapshots!(df);
    df.run_available();
}

#[multiplatform_test]
pub fn test_flo_nested() {
    let mut df = dfir_syntax! {
        users = source_iter(["alice", "bob"]);
        messages = source_stream(iter_batches_stream(0..12, 3));
        loop {
            // TODO(mingwei): cross_join type negotion should allow us to eliminate `flatten()`.
            users -> batch() -> flatten() -> [0]cp;
            messages -> batch() -> flatten() -> [1]cp;
            cp = cross_join::<'static, 'tick>();
            loop {
                cp
                    -> all_once()
                    -> map(|vec| (context.current_tick().0, vec))
                    -> assert_eq([
                        (0, vec![("alice", 0), ("alice", 1), ("alice", 2), ("bob", 0), ("bob", 1), ("bob", 2)]),
                        (1, vec![("alice", 3), ("alice", 4), ("alice", 5), ("bob", 3), ("bob", 4), ("bob", 5)]),
                        (2, vec![("alice", 6), ("alice", 7), ("alice", 8), ("bob", 6), ("bob", 7), ("bob", 8)]),
                        (3, vec![("alice", 9), ("alice", 10), ("alice", 11), ("bob", 9), ("bob", 10), ("bob", 11)]),
                    ]);
            }
        }
    };
    assert_graphvis_snapshots!(df);
    df.run_available();
}

#[multiplatform_test]
pub fn test_flo_repeat_n() {
    let mut df = dfir_syntax! {
        users = source_iter(["alice", "bob"]);
        messages = source_stream(iter_batches_stream(0..12, 3));
        loop {
            // TODO(mingwei): cross_join type negotion should allow us to eliminate `flatten()`.
            users -> batch() -> flatten() -> [0]cp;
            messages -> batch() -> flatten() -> [1]cp;
            cp = cross_join::<'static, 'tick>();
            loop {
                cp
                    -> repeat_n(3)
                    -> map(|vec| (context.current_tick().0, vec))
                    -> inspect(|x| println!("{:?}", x))
                    -> assert_eq([
                        (0, vec![("alice", 0), ("alice", 1), ("alice", 2), ("bob", 0), ("bob", 1), ("bob", 2)]),
                        (0, vec![("alice", 0), ("alice", 1), ("alice", 2), ("bob", 0), ("bob", 1), ("bob", 2)]),
                        (0, vec![("alice", 0), ("alice", 1), ("alice", 2), ("bob", 0), ("bob", 1), ("bob", 2)]),
                        (1, vec![("alice", 3), ("alice", 4), ("alice", 5), ("bob", 3), ("bob", 4), ("bob", 5)]),
                        (1, vec![("alice", 3), ("alice", 4), ("alice", 5), ("bob", 3), ("bob", 4), ("bob", 5)]),
                        (1, vec![("alice", 3), ("alice", 4), ("alice", 5), ("bob", 3), ("bob", 4), ("bob", 5)]),
                        (2, vec![("alice", 6), ("alice", 7), ("alice", 8), ("bob", 6), ("bob", 7), ("bob", 8)]),
                        (2, vec![("alice", 6), ("alice", 7), ("alice", 8), ("bob", 6), ("bob", 7), ("bob", 8)]),
                        (2, vec![("alice", 6), ("alice", 7), ("alice", 8), ("bob", 6), ("bob", 7), ("bob", 8)]),
                        (3, vec![("alice", 9), ("alice", 10), ("alice", 11), ("bob", 9), ("bob", 10), ("bob", 11)]),
                        (3, vec![("alice", 9), ("alice", 10), ("alice", 11), ("bob", 9), ("bob", 10), ("bob", 11)]),
                        (3, vec![("alice", 9), ("alice", 10), ("alice", 11), ("bob", 9), ("bob", 10), ("bob", 11)]),
                    ]);
            }
        }
    };
    assert_graphvis_snapshots!(df);
    df.run_available();
}

#[multiplatform_test]
pub fn test_flo_repeat_n_nested() {
    let mut df = dfir_syntax! {
        usrs1 = source_iter(["alice", "bob"]);
        loop {
            usrs2 = usrs1 -> batch() -> flatten();
            loop {
                usrs3 = usrs2 -> repeat_n(3) -> flatten();
                loop {
                    usrs3 -> repeat_n(3)
                        -> inspect(|x| println!("{:?}", x))
                        -> assert_eq([
                            vec!["alice", "bob", "alice", "bob", "alice", "bob"],
                            vec!["alice", "bob", "alice", "bob", "alice", "bob"],
                            vec!["alice", "bob", "alice", "bob", "alice", "bob"],
                        ]);
                }
            }
        }
    };
    assert_graphvis_snapshots!(df);
    df.run_available();
}
