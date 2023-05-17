use std::collections::HashSet;

use hydroflow::scheduled::graph::Hydroflow;
use hydroflow::util::collect_ready;
use hydroflow::{assert_graphvis_snapshots, hydroflow_syntax};
use multiplatform_test::multiplatform_test;

// TODO(mingwei): custom operators? How to handle in syntax? How to handle state?

// TODO(mingwei): Still need to handle crossing stratum boundaries
// TODO(mingwei): Implement non-monotonicity handling.

// TODO(mingwei): Tiemo user test after Tuesday.

// TODO(mingwei): Try to get more bad error messages to appear.

// TODO(joe): QOL: make a way to generate/print the mermaid graph.

// TODO(mingwei): Prevent unused variable warnings when hydroflow code is not generated.

// Joe:
// TODO(mingwei): Documentation articles.
// TODO(mingwei): Find a way to display join keys

#[multiplatform_test]
pub fn test_basic_2() {
    let (out_send, mut out_recv) = hydroflow::util::unbounded_channel::<usize>();

    let mut df = hydroflow_syntax! {
        source_iter([1]) -> for_each(|v| out_send.send(v).unwrap());
    };
    assert_graphvis_snapshots!(df);
    df.run_available();

    assert_eq!(&[1], &*collect_ready::<Vec<_>, _>(&mut out_recv));
}

#[multiplatform_test]
pub fn test_basic_3() {
    let (out_send, mut out_recv) = hydroflow::util::unbounded_channel::<usize>();

    let mut df = hydroflow_syntax! {
        source_iter([1]) -> map(|v| v + 1) -> for_each(|v| out_send.send(v).unwrap());
    };
    assert_graphvis_snapshots!(df);
    df.run_available();

    assert_eq!(&[2], &*collect_ready::<Vec<_>, _>(&mut out_recv));
}

#[multiplatform_test]
pub fn test_basic_merge() {
    let (out_send, mut out_recv) = hydroflow::util::unbounded_channel::<usize>();

    let mut df = hydroflow_syntax! {
        m = merge() -> for_each(|v| out_send.send(v).unwrap());
        source_iter([1]) -> [0]m;
        source_iter([2]) -> [1]m;
    };
    assert_graphvis_snapshots!(df);
    df.run_available();

    assert_eq!(&[1, 2], &*collect_ready::<Vec<_>, _>(&mut out_recv));
}

#[multiplatform_test]
pub fn test_basic_tee() {
    let (out_send_a, mut out_recv) = hydroflow::util::unbounded_channel::<String>();
    let out_send_b = out_send_a.clone();

    let mut df = hydroflow_syntax! {
        t = source_iter([1]) -> tee();
        t[0] -> for_each(|v| out_send_a.send(format!("A {}", v)).unwrap());
        t[1] -> for_each(|v| out_send_b.send(format!("B {}", v)).unwrap());
    };
    df.run_available();

    let out: HashSet<_> = collect_ready(&mut out_recv);
    assert_eq!(2, out.len());
    assert!(out.contains(&"A 1".to_owned()));
    assert!(out.contains(&"B 1".to_owned()));
}

#[multiplatform_test]
pub fn test_basic_inspect_null() {
    use std::cell::RefCell;
    use std::rc::Rc;

    let seen = Rc::new(RefCell::new(Vec::new()));
    let seen_inner = Rc::clone(&seen);

    let mut df = hydroflow_syntax! {
        source_iter([1, 2, 3, 4]) -> inspect(|&x| seen_inner.borrow_mut().push(x)) -> null();
    };
    df.run_available();

    assert_eq!(&[1, 2, 3, 4], &**seen.borrow());
}

// Mainly checking subgraph partitioning pull-push handling.
#[multiplatform_test]
pub fn test_large_diamond() {
    #[allow(clippy::map_identity)]
    let mut df: Hydroflow = hydroflow_syntax! {
        t = source_iter([1]) -> tee();
        j = merge() -> for_each(|x| println!("{}", x));
        t[0] -> map(std::convert::identity) -> map(std::convert::identity) -> [0]j;
        t[1] -> map(std::convert::identity) -> map(std::convert::identity) -> [1]j;
    };
    df.run_available();
}

/// Test that source_stream can handle "complex" expressions.
#[multiplatform_test]
pub fn test_recv_expr() {
    let send_recv = hydroflow::util::unbounded_channel::<usize>();

    let mut df = hydroflow_syntax! {
        source_stream(send_recv.1)
            -> for_each(|v| print!("{:?}", v));
    };
    assert_graphvis_snapshots!(df);
    df.run_available();

    let items_send = send_recv.0;
    items_send.send(9).unwrap();
    items_send.send(2).unwrap();
    items_send.send(5).unwrap();
    df.run_available();
}

#[multiplatform_test]
pub fn test_unzip() {
    let (send0, mut recv0) = hydroflow::util::unbounded_channel::<&'static str>();
    let (send1, mut recv1) = hydroflow::util::unbounded_channel::<&'static str>();
    let mut df = hydroflow_syntax! {
        my_unzip = source_iter(vec![("Hello", "Foo"), ("World", "Bar")]) -> unzip();
        my_unzip[0] -> for_each(|v| send0.send(v).unwrap());
        my_unzip[1] -> for_each(|v| send1.send(v).unwrap());
    };

    df.run_available();

    let out0: Vec<_> = collect_ready(&mut recv0);
    assert_eq!(&["Hello", "World"], &*out0);
    let out1: Vec<_> = collect_ready(&mut recv1);
    assert_eq!(&["Foo", "Bar"], &*out1);
}

#[multiplatform_test]
pub fn test_join_order() {
    let _df_good = hydroflow_syntax! {
        yikes = join() -> for_each(|m: ((), (u32, String))| println!("{:?}", m));
        source_iter([0,1,2]) -> map(|i| ((), i)) -> [0]yikes;
        source_iter(["a".to_string(),"b".to_string(),"c".to_string()]) -> map(|s| ((), s)) -> [1]yikes;
    };
    let _df_bad = hydroflow_syntax! {
        yikes = join() -> for_each(|m: ((), (u32, String))| println!("{:?}", m));
        source_iter(["a".to_string(),"b".to_string(),"c".to_string()]) -> map(|s| ((), s)) -> [1]yikes;
        source_iter([0,1,2]) -> map(|i| ((), i)) -> [0]yikes;
    };
}

#[multiplatform_test]
pub fn test_multiset_join() {
    // HalfJoinStateSetUnion
    {
        use hydroflow::compiled::pull::HalfSetJoinState;

        let (out_tx, mut out_rx) = hydroflow::util::unbounded_channel::<(usize, (usize, usize))>();

        let mut df = hydroflow_syntax! {
            my_join = join::<HalfSetJoinState>() -> for_each(|m| out_tx.send(m).unwrap());
            source_iter([(0, 1), (0, 1)]) -> [0]my_join;
            source_iter([(0, 2)]) -> [1]my_join;
        };

        df.run_available();

        let out: Vec<_> = collect_ready(&mut out_rx);
        assert_eq!(out, vec![(0, (1, 2))]);
    }

    // HalfMultisetJoinState lhs biased
    {
        use hydroflow::compiled::pull::HalfMultisetJoinState;
        let (out_tx, mut out_rx) = hydroflow::util::unbounded_channel::<(usize, (usize, usize))>();

        let mut df = hydroflow_syntax! {
            my_join = join::<HalfMultisetJoinState>() -> for_each(|m| out_tx.send(m).unwrap());
            source_iter([(1, 1), (1, 1), (1, 1)]) -> [0]my_join;
            source_iter([(1, 2), (1, 2), (1, 2), (1, 2)]) -> [1]my_join;
        };

        df.run_available();

        let out: Vec<_> = collect_ready(&mut out_rx);
        assert_eq!(out, [(1, (1, 2)); 12].to_vec());
    }

    // HalfMultisetJoinState rhs biased
    {
        use hydroflow::compiled::pull::HalfMultisetJoinState;
        let (out_tx, mut out_rx) = hydroflow::util::unbounded_channel::<(usize, (usize, usize))>();

        let mut df = hydroflow_syntax! {
            my_join = join::<HalfMultisetJoinState>() -> for_each(|m| out_tx.send(m).unwrap());
            source_iter([(1, 1), (1, 1), (1, 1), (1, 1)]) -> [0]my_join;
            source_iter([(1, 2), (1, 2), (1, 2)]) -> [1]my_join;
        };

        df.run_available();

        let out: Vec<_> = collect_ready(&mut out_rx);
        assert_eq!(out, [(1, (1, 2)); 12].to_vec());
    }
}

#[multiplatform_test]
pub fn test_cross_join() {
    let (out_send, mut out_recv) = hydroflow::util::unbounded_channel::<(usize, &str)>();

    let mut df = hydroflow_syntax! {
        cj = cross_join() -> for_each(|v| out_send.send(v).unwrap());
        source_iter([1, 2, 3]) -> [0]cj;
        source_iter(["a", "b", "c"]) -> [1]cj;
    };
    df.run_available();

    let out: HashSet<_> = collect_ready(&mut out_recv);
    assert_eq!(3 * 3, out.len());
    for n in [1, 2, 3] {
        for c in ["a", "b", "c"] {
            assert!(out.contains(&(n, c)));
        }
    }
}

#[multiplatform_test]
pub fn test_lattice_join() {
    use hydroflow::lattices::ord::Max;

    // 'static, 'tick.
    {
        let (out_tx, mut out_rx) =
            hydroflow::util::unbounded_channel::<(usize, (Max<usize>, Max<usize>))>();
        let (lhs_tx, lhs_rx) = hydroflow::util::unbounded_channel::<(usize, Max<usize>)>();
        let (rhs_tx, rhs_rx) = hydroflow::util::unbounded_channel::<(usize, Max<usize>)>();

        let mut df = hydroflow_syntax! {
            my_join = lattice_join::<'static, 'tick, Max<usize>, Max<usize>>();

            source_stream(lhs_rx) -> [0]my_join;
            source_stream(rhs_rx) -> [1]my_join;

            my_join -> for_each(|v| out_tx.send(v).unwrap());
        };

        // Merges forward correctly, without going backward
        lhs_tx.send((7, Max::new(3))).unwrap();
        lhs_tx.send((7, Max::new(4))).unwrap();
        rhs_tx.send((7, Max::new(6))).unwrap();
        rhs_tx.send((7, Max::new(5))).unwrap();

        df.run_tick();

        let out: Vec<_> = collect_ready(&mut out_rx);
        assert_eq!(out, vec![(7, (Max::new(4), Max::new(6)))]);

        // Forgets rhs state
        rhs_tx.send((7, Max::new(6))).unwrap();
        rhs_tx.send((7, Max::new(5))).unwrap();

        df.run_tick();

        let out: Vec<_> = collect_ready(&mut out_rx);
        assert_eq!(out, vec![(7, (Max::new(4), Max::new(6)))]);
    }

    // 'static, 'static.
    {
        let (out_tx, mut out_rx) =
            hydroflow::util::unbounded_channel::<(usize, (Max<usize>, Max<usize>))>();
        let (lhs_tx, lhs_rx) = hydroflow::util::unbounded_channel::<(usize, Max<usize>)>();
        let (rhs_tx, rhs_rx) = hydroflow::util::unbounded_channel::<(usize, Max<usize>)>();

        let mut df = hydroflow_syntax! {
            my_join = lattice_join::<'static, 'static, Max<usize>, Max<usize>>();

            source_stream(lhs_rx) -> [0]my_join;
            source_stream(rhs_rx) -> [1]my_join;

            my_join -> for_each(|v| out_tx.send(v).unwrap());
        };

        lhs_tx.send((7, Max::new(3))).unwrap();
        lhs_tx.send((7, Max::new(4))).unwrap();
        rhs_tx.send((7, Max::new(6))).unwrap();
        rhs_tx.send((7, Max::new(5))).unwrap();

        df.run_tick();
        let out: Vec<_> = collect_ready(&mut out_rx);
        assert_eq!(out, vec![(7, (Max::new(4), Max::new(6)))]);

        // Doesn't produce unless a lattice has changed.
        lhs_tx.send((7, Max::new(4))).unwrap();
        rhs_tx.send((7, Max::new(5))).unwrap();

        df.run_tick();
        let out: Vec<_> = collect_ready(&mut out_rx);
        assert_eq!(out, vec![]);
    }
}

#[multiplatform_test]
pub fn test_next_tick() {
    let (inp_send, inp_recv) = hydroflow::util::unbounded_channel::<usize>();
    let (out_send, mut out_recv) = hydroflow::util::unbounded_channel::<usize>();
    let mut flow = hydroflow::hydroflow_syntax! {
        inp = source_stream(inp_recv) -> tee();
        diff = difference() -> for_each(|x| out_send.send(x).unwrap());
        inp -> [pos]diff;
        inp -> next_tick() -> [neg]diff;
    };

    for x in [1, 2, 3, 4] {
        inp_send.send(x).unwrap();
    }
    flow.run_tick();

    for x in [3, 4, 5, 6] {
        inp_send.send(x).unwrap();
    }
    flow.run_tick();

    flow.run_available();
    let out: Vec<_> = collect_ready(&mut out_recv);
    assert_eq!(&[1, 2, 3, 4, 5, 6], &*out);
}

#[multiplatform_test]
pub fn test_anti_join() {
    let (inp_send, inp_recv) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let (out_send, mut out_recv) = hydroflow::util::unbounded_channel::<(usize, usize)>();
    let mut flow = hydroflow::hydroflow_syntax! {
        inp = source_stream(inp_recv) -> tee();
        diff = anti_join() -> for_each(|x| out_send.send(x).unwrap());
        inp -> [pos]diff;
        inp -> next_tick() -> map(|x: (usize, usize)| x.0) -> [neg]diff;
    };

    for x in [(1, 2), (2, 3), (3, 4), (4, 5)] {
        inp_send.send(x).unwrap();
    }
    flow.run_tick();

    for x in [(3, 2), (4, 3), (5, 4), (6, 5)] {
        inp_send.send(x).unwrap();
    }
    flow.run_tick();

    flow.run_available();
    let out: Vec<_> = collect_ready(&mut out_recv);
    assert_eq!(&[(1, 2), (2, 3), (3, 4), (4, 5), (5, 4), (6, 5)], &*out);
}

#[multiplatform_test]
pub fn test_repeat_fn() {
    // regular Fn closure
    {
        let (out_send, mut out_recv) = hydroflow::util::unbounded_channel::<usize>();
        let mut df = hydroflow::hydroflow_syntax! {
            repeat_fn(3, || 1) -> for_each(|x| out_send.send(x).unwrap());
        };

        df.run_tick();
        let out: Vec<_> = collect_ready(&mut out_recv);
        assert_eq!(vec![1, 1, 1], out);

        df.run_tick();
        let out: Vec<_> = collect_ready(&mut out_recv);
        assert_eq!(vec![1, 1, 1], out);

        df.run_tick();
        let out: Vec<_> = collect_ready(&mut out_recv);
        assert_eq!(vec![1, 1, 1], out);
    }

    // FnMut closure
    {
        let mut x = 0;

        let (out_send, mut out_recv) = hydroflow::util::unbounded_channel::<usize>();
        let mut df = hydroflow::hydroflow_syntax! {
            repeat_fn(3, move || { x += 1; x }) -> for_each(|x| out_send.send(x).unwrap());
        };

        df.run_tick();
        let out: Vec<_> = collect_ready(&mut out_recv);
        assert_eq!(vec![1, 2, 3], out);

        df.run_tick();
        let out: Vec<_> = collect_ready(&mut out_recv);
        assert_eq!(vec![4, 5, 6], out);

        df.run_tick();
        let out: Vec<_> = collect_ready(&mut out_recv);
        assert_eq!(vec![7, 8, 9], out);
    }
}

#[multiplatform_test]
pub fn test_batch() {
    let (batch1_tx, batch1_rx) = hydroflow::util::unbounded_channel::<()>();
    let (batch2_tx, batch2_rx) = hydroflow::util::unbounded_channel::<()>();
    let (tx, mut rx) = hydroflow::util::unbounded_channel::<()>();
    let mut df = hydroflow_syntax! {
        my_tee = tee();

        source_iter([()])
            -> batch(1, batch1_rx) // pull
            -> my_tee;

        my_tee -> for_each(|x| tx.send(x).unwrap());
        my_tee
            -> batch(1, batch2_rx) // push
            -> for_each(|x| tx.send(x).unwrap());
    };

    df.run_available();
    let out: Vec<_> = collect_ready(&mut rx);
    assert_eq!(out, Vec::<()>::new());

    batch1_tx.send(()).unwrap();
    df.run_available();
    let out: Vec<_> = collect_ready(&mut rx);
    assert_eq!(out, vec![()]);

    batch2_tx.send(()).unwrap();
    df.run_available();
    let out: Vec<_> = collect_ready(&mut rx);
    assert_eq!(out, vec![()]);
}

#[multiplatform_test]
pub fn test_lattice_batch() {
    type SetUnionHashSet = lattices::set_union::SetUnionHashSet<usize>;
    type SetUnionSingletonSet = lattices::set_union::SetUnionSingletonSet<usize>;

    let (batch1_tx, batch1_rx) = hydroflow::util::unbounded_channel::<()>();
    let (batch2_tx, batch2_rx) = hydroflow::util::unbounded_channel::<()>();
    let (tx_in, rx_in) = hydroflow::util::unbounded_channel::<SetUnionSingletonSet>();
    let (tx_out, mut rx_out) = hydroflow::util::unbounded_channel::<SetUnionHashSet>();
    let mut df = hydroflow_syntax! {
        my_tee = tee();

        source_stream(rx_in)
            -> lattice_batch::<SetUnionHashSet>(batch1_rx) // pull
            -> my_tee;

        my_tee
            -> for_each(|x| tx_out.send(x).unwrap());

        my_tee
            -> lattice_batch::<SetUnionHashSet>(batch2_rx) // push
            -> for_each(|x| tx_out.send(x).unwrap());
    };

    tx_in.send(SetUnionSingletonSet::new_from(0)).unwrap();
    df.run_available();
    let out: Vec<_> = collect_ready(&mut rx_out);
    assert_eq!(out, Vec::<SetUnionHashSet>::new());

    batch1_tx.send(()).unwrap();
    df.run_available();
    let out: Vec<_> = collect_ready(&mut rx_out);
    assert_eq!(out, vec![SetUnionHashSet::new_from([0])]);

    batch1_tx.send(()).unwrap();
    df.run_available();
    let out: Vec<_> = collect_ready(&mut rx_out);
    assert_eq!(out, Vec::<SetUnionHashSet>::new());

    batch2_tx.send(()).unwrap();
    df.run_available();
    let out: Vec<_> = collect_ready(&mut rx_out);
    assert_eq!(out, vec![SetUnionHashSet::new_from([0])]);
}

#[multiplatform_test]
pub fn test_batch_exceed_limit() {
    let (_, batch1_rx) = hydroflow::util::unbounded_channel::<()>();
    let (_, batch2_rx) = hydroflow::util::unbounded_channel::<()>();
    let (tx_in, rx_in) = hydroflow::util::unbounded_channel::<usize>();
    let (tx_out, mut rx_out) = hydroflow::util::unbounded_channel::<usize>();
    let mut df = hydroflow_syntax! {
        my_tee = tee();

        source_stream(rx_in)
            -> batch(1, batch1_rx) // pull
            -> my_tee;

        my_tee
            -> for_each(|x| tx_out.send(x).unwrap());

        my_tee
            -> batch(1, batch2_rx) // push
            -> for_each(|x| tx_out.send(x).unwrap());
    };

    tx_in.send(0).unwrap();
    df.run_available();
    let out: Vec<_> = collect_ready(&mut rx_out);
    assert_eq!(out, Vec::<usize>::new());

    tx_in.send(1).unwrap();
    df.run_available();
    let out: Vec<_> = collect_ready(&mut rx_out);
    assert_eq!(out, vec![0, 1, 0, 1]);

    tx_in.send(2).unwrap();
    df.run_available();
    let out: Vec<_> = collect_ready(&mut rx_out);
    assert_eq!(out, Vec::<usize>::new());
}

#[multiplatform_test]
pub fn test_sort() {
    let (items_send, items_recv) = hydroflow::util::unbounded_channel::<usize>();

    let mut df = hydroflow_syntax! {
        source_stream(items_recv)
            -> sort()
            -> for_each(|v| print!("{:?}, ", v));
    };
    assert_graphvis_snapshots!(df);
    df.run_available();

    print!("\nA: ");

    items_send.send(9).unwrap();
    items_send.send(2).unwrap();
    items_send.send(5).unwrap();
    df.run_available();

    print!("\nB: ");

    items_send.send(9).unwrap();
    items_send.send(5).unwrap();
    items_send.send(2).unwrap();
    items_send.send(0).unwrap();
    items_send.send(3).unwrap();
    df.run_available();

    println!();
}

#[multiplatform_test]
pub fn test_sort_by() {
    let mut df = hydroflow_syntax! {
        source_iter(vec!((2, 'y'), (3, 'x'), (1, 'z')))
            -> sort_by(|(k, _v)| k)
            -> for_each(|v| println!("{:?}", v));
    };
    assert_graphvis_snapshots!(df);
    df.run_available();
    println!();
}

#[multiplatform_test]
fn test_sort_by_owned() {
    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
    struct Dummy {
        x: String,
        y: i8,
    }

    let (out_send, mut out_recv) = hydroflow::util::unbounded_channel::<Dummy>();

    let dummies: Vec<Dummy> = vec![
        Dummy {
            x: "a".to_string(),
            y: 2,
        },
        Dummy {
            x: "b".to_string(),
            y: 1,
        },
    ];
    let mut dummies_saved = dummies.clone();

    let mut df = hydroflow_syntax! {
        source_iter(dummies) -> sort_by(|d| &d.x) -> for_each(|d| out_send.send(d).unwrap());
    };
    df.run_available();
    let results = collect_ready::<Vec<_>, _>(&mut out_recv);
    dummies_saved.sort_unstable_by(|d1, d2| d1.y.cmp(&d2.y));
    assert_ne!(&dummies_saved, &*results);
    dummies_saved.sort_unstable_by(|d1, d2| d1.x.cmp(&d2.x));
    assert_eq!(&dummies_saved, &*results);
}

#[multiplatform_test]
pub fn test_demux_1() {
    enum Shape {
        Circle(f64),
        Rectangle { width: f64, height: f64 },
        Square(f64),
    }

    let mut df = hydroflow_syntax! {
        my_demux = source_iter([
            Shape::Circle(5.0),
            Shape::Rectangle { width: 10.0, height: 8.0 },
            Shape::Square(9.0),
        ]) -> demux(|shape, var_args!(circ, rect)| {
            match shape {
                Shape::Circle(radius) => circ.give(radius),
                Shape::Rectangle { width, height } => rect.give((width, height)),
                Shape::Square(side) => rect.give((side, side)),
            }
        });

        out = merge() -> for_each(|a| println!("area: {}", a));

        my_demux[circ] -> map(|r| std::f64::consts::PI * r * r) -> out;
        my_demux[rect] -> map(|(w, h)| w * h) -> out;
    };
    df.run_available();
}

#[multiplatform_test]
pub fn test_demux_fizzbuzz_1() {
    let mut df = hydroflow_syntax! {
        my_demux = source_iter(1..=100)
            -> demux(|v, var_args!(fzbz, fizz, buzz, vals)|
                match v {
                    v if 0 == v % 15 => fzbz.give(()),
                    v if 0 == v % 3 => fizz.give(()),
                    v if 0 == v % 5 => buzz.give(()),
                    v => vals.give(v),
                }
            );
        my_demux[fzbz] -> for_each(|_| println!("fizzbuzz"));
        my_demux[fizz] -> for_each(|_| println!("fizz"));
        my_demux[buzz] -> for_each(|_| println!("buzz"));
        my_demux[vals] -> for_each(|x| println!("{}", x));
    };
    df.run_available();
}

#[multiplatform_test]
pub fn test_demux_fizzbuzz_2() {
    let mut df = hydroflow_syntax! {
        my_demux = source_iter(1..=100)
        -> demux(|v, var_args!(fzbz, fizz, buzz, vals)|
            match (v % 3, v % 5) {
                (0, 0) => fzbz.give(()),
                (0, _) => fizz.give(()),
                (_, 0) => buzz.give(()),
                (_, _) => vals.give(v),
            }
        );
        my_demux[fzbz] -> for_each(|_| println!("fizzbuzz"));
        my_demux[fizz] -> for_each(|_| println!("fizz"));
        my_demux[buzz] -> for_each(|_| println!("buzz"));
        my_demux[vals] -> for_each(|x| println!("{}", x));
    };
    df.run_available();
}

#[multiplatform_test]
pub fn test_channel_minimal() {
    let (send, recv) = hydroflow::util::unbounded_channel::<usize>();

    let mut df1 = hydroflow_syntax! {
        source_iter([1, 2, 3]) -> for_each(|x| { send.send(x).unwrap(); });
    };

    let mut df2 = hydroflow_syntax! {
        source_stream(recv) -> for_each(|x| println!("{}", x));
    };

    df2.run_available();
    println!("A");
    df1.run_available();
    println!("B");
    df2.run_available();
}

#[multiplatform_test]
pub fn test_surface_syntax_reachability_generated() {
    // An edge in the input data = a pair of `usize` vertex IDs.
    let (pairs_send, pairs_recv) = hydroflow::util::unbounded_channel::<(usize, usize)>();

    let mut df: Hydroflow = hydroflow_syntax! {
        reached_vertices = merge() -> map(|v| (v, ()));
        source_iter(vec![0]) -> [0]reached_vertices;

        my_join_tee = join() -> map(|(_src, ((), dst))| dst) -> tee();
        reached_vertices -> [0]my_join_tee;
        source_stream(pairs_recv) -> [1]my_join_tee;

        my_join_tee[0] -> [1]reached_vertices;
        my_join_tee[1] -> for_each(|x| println!("Reached: {}", x));
    };
    assert_graphvis_snapshots!(df);
    df.run_available();

    pairs_send.send((0, 1)).unwrap();
    df.run_available();

    pairs_send.send((2, 4)).unwrap();
    pairs_send.send((3, 4)).unwrap();
    df.run_available();

    pairs_send.send((1, 2)).unwrap();
    df.run_available();

    pairs_send.send((0, 3)).unwrap();
    df.run_available();

    pairs_send.send((0, 3)).unwrap();
    df.run_available();

    // Reached: 1
    // Reached: 2
    // Reached: 4
    // Reached: 3
    // Reached: 4
}

#[multiplatform_test]
pub fn test_transitive_closure() {
    // An edge in the input data = a pair of `usize` vertex IDs.
    let (pairs_send, pairs_recv) = hydroflow::util::unbounded_channel::<(usize, usize)>();

    let mut df = hydroflow_syntax! {
        // edge(x,y) :- link(x,y)
        edge_merge_tee = merge() -> tee();
        link_tee = tee();
        source_stream(pairs_recv) -> link_tee;
        link_tee[0] -> [0]edge_merge_tee;

        // edge(a,b) :- edge(a,k), link(k,b)
        the_join = join();
        edge_merge_tee[0] -> map(|(a, k)| (k, a)) -> [0]the_join;
        link_tee[1] -> [1]the_join;
        the_join -> map(|(_k, (a, b))| (a, b)) -> [1]edge_merge_tee;
        edge_merge_tee[1] -> for_each(|(a, b)| println!("transitive closure: ({},{})", a, b));
    };
    assert_graphvis_snapshots!(df);
    df.run_available();

    pairs_send.send((0, 1)).unwrap();
    df.run_available();

    pairs_send.send((2, 4)).unwrap();
    pairs_send.send((3, 4)).unwrap();
    df.run_available();

    pairs_send.send((1, 2)).unwrap();
    df.run_available();

    pairs_send.send((0, 3)).unwrap();
    df.run_available();

    pairs_send.send((0, 3)).unwrap();
    df.run_available();

    // transitive closure: (0,1)
    // transitive closure: (2,4)
    // transitive closure: (3,4)
    // transitive closure: (1,2)
    // transitive closure: (0,2)
    // transitive closure: (1,4)
    // transitive closure: (0,4)
    // transitive closure: (0,3)
    // transitive closure: (0,4)
    // transitive closure: (0,3)
}

#[multiplatform_test]
pub fn test_covid_tracing() {
    use hydroflow::util::unbounded_channel;

    const TRANSMISSIBLE_DURATION: usize = 14; // Days.

    type Pid = usize;
    type Name = &'static str;
    type Phone = &'static str;
    type DateTime = usize; // Days.

    let (contacts_send, contacts_recv) = unbounded_channel::<(Pid, Pid, DateTime)>();
    let (diagnosed_send, diagnosed_recv) = unbounded_channel::<(Pid, (DateTime, DateTime))>();
    let (people_send, people_recv) = unbounded_channel::<(Pid, (Name, Phone))>();

    let mut hydroflow = hydroflow_syntax! {
        contacts = source_stream(contacts_recv) -> flat_map(|(pid_a, pid_b, time)| [(pid_a, (pid_b, time)), (pid_b, (pid_a, time))]);

        exposed = merge();
        source_stream(diagnosed_recv) -> [0]exposed;

        new_exposed = (
            join() ->
            filter(|(_pid_a, ((_pid_b, t_contact), (t_from, t_to)))| {
                (t_from..=t_to).contains(&t_contact)
            }) ->
            map(|(_pid_a, (pid_b_t_contact, _t_from_to))| pid_b_t_contact) ->
            tee()
        );
        contacts -> [0]new_exposed;
        exposed -> [1]new_exposed;
        new_exposed[0] -> map(|(pid, t)| (pid, (t, t + TRANSMISSIBLE_DURATION))) -> [1]exposed;

        notifs = (
            join() ->
            for_each(|(_pid, ((name, phone), exposure))| {
                println!(
                    "[{}] To {}: Possible Exposure at t = {}",
                    name, phone, exposure
                );
            })
        );
        source_stream(people_recv) -> [0]notifs;
        new_exposed[1] -> [1]notifs;
    };
    assert_graphvis_snapshots!(hydroflow);

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

        hydroflow.run_available();
        println!("A");

        contacts_send
            .send((101, 103, mae_diag_datetime + 6))
            .unwrap(); // Mingwei + Mae

        hydroflow.run_available();
        println!("B");

        people_send
            .send((103, ("Joe H", "+1 510 555 9999")))
            .unwrap();

        hydroflow.run_available();
    }
}
