use dfir_lang::graph::WriteConfig;
use dfir_rs::util::{collect_ready, iter_batches_stream};
use dfir_rs::{assert_graphvis_snapshots, dfir_syntax};
use multiplatform_test::multiplatform_test;

#[multiplatform_test(test, env_tracing, wasm)]
pub fn test_batches_basic() {
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<_>();

    let mut df = dfir_syntax! {
        x = source_stream(iter_batches_stream(0..10, 1));
        loop {
            x -> batch()
                -> for_each(|x| result_send.send((context.loop_iter_count(), x)).unwrap());
        };
    };
    df.run_available();

    assert_eq!(
        &[
            (0, 0),
            (1, 1),
            (2, 2),
            (3, 3),
            (4, 4),
            (5, 5),
            (6, 6),
            (7, 7),
            (8, 8),
            (9, 9),
        ],
        &*collect_ready::<Vec<_>, _>(&mut result_recv)
    );
}

#[multiplatform_test(test, env_tracing, wasm)]
pub fn test_flo_syntax() {
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<_>();

    let mut df = dfir_syntax! {
        users = source_iter(["alice", "bob"]);
        messages = source_stream(iter_batches_stream(0..12, 3));
        loop {
            users -> prefix() -> [0]cp;
            messages -> batch() -> [1]cp;
            cp = cross_join()
                -> map(|item| (context.loop_iter_count(), item))
                -> for_each(|x| result_send.send(x).unwrap());
        };
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

#[multiplatform_test(test, wasm, env_tracing)]
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
                    -> map(|item| (context.current_tick().0, item))
                    -> for_each(|x| result_send.send(x).unwrap());
            };
        };
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
                cp -> repeat_n(2)
                    -> inspect(|x| println!("{:?} {}", x, context.loop_iter_count()))
                    -> for_each(|x| result_send.send(x).unwrap());
            };
        };
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
                usrs3 = usrs2 -> repeat_n(3) -> inspect(|x| println!("A {:?} {}", x, context.loop_iter_count()));
                loop {
                    usrs3 -> repeat_n(3)
                        -> inspect(|x| println!("B {:?} {}", x, context.loop_iter_count()))
                        -> for_each(|x| result_send.send(x).unwrap());
                };
            };
        };
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

#[multiplatform_test]
pub fn test_flo_repeat_n_multiple_nested() {
    let (result1_send, mut result1_recv) = dfir_rs::util::unbounded_channel::<_>();
    let (result2_send, mut result2_recv) = dfir_rs::util::unbounded_channel::<_>();

    let mut df = dfir_syntax! {
        usrs1 = source_iter(["alice", "bob"]);
        loop {
            usrs2 = usrs1 -> batch();
            loop {
                usrs3 = usrs2 -> repeat_n(3)
                    -> inspect(|x| println!("{:?} {}", x, context.loop_iter_count()))
                    -> tee();
                loop {
                    usrs3 -> repeat_n(3)
                    -> inspect(|x| println!("{} {:?} {}", line!(), x, context.loop_iter_count()))
                    -> for_each(|x| result1_send.send(x).unwrap());
                };
                loop {
                    usrs3 -> repeat_n(3)
                        -> inspect(|x| println!("{} {:?} {}", line!(), x, context.loop_iter_count()))
                        -> for_each(|x| result2_send.send(x).unwrap());
                };
            };
        };
    };
    assert_graphvis_snapshots!(df);
    df.run_available();

    assert_eq!(
        &[
            "alice", "bob", "alice", "bob", "alice", "bob", "alice", "bob", "alice", "bob",
            "alice", "bob", "alice", "bob", "alice", "bob", "alice", "bob",
        ],
        &*collect_ready::<Vec<_>, _>(&mut result1_recv)
    );

    assert_eq!(
        &[
            "alice", "bob", "alice", "bob", "alice", "bob", "alice", "bob", "alice", "bob",
            "alice", "bob", "alice", "bob", "alice", "bob", "alice", "bob",
        ],
        &*collect_ready::<Vec<_>, _>(&mut result2_recv)
    );
}

#[multiplatform_test(test, wasm, env_tracing)]
pub fn test_flo_repeat_kmeans() {
    const POINTS: &[[i32; 2]] = &[
        [-210, -104],
        [-226, -143],
        [-258, -119],
        [-331, -129],
        [-250, -69],
        [-202, -113],
        [-222, -133],
        [-232, -155],
        [-220, -107],
        [-159, -109],
        [-49, 57],
        [-156, 52],
        [-22, 125],
        [-140, 168],
        [-118, 89],
        [-93, 133],
        [-101, 80],
        [-145, 79],
        [187, 36],
        [208, -66],
        [142, 5],
        [232, 41],
        [91, -37],
        [132, 16],
        [248, -39],
        [158, 65],
        [108, -41],
        [171, -121],
        [147, 5],
        [192, 58],
    ];
    const CENTROIDS: &[[i32; 2]] = &[[-50, 0], [0, 0], [50, 0]];

    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<_>();

    let mut df = dfir_syntax! {
        init_points = source_iter(POINTS) -> map(std::clone::Clone::clone);
        init_centroids = source_iter(CENTROIDS) -> map(std::clone::Clone::clone);
        loop {
            batch_points = init_points -> batch();
            batch_centroids = init_centroids -> batch();

            loop {
                points = batch_points
                    -> repeat_n(10)
                    -> [0]cj;
                batch_centroids -> all_once() -> centroids;

                centroids = union() -> tee();
                centroids -> [1]cj;

                cj = cross_join_multiset()
                    -> map(|(point, centroid): ([i32; 2], [i32; 2])| {
                        let dist2 = (point[0] - centroid[0]).pow(2) + (point[1] - centroid[1]).pow(2);
                        (point, (dist2, centroid))
                    })
                    -> reduce_keyed(|(a_dist2, a_centroid), (b_dist2, b_centroid)| {
                        if b_dist2 < *a_dist2 {
                            *a_dist2 = b_dist2;
                            *a_centroid = b_centroid;
                        }
                    })
                    -> map(|(point, (_dist2, centroid))| {
                        (centroid, (point, 1))
                    })
                    -> reduce_keyed(|(p1, n1), (p2, n2): ([i32; 2], i32)| {
                        p1[0] += p2[0];
                        p1[1] += p2[1];
                        *n1 += n2;
                    })
                    -> map(|(_centroid, (p, n)): (_, ([i32; 2], i32))| {
                         [p[0] / n, p[1] / n]
                    })
                    -> next_iteration()
                    -> inspect(|x| println!("centroid: {:?}", x))
                    -> centroids;
            };

            centroids
                -> all_iterations()
                -> for_each(|x| result_send.send(x).unwrap());
        };
    };
    assert_graphvis_snapshots!(df);
    df.run_available();

    let mut result = collect_ready::<Vec<_>, _>(&mut result_recv);
    let n = result.len();
    let last = &mut result[n - 3..];
    last.sort_unstable();
    assert_eq!(&[[-231, -118], [-103, 97], [168, -6]], last);
}

#[test]
fn test_state_codegen() {
    let mut df = dfir_syntax! {
        a = source_iter((0..10).chain(5..15)) -> tee();
        pairs = a -> map(|n| (n / 3, n)) -> tee();
        loop {
            a -> batch() -> fold(|| 0, |old: &mut _, val| {
                *old += val;
            }) -> for_each(|v| println!("fold1 {:?}", v));

            a -> batch() -> fold::<'none>(|| 0, |old: &mut _, val| {
                *old += val;
            }) -> for_each(|v| println!("fold2 {:?}", v));

            a -> batch() -> reduce::<'none>(|old: &mut _, val| {
                *old += val;
            }) -> for_each(|v| println!("reduce {:?}", v));

            pairs -> batch() -> fold_keyed(|| 0, |old: &mut _, val| {
                *old += val;
            }) -> for_each(|v| println!("fold_keyed {:?}", v));

            a -> batch() -> unique() -> for_each(|v| println!("unique {:?}", v));

            j = join() -> for_each(|v| println!("join {:?}", v));
            pairs -> batch() -> [0]j;
            pairs -> batch() -> [1]j;

            aj = difference() -> for_each(|v| println!("difference {:?}", v));
            a -> batch() -> filter(|n| 0 == n % 2) -> [neg]aj;
            a -> batch() -> [pos]aj;
        };
    };
    df.run_available();
}

#[multiplatform_test]
pub fn test_enumerate_loop() {
    let (result1_send, mut result1_recv) = dfir_rs::util::unbounded_channel::<_>();
    let (result2_send, mut result2_recv) = dfir_rs::util::unbounded_channel::<_>();
    let mut df = dfir_syntax! {
        init = source_iter(0..5);
        loop {
            batch_init = init -> batch() -> tee();
            loop {
                batch_init -> repeat_n(3) -> enumerate::<'none>() -> for_each(|x| result1_send.send(x).unwrap());
                batch_init -> repeat_n(3) -> enumerate::<'loop>() -> for_each(|x| result2_send.send(x).unwrap());
            };
        };
    };
    assert_graphvis_snapshots!(df);
    df.run_available();

    assert_eq!(
        &[
            (0, 0),
            (1, 1),
            (2, 2),
            (3, 3),
            (4, 4),
            (0, 0),
            (1, 1),
            (2, 2),
            (3, 3),
            (4, 4),
            (0, 0),
            (1, 1),
            (2, 2),
            (3, 3),
            (4, 4)
        ],
        &*collect_ready::<Vec<_>, _>(&mut result1_recv)
    );

    assert_eq!(
        &[
            (0, 0),
            (1, 1),
            (2, 2),
            (3, 3),
            (4, 4),
            (5, 0),
            (6, 1),
            (7, 2),
            (8, 3),
            (9, 4),
            (10, 0),
            (11, 1),
            (12, 2),
            (13, 3),
            (14, 4)
        ],
        &*collect_ready::<Vec<_>, _>(&mut result2_recv)
    );
}

#[multiplatform_test(test, wasm, env_tracing)]
pub fn test_write() {
    let (result_send, _result_recv) = dfir_rs::util::unbounded_channel::<_>();

    let df = dfir_syntax! {
        users = source_iter(["alice", "bob"]);
        messages = source_stream(iter_batches_stream(0..12, 3));
        loop {
            users -> prefix() -> [0]cp;
            messages -> batch() -> [1]cp;
            cp = cross_join();
            loop {
                cp
                    -> all_once()
                    -> map(|item| (context.current_tick().0, item))
                    -> for_each(|x| result_send.send(x).unwrap());
            };
        };
    };
    assert_graphvis_snapshots!(df, &WriteConfig::default());
    assert_graphvis_snapshots!(
        df,
        &WriteConfig {
            no_loops: true,
            ..WriteConfig::default()
        }
    );
    assert_graphvis_snapshots!(
        df,
        &WriteConfig {
            no_subgraphs: true,
            ..WriteConfig::default()
        }
    );
    assert_graphvis_snapshots!(
        df,
        &WriteConfig {
            no_varnames: true,
            ..WriteConfig::default()
        }
    );
    assert_graphvis_snapshots!(
        df,
        &WriteConfig {
            no_loops: true,
            no_subgraphs: true,
            ..WriteConfig::default()
        }
    );
    assert_graphvis_snapshots!(
        df,
        &WriteConfig {
            no_loops: true,
            no_varnames: true,
            ..WriteConfig::default()
        }
    );
    assert_graphvis_snapshots!(
        df,
        &WriteConfig {
            no_subgraphs: true,
            no_varnames: true,
            ..WriteConfig::default()
        }
    );
    assert_graphvis_snapshots!(
        df,
        &WriteConfig {
            no_loops: true,
            no_subgraphs: true,
            no_varnames: true,
            ..WriteConfig::default()
        }
    );
}
