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

#[multiplatform_test(test, env_tracing)]
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

    let mut df = dfir_syntax! {
        init_points = source_iter(POINTS) -> map(std::clone::Clone::clone);
        init_centroids = source_iter(CENTROIDS) -> map(std::clone::Clone::clone);
        loop {
            batch_points = init_points -> batch() -> flatten();
            batch_centroids = init_centroids -> batch() -> flatten();

            loop {
                points = batch_points
                    -> repeat_n(10)
                    -> flatten()
                    -> [0]cj;
                batch_centroids -> all_once() -> flatten() -> centroids;

                centroids = union() -> [1]cj;

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
                    -> map(|(_centroid, (p, n))| {
                         [p[0] / n, p[1] / n]
                    })
                    -> next_loop()
                    -> inspect(|x| println!("centroid: {:?}", x))
                    -> centroids;
            }
        }
    };
    assert_graphvis_snapshots!(df);
    df.run_available();
}
