use std::collections::HashSet;

use dfir_rs::scheduled::graph::Dfir;
use dfir_rs::scheduled::ticks::TickInstant;
use dfir_rs::util::collect_ready;
use dfir_rs::util::multiset::HashMultiSet;
use dfir_rs::{assert_graphvis_snapshots, dfir_syntax};
use multiplatform_test::multiplatform_test;

// TODO(mingwei): custom operators? How to handle in syntax? How to handle state?

// TODO(mingwei): Still need to handle crossing stratum boundaries
// TODO(mingwei): Implement non-monotonicity handling.

// TODO(mingwei): Tiemo user test after Tuesday.

// TODO(mingwei): Try to get more bad error messages to appear.

// TODO(joe): QOL: make a way to generate/print the mermaid graph.

// TODO(mingwei): Prevent unused variable warnings when dfir code is not generated.

// Joe:
// TODO(mingwei): Documentation articles.
// TODO(mingwei): Find a way to display join keys

#[multiplatform_test]
pub fn test_basic_2() {
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<usize>();

    let mut df = dfir_syntax! {
        source_iter([1]) -> for_each(|v| out_send.send(v).unwrap());
    };
    assert_graphvis_snapshots!(df);
    df.run_available_sync();

    assert_eq!(&[1], &*collect_ready::<Vec<_>, _>(&mut out_recv));
}

#[multiplatform_test]
pub fn test_basic_3() {
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<usize>();

    let mut df = dfir_syntax! {
        source_iter([1]) -> map(|v| v + 1) -> for_each(|v| out_send.send(v).unwrap());
    };
    assert_graphvis_snapshots!(df);
    df.run_available_sync();

    assert_eq!(&[2], &*collect_ready::<Vec<_>, _>(&mut out_recv));
}

#[multiplatform_test]
pub fn test_basic_union() {
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<usize>();

    let mut df = dfir_syntax! {
        m = union() -> for_each(|v| out_send.send(v).unwrap());
        source_iter([1]) -> [0]m;
        source_iter([2]) -> [1]m;
    };
    assert_graphvis_snapshots!(df);
    df.run_available_sync();

    assert_eq!(&[1, 2], &*collect_ready::<Vec<_>, _>(&mut out_recv));
}

#[multiplatform_test]
pub fn test_basic_tee() {
    let (out_send_a, mut out_recv) = dfir_rs::util::unbounded_channel::<String>();
    let out_send_b = out_send_a.clone();

    let mut df = dfir_syntax! {
        t = source_iter([1]) -> tee();
        t[0] -> for_each(|v| out_send_a.send(format!("A {}", v)).unwrap());
        t[1] -> for_each(|v| out_send_b.send(format!("B {}", v)).unwrap());
    };
    df.run_available_sync();

    let out: HashSet<_> = collect_ready(&mut out_recv);
    assert_eq!(2, out.len());
    assert!(out.contains("A 1"));
    assert!(out.contains("B 1"));
}

#[multiplatform_test]
pub fn test_basic_inspect_null() {
    use std::cell::RefCell;
    use std::rc::Rc;

    let seen = Rc::new(RefCell::new(Vec::new()));
    let seen_inner = Rc::clone(&seen);

    let mut df = dfir_syntax! {
        source_iter([1, 2, 3, 4]) -> inspect(|&x| seen_inner.borrow_mut().push(x)) -> null();
    };
    df.run_available_sync();

    assert_eq!(&[1, 2, 3, 4], &**seen.borrow());
}

#[multiplatform_test]
pub fn test_basic_inspect_no_null() {
    use std::cell::RefCell;
    use std::rc::Rc;

    let seen = Rc::new(RefCell::new(Vec::new()));
    let seen_inner = Rc::clone(&seen);

    let mut df = dfir_syntax! {
        source_iter([1, 2, 3, 4]) -> inspect(|&x| seen_inner.borrow_mut().push(x));
    };
    df.run_available_sync();

    assert_eq!(&[1, 2, 3, 4], &**seen.borrow());
}

// Mainly checking subgraph partitioning pull-push handling.
#[multiplatform_test]
pub fn test_large_diamond() {
    #[expect(clippy::map_identity, reason = "testing topology")]
    let mut df: Dfir = dfir_syntax! {
        t = source_iter([1]) -> tee();
        j = union() -> for_each(|x| println!("{}", x));
        t[0] -> map(std::convert::identity) -> map(std::convert::identity) -> [0]j;
        t[1] -> map(std::convert::identity) -> map(std::convert::identity) -> [1]j;
    };
    df.run_available_sync();
}

/// Test that source_stream can handle "complex" expressions.
#[multiplatform_test]
pub fn test_recv_expr() {
    let send_recv = dfir_rs::util::unbounded_channel::<usize>();

    let mut df = dfir_syntax! {
        source_stream(send_recv.1)
            -> for_each(|v| print!("{:?}", v));
    };
    assert_graphvis_snapshots!(df);
    df.run_available_sync();

    let items_send = send_recv.0;
    items_send.send(9).unwrap();
    items_send.send(2).unwrap();
    items_send.send(5).unwrap();
    df.run_available_sync();
}

#[multiplatform_test]
pub fn test_join_order() {
    let _df_good = dfir_syntax! {
        yikes = join() -> for_each(|m: ((), (u32, String))| println!("{:?}", m));
        source_iter([0,1,2]) -> map(|i| ((), i)) -> [0]yikes;
        source_iter(["a".to_string(),"b".to_string(),"c".to_string()]) -> map(|s| ((), s)) -> [1]yikes;
    };
    let _df_bad = dfir_syntax! {
        yikes = join() -> for_each(|m: ((), (u32, String))| println!("{:?}", m));
        source_iter(["a".to_string(),"b".to_string(),"c".to_string()]) -> map(|s| ((), s)) -> [1]yikes;
        source_iter([0,1,2]) -> map(|i| ((), i)) -> [0]yikes;
    };
}

#[multiplatform_test]
pub fn test_multiset_join() {
    // HalfJoinStateSetUnion
    {
        use dfir_rs::compiled::pull::HalfSetJoinState;

        let (out_tx, mut out_rx) = dfir_rs::util::unbounded_channel::<(usize, (usize, usize))>();

        let mut df = dfir_syntax! {
            my_join = join::<HalfSetJoinState>() -> for_each(|m| out_tx.send(m).unwrap());
            source_iter([(0, 1), (0, 1)]) -> [0]my_join;
            source_iter([(0, 2)]) -> [1]my_join;
        };

        df.run_available_sync();

        let out: Vec<_> = collect_ready(&mut out_rx);
        assert_eq!(out, vec![(0, (1, 2))]);
    }

    // HalfMultisetJoinState lhs biased
    {
        use dfir_rs::compiled::pull::HalfMultisetJoinState;
        let (out_tx, mut out_rx) = dfir_rs::util::unbounded_channel::<(usize, (usize, usize))>();

        let mut df = dfir_syntax! {
            my_join = join::<HalfMultisetJoinState>() -> for_each(|m| out_tx.send(m).unwrap());
            source_iter([(1, 1), (1, 1), (1, 1)]) -> [0]my_join;
            source_iter([(1, 2), (1, 2), (1, 2), (1, 2)]) -> [1]my_join;
        };

        df.run_available_sync();

        let out: Vec<_> = collect_ready(&mut out_rx);
        assert_eq!(out, [(1, (1, 2)); 12].to_vec());
    }

    // HalfMultisetJoinState rhs biased
    {
        use dfir_rs::compiled::pull::HalfMultisetJoinState;
        let (out_tx, mut out_rx) = dfir_rs::util::unbounded_channel::<(usize, (usize, usize))>();

        let mut df = dfir_syntax! {
            my_join = join::<HalfMultisetJoinState>() -> for_each(|m| out_tx.send(m).unwrap());
            source_iter([(1, 1), (1, 1), (1, 1), (1, 1)]) -> [0]my_join;
            source_iter([(1, 2), (1, 2), (1, 2)]) -> [1]my_join;
        };

        df.run_available_sync();

        let out: Vec<_> = collect_ready(&mut out_rx);
        assert_eq!(out, [(1, (1, 2)); 12].to_vec());
    }
}

#[multiplatform_test]
pub fn test_cross_join() {
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<(usize, &str)>();

    let mut df = dfir_syntax! {
        cj = cross_join() -> for_each(|v| out_send.send(v).unwrap());
        source_iter([1, 2, 2, 3]) -> [0]cj;
        source_iter(["a", "b", "c", "c"]) -> [1]cj;
    };
    df.run_available_sync();

    let mut out: Vec<_> = collect_ready(&mut out_recv);
    out.sort();
    assert_eq!(
        out,
        [
            (1, "a"),
            (1, "b"),
            (1, "c"),
            (2, "a"),
            (2, "b"),
            (2, "c"),
            (3, "a"),
            (3, "b"),
            (3, "c")
        ]
    );
}

#[multiplatform_test]
pub fn test_cross_join_multiset() {
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<(usize, &str)>();

    let mut df = dfir_syntax! {
        cj = cross_join_multiset() -> for_each(|v| out_send.send(v).unwrap());
        source_iter([1, 2, 2, 3]) -> [0]cj;
        source_iter(["a", "b", "c", "c"]) -> [1]cj;
    };
    df.run_available_sync();

    let mut out: Vec<_> = collect_ready(&mut out_recv);
    out.sort();
    assert_eq!(
        out,
        [
            (1, "a"),
            (1, "b"),
            (1, "c"),
            (1, "c"),
            (2, "a"),
            (2, "a"),
            (2, "b"),
            (2, "b"),
            (2, "c"),
            (2, "c"),
            (2, "c"),
            (2, "c"),
            (3, "a"),
            (3, "b"),
            (3, "c"),
            (3, "c"),
        ]
    );
}

#[multiplatform_test]
pub fn test_defer_tick() {
    let (inp_send, inp_recv) = dfir_rs::util::unbounded_channel::<usize>();
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<usize>();
    let mut flow = dfir_rs::dfir_syntax! {
        inp = source_stream(inp_recv) -> tee();
        diff = difference() -> for_each(|x| out_send.send(x).unwrap());
        inp -> [pos]diff;
        inp -> defer_tick() -> [neg]diff;
    };

    for x in [1, 2, 3, 4] {
        inp_send.send(x).unwrap();
    }
    flow.run_tick_sync();

    for x in [3, 4, 5, 6] {
        inp_send.send(x).unwrap();
    }
    flow.run_tick_sync();

    flow.run_available_sync();
    assert_eq!(
        HashMultiSet::from_iter([1, 2, 3, 4, 5, 6]),
        collect_ready(&mut out_recv)
    );
}

#[multiplatform_test]
pub fn test_channel_minimal() {
    let (send, recv) = dfir_rs::util::unbounded_channel::<usize>();

    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<usize>();

    let mut df1 = dfir_syntax! {
        source_iter([1, 2, 3]) -> for_each(|x| { send.send(x).unwrap(); });
    };

    let mut df2 = dfir_syntax! {
        source_stream(recv) -> for_each(|x| out_send.send(x).unwrap());
    };

    df2.run_available_sync();
    let results = collect_ready::<Vec<_>, _>(&mut out_recv);
    assert_eq!([] as [usize; 0], *results);

    df1.run_available_sync();
    let results = collect_ready::<Vec<_>, _>(&mut out_recv);
    assert_eq!([] as [usize; 0], *results);

    df2.run_available_sync();
    let results = collect_ready::<Vec<_>, _>(&mut out_recv);
    assert_eq!([1, 2, 3], *results);
}

#[multiplatform_test]
pub fn test_surface_syntax_reachability_generated() {
    // An edge in the input data = a pair of `usize` vertex IDs.
    let (pairs_send, pairs_recv) = dfir_rs::util::unbounded_channel::<(usize, usize)>();

    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<usize>();

    let mut df: Dfir = dfir_syntax! {
        reached_vertices = union() -> map(|v| (v, ()));
        source_iter(vec![0]) -> [0]reached_vertices;

        my_join_tee = join::<'static>()
            -> map(|(_src, ((), dst))| dst)
            -> unique::<'static>()
            -> tee();
        reached_vertices -> [0]my_join_tee;
        source_stream(pairs_recv) -> [1]my_join_tee;

        my_join_tee[0] -> [1]reached_vertices;
        my_join_tee[1] -> for_each(|x| out_send.send(x).unwrap());
    };
    assert_graphvis_snapshots!(df);

    df.run_available_sync();
    let results = collect_ready::<Vec<_>, _>(&mut out_recv);
    assert_eq!([] as [usize; 0], *results);

    pairs_send.send((0, 1)).unwrap();
    df.run_available_sync();
    let results = collect_ready::<Vec<_>, _>(&mut out_recv);
    assert_eq!([1], *results);

    pairs_send.send((2, 4)).unwrap();
    pairs_send.send((3, 4)).unwrap();
    df.run_available_sync();
    let results = collect_ready::<Vec<_>, _>(&mut out_recv);
    assert_eq!([] as [usize; 0], *results);

    pairs_send.send((1, 2)).unwrap();
    df.run_available_sync();
    let results = collect_ready::<Vec<_>, _>(&mut out_recv);
    assert_eq!([2, 4], *results);

    pairs_send.send((0, 3)).unwrap();
    df.run_available_sync();
    let results = collect_ready::<Vec<_>, _>(&mut out_recv);
    assert_eq!([3], *results);

    pairs_send.send((0, 3)).unwrap();
    df.run_available_sync();
    let results = collect_ready::<Vec<_>, _>(&mut out_recv);
    assert_eq!([] as [usize; 0], *results);
}

#[multiplatform_test]
pub fn test_transitive_closure() {
    // An edge in the input data = a pair of `usize` vertex IDs.
    let (pairs_send, pairs_recv) = dfir_rs::util::unbounded_channel::<(usize, usize)>();

    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<(usize, usize)>();

    let mut df = dfir_syntax! {
        // edge(x,y) :- link(x,y)
        edge_union_tee = union() -> tee();
        link_tee = tee();
        source_stream(pairs_recv) -> link_tee;
        link_tee[0] -> [0]edge_union_tee;

        // edge(a,b) :- edge(a,k), link(k,b)
        the_join = join::<'static>() -> unique::<'static>();
        edge_union_tee[0] -> map(|(a, k)| (k, a)) -> [0]the_join;
        link_tee[1] -> [1]the_join;
        the_join -> map(|(_k, (a, b))| (a, b)) -> [1]edge_union_tee;
        edge_union_tee[1] -> for_each(|(a, b)| out_send.send((a, b)).unwrap());
    };
    assert_graphvis_snapshots!(df);

    df.run_available_sync();
    let results = collect_ready::<Vec<_>, _>(&mut out_recv);
    assert_eq!([] as [(usize, usize); 0], *results);

    pairs_send.send((0, 1)).unwrap();
    df.run_available_sync();
    let results = collect_ready::<Vec<_>, _>(&mut out_recv);
    assert_eq!([(0, 1)], *results);

    pairs_send.send((2, 4)).unwrap();
    pairs_send.send((3, 4)).unwrap();
    df.run_available_sync();
    let results = collect_ready::<Vec<_>, _>(&mut out_recv);
    assert_eq!([(2, 4), (3, 4)], *results);

    pairs_send.send((1, 2)).unwrap();
    df.run_available_sync();
    let results = collect_ready::<Vec<_>, _>(&mut out_recv);
    assert_eq!([(1, 2), (0, 2), (1, 4), (0, 4)], *results);

    pairs_send.send((0, 3)).unwrap();
    df.run_available_sync();
    let results = collect_ready::<Vec<_>, _>(&mut out_recv);
    assert_eq!([(0, 3), (0, 4)], *results);

    pairs_send.send((0, 3)).unwrap();
    df.run_available_sync();
    let results = collect_ready::<Vec<_>, _>(&mut out_recv);
    assert_eq!([(0, 3)], *results);
}

#[multiplatform_test]
pub fn test_covid_tracing() {
    use dfir_rs::util::unbounded_channel;

    const TRANSMISSIBLE_DURATION: usize = 14; // Days.

    type Pid = usize;
    type Name = &'static str;
    type Phone = &'static str;
    type DateTime = usize; // Days.

    let (contacts_send, contacts_recv) = unbounded_channel::<(Pid, Pid, DateTime)>();
    let (diagnosed_send, diagnosed_recv) = unbounded_channel::<(Pid, (DateTime, DateTime))>();
    let (people_send, people_recv) = unbounded_channel::<(Pid, (Name, Phone))>();

    let (out_send, mut out_recv) = unbounded_channel::<String>();

    let mut df = dfir_syntax! {
        contacts = source_stream(contacts_recv) -> flat_map(|(pid_a, pid_b, time)| [(pid_a, (pid_b, time)), (pid_b, (pid_a, time))]);

        exposed = union();
        source_stream(diagnosed_recv) -> [0]exposed;

        new_exposed = join::<'static>()
            -> filter(|(_pid_a, ((_pid_b, t_contact), (t_from, t_to)))| {
                (t_from..=t_to).contains(&t_contact)
            })
            -> map(|(_pid_a, (pid_b_t_contact, _t_from_to))| pid_b_t_contact)
            -> tee();
        contacts -> [0]new_exposed;
        exposed -> [1]new_exposed;
        new_exposed[0] -> map(|(pid, t)| (pid, (t, t + TRANSMISSIBLE_DURATION))) -> [1]exposed;

        notifs = join::<'static>()
            -> map(|(_pid, ((name, phone), exposure))| {
                format!(
                    "[{}] To {}: Possible Exposure at t = {}",
                    name, phone, exposure,
                )
            })
            -> tee();
        notifs -> for_each(|msg| println!("{}", msg));
        notifs -> for_each(|msg| out_send.send(msg).unwrap());

        source_stream(people_recv) -> [0]notifs;
        new_exposed[1] -> [1]notifs;
    };
    assert_graphvis_snapshots!(df);

    {
        people_send
            .send((101, ("Mingwei S", "+1 650 555 7283")))
            .unwrap();
        people_send
            .send((102, ("Justin J", "+1 519 555 3458")))
            .unwrap();
        people_send
            .send((103, ("Mae M", "+1 912 555 9129")))
            .unwrap();

        contacts_send.send((101, 102, 1031)).unwrap(); // Mingwei + Justin
        contacts_send.send((101, 201, 1027)).unwrap(); // Mingwei + Joe

        let mae_diag_datetime = 1022;

        diagnosed_send
            .send((
                103, // Mae
                (
                    mae_diag_datetime,
                    mae_diag_datetime + TRANSMISSIBLE_DURATION,
                ),
            ))
            .unwrap();

        df.run_available_sync();
        let results = collect_ready::<Vec<_>, _>(&mut out_recv);
        assert_eq!([] as [String; 0], *results);
        println!("A");

        contacts_send
            .send((101, 103, mae_diag_datetime + 6))
            .unwrap(); // Mingwei + Mae

        df.run_available_sync();
        let results = collect_ready::<Vec<_>, _>(&mut out_recv);
        assert_eq!(
            [
                "[Mingwei S] To +1 650 555 7283: Possible Exposure at t = 1028",
                "[Justin J] To +1 519 555 3458: Possible Exposure at t = 1031",
                "[Mae M] To +1 912 555 9129: Possible Exposure at t = 1028",
                "[Mingwei S] To +1 650 555 7283: Possible Exposure at t = 1031",
            ],
            *results
        );
        println!("B");

        people_send
            .send((103, ("Joe H", "+1 510 555 9999")))
            .unwrap();

        df.run_available_sync();
        let results = collect_ready::<Vec<_>, _>(&mut out_recv);
        assert_eq!(
            [
                "[Mingwei S] To +1 650 555 7283: Possible Exposure at t = 1028",
                "[Mingwei S] To +1 650 555 7283: Possible Exposure at t = 1031",
                "[Justin J] To +1 519 555 3458: Possible Exposure at t = 1031",
                "[Mae M] To +1 912 555 9129: Possible Exposure at t = 1028",
                "[Joe H] To +1 510 555 9999: Possible Exposure at t = 1028"
            ],
            *results
        );
    }
}

#[multiplatform_test]
pub fn test_assert_eq() {
    let mut df = dfir_syntax! {
        source_iter([1, 2, 3]) -> assert_eq([1, 2, 3]) -> assert_eq([1, 2, 3]); // one in pull, one in push
        source_iter([1, 2, 3]) -> assert_eq([1, 2, 3]) -> assert_eq(vec![1, 2, 3]);
        source_iter([1, 2, 3]) -> assert_eq(vec![1, 2, 3]) -> assert_eq([1, 2, 3]);
        source_iter(vec![1, 2, 3]) -> assert_eq([1, 2, 3]) -> assert_eq([1, 2, 3]);
    };
    df.run_available_sync();
}

#[multiplatform_test(test)]
pub fn test_assert_failures() {
    assert!(
        std::panic::catch_unwind(|| {
            let mut df = dfir_syntax! {
                source_iter([0]) -> assert_eq([1]);
            };

            df.run_available_sync();
        })
        .is_err()
    );

    assert!(
        std::panic::catch_unwind(|| {
            let mut df = dfir_syntax! {
                source_iter([0]) -> assert_eq([1]) -> null();
            };

            df.run_available_sync();
        })
        .is_err()
    );
}

#[multiplatform_test]
pub fn test_iter_stream_batches() {
    const ITEMS: usize = 100;
    const BATCH: usize = 5;
    let stream = dfir_rs::util::iter_batches_stream(0..ITEMS, BATCH);

    // expect 5 items per tick.
    let expected: Vec<_> = (0..ITEMS)
        .map(|n| (TickInstant::new((n / BATCH).try_into().unwrap()), n))
        .collect();

    let mut df = dfir_syntax! {
        source_stream(stream)
            -> map(|x| (context.current_tick(), x))
            -> assert_eq(expected);
    };
    df.run_available_sync();
}
