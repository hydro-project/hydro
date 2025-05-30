use dfir_rs::util::{collect_ready, iter_batches_stream, unbounded_channel};
use dfir_rs::{assert_graphvis_snapshots, dfir_syntax};
use multiplatform_test::multiplatform_test;

#[multiplatform_test]
pub fn test_difference() {
    let (result_send, mut result_recv) = unbounded_channel::<usize>();

    let mut df = dfir_syntax! {
        source_iter([1, 2, 3, 4, 5]) -> [pos]diff;
        source_iter([2, 3, 4]) -> [neg]diff;
        diff = difference() -> for_each(|x| result_send.send(x).unwrap());
    };
    df.run_available();

    assert_eq!(&[1, 5], &*collect_ready::<Vec<_>, _>(&mut result_recv));
}

#[multiplatform_test]
pub fn test_difference_multiset() {
    let (result_send, mut result_recv) = unbounded_channel::<usize>();

    let mut df = dfir_syntax! {
        source_iter([1, 2, 2, 3, 3, 4, 4, 5, 5]) -> [pos]diff;
        source_iter([2, 3, 4]) -> [neg]diff;
        diff = difference_multiset() -> for_each(|x| result_send.send(x).unwrap());
    };
    df.run_available();

    assert_eq!(&[1, 5, 5], &*collect_ready::<Vec<_>, _>(&mut result_recv));
}

#[multiplatform_test]
pub fn test_diff_timing() {
    // An edge in the input data = a pair of `usize` vertex IDs.
    let (pos_send, pos_recv) = unbounded_channel::<usize>();
    let (neg_send, neg_recv) = unbounded_channel::<usize>();

    let (output_send, mut output_recv) = unbounded_channel::<_>();

    let mut df = dfir_syntax! {
        source_stream(pos_recv) -> [pos]diff;
        source_stream(neg_recv) -> [neg]diff;
        diff = difference() -> for_each(|x| output_send.send((context.current_tick().0, x)).unwrap());
    };
    assert_graphvis_snapshots!(df);

    df.run_tick();
    println!("{}x{}", df.current_tick(), df.current_stratum());

    println!("A");

    pos_send.send(1).unwrap();
    pos_send.send(2).unwrap();
    pos_send.send(3).unwrap();
    pos_send.send(4).unwrap();
    pos_send.send(4).unwrap();
    neg_send.send(2).unwrap();
    neg_send.send(3).unwrap();
    df.run_tick();

    assert_eq!(
        &[(1, 1), (1, 4), (1, 4)],
        &*collect_ready::<Vec<_>, _>(&mut output_recv)
    );

    println!("B");
    neg_send.send(1).unwrap();
    df.run_tick();

    assert_eq!(
        &[] as &[(u64, usize)],
        collect_ready::<Vec<_>, _>(&mut output_recv)
    );
}

#[multiplatform_test]
pub fn test_diff_static() {
    // An edge in the input data = a pair of `usize` vertex IDs.
    let (pos_send, pos_recv) = unbounded_channel::<usize>();
    let (neg_send, neg_recv) = unbounded_channel::<usize>();

    let (output_send, mut output_recv) = unbounded_channel::<usize>();

    let mut df = dfir_syntax! {
        source_stream(pos_recv) -> [pos]diff;
        source_stream(neg_recv) -> [neg]diff;
        diff = difference::<'tick, 'static>() -> sort() -> for_each(|v| output_send.send(v).unwrap());
    };
    assert_graphvis_snapshots!(df);

    pos_send.send(1).unwrap();
    pos_send.send(1).unwrap();
    pos_send.send(2).unwrap();

    neg_send.send(2).unwrap();

    df.run_tick();

    assert_eq!(&[1, 1], &*collect_ready::<Vec<_>, _>(&mut output_recv));

    pos_send.send(1).unwrap();
    pos_send.send(1).unwrap();
    pos_send.send(2).unwrap();
    pos_send.send(3).unwrap();

    df.run_tick();

    assert_eq!(&[1, 1, 3], &*collect_ready::<Vec<_>, _>(&mut output_recv));
}

#[multiplatform_test]
pub fn test_diff_multiset_timing() {
    // An edge in the input data = a pair of `usize` vertex IDs.
    let (pos_send, pos_recv) = unbounded_channel::<usize>();
    let (neg_send, neg_recv) = unbounded_channel::<usize>();

    let (output_send, mut output_recv) = unbounded_channel::<_>();

    let mut df = dfir_syntax! {
        source_stream(pos_recv) -> [pos]diff;
        source_stream(neg_recv) -> [neg]diff;
        diff = difference_multiset() -> for_each(|x| output_send.send((context.current_tick().0, x)).unwrap());
    };
    assert_graphvis_snapshots!(df);

    df.run_tick();
    println!("{}x{}", df.current_tick(), df.current_stratum());

    println!("A");

    pos_send.send(1).unwrap();
    pos_send.send(2).unwrap();
    pos_send.send(3).unwrap();
    pos_send.send(4).unwrap();
    pos_send.send(4).unwrap();
    neg_send.send(2).unwrap();
    neg_send.send(3).unwrap();
    df.run_tick();

    assert_eq!(
        &[(1, 1), (1, 4), (1, 4)],
        &*collect_ready::<Vec<_>, _>(&mut output_recv)
    );

    println!("B");
    neg_send.send(1).unwrap();
    df.run_tick();

    assert_eq!(
        &[] as &[(u64, usize)],
        collect_ready::<Vec<_>, _>(&mut output_recv)
    );
}

#[multiplatform_test]
pub fn test_diff_multiset_static() {
    // An edge in the input data = a pair of `usize` vertex IDs.
    let (pos_send, pos_recv) = unbounded_channel::<usize>();
    let (neg_send, neg_recv) = unbounded_channel::<usize>();

    let (output_send, mut output_recv) = unbounded_channel::<usize>();

    let mut df = dfir_syntax! {
        diff = difference_multiset::<'static>() -> sort() -> for_each(|v| output_send.send(v).unwrap());

        poss = source_stream(pos_recv); //-> tee();
        poss -> [pos]diff;
        // if you enable the comment below it produces the right answer
        // poss -> for_each(|x| println!("pos: {:?}", x));

        negs = source_stream(neg_recv) -> tee();
        negs -> [neg]diff;
        negs -> for_each(|x| println!("neg: {:?}", x));

    };
    assert_graphvis_snapshots!(df);

    pos_send.send(1).unwrap();
    pos_send.send(1).unwrap();
    pos_send.send(2).unwrap();

    neg_send.send(2).unwrap();

    df.run_tick();

    assert_eq!(&[1, 1], &*collect_ready::<Vec<_>, _>(&mut output_recv));

    pos_send.send(1).unwrap();
    pos_send.send(1).unwrap();
    pos_send.send(2).unwrap();
    pos_send.send(3).unwrap();

    df.run_tick();

    assert_eq!(
        &[1, 1, 1, 1, 3],
        &*collect_ready::<Vec<_>, _>(&mut output_recv)
    );
}

#[multiplatform_test]
pub fn test_diff_multiset_tick_static() {
    // An edge in the input data = a pair of `usize` vertex IDs.
    let (pos_send, pos_recv) = unbounded_channel::<usize>();
    let (neg_send, neg_recv) = unbounded_channel::<usize>();

    let (output_send, mut output_recv) = unbounded_channel::<usize>();

    let mut df = dfir_syntax! {
        diff = difference_multiset::<'tick, 'static>() -> sort() -> for_each(|v| output_send.send(v).unwrap());

        poss = source_stream(pos_recv); //-> tee();
        poss -> [pos]diff;
        // if you enable the comment below it produces the right answer
        // poss -> for_each(|x| println!("pos: {:?}", x));

        negs = source_stream(neg_recv) -> tee();
        negs -> [neg]diff;
        negs -> for_each(|x| println!("neg: {:?}", x));

    };
    assert_graphvis_snapshots!(df);

    pos_send.send(1).unwrap();
    pos_send.send(1).unwrap();
    pos_send.send(2).unwrap();

    neg_send.send(2).unwrap();

    df.run_tick();

    assert_eq!(&[1, 1], &*collect_ready::<Vec<_>, _>(&mut output_recv));

    pos_send.send(1).unwrap();
    pos_send.send(1).unwrap();
    pos_send.send(2).unwrap();
    pos_send.send(3).unwrap();

    df.run_tick();

    assert_eq!(&[1, 1, 3], &*collect_ready::<Vec<_>, _>(&mut output_recv));
}

#[multiplatform_test(wasm, test, env_tracing)]
pub fn test_diff_multiset_static_tick() {
    // An edge in the input data = a pair of `usize` vertex IDs.
    let (pos_send, pos_recv) = unbounded_channel::<usize>();
    let (neg_send, neg_recv) = unbounded_channel::<usize>();

    let (output_send, mut output_recv) = unbounded_channel::<usize>();

    let mut df = dfir_syntax! {
        diff = difference_multiset::<'static, 'tick>() -> sort() -> for_each(|v| output_send.send(v).unwrap());

        poss = source_stream(pos_recv); //-> tee();
        poss -> [pos]diff;
        // if you enable the comment below it produces the right answer
        // poss -> for_each(|x| println!("pos: {:?}", x));

        negs = source_stream(neg_recv) -> tee();
        negs -> [neg]diff;
        negs -> for_each(|x| println!("neg: {:?}", x));

    };
    assert_graphvis_snapshots!(df);

    pos_send.send(1).unwrap();
    pos_send.send(1).unwrap();
    pos_send.send(2).unwrap();

    neg_send.send(2).unwrap();

    df.run_tick();

    assert_eq!(&[1, 1], &*collect_ready::<Vec<_>, _>(&mut output_recv));

    pos_send.send(3).unwrap();

    neg_send.send(3).unwrap();

    df.run_tick();

    assert_eq!(&[1, 1, 2], &*collect_ready::<Vec<_>, _>(&mut output_recv));
}

#[multiplatform_test]
pub fn test_anti_join_multiset() {
    let (inp_send, inp_recv) = unbounded_channel::<(usize, usize)>();
    let (out_send, mut out_recv) = unbounded_channel::<(usize, usize)>();
    let mut flow = dfir_syntax! {
        inp = source_stream(inp_recv) -> tee();
        diff = anti_join_multiset() -> sort() -> for_each(|x| out_send.send(x).unwrap());
        inp -> [pos]diff;
        inp -> defer_tick() -> map(|x: (usize, usize)| x.0) -> [neg]diff;
    };

    for x in [(1, 2), (1, 2), (2, 3), (3, 4), (4, 5)] {
        inp_send.send(x).unwrap();
    }
    flow.run_tick();

    for x in [(3, 2), (4, 3), (5, 4), (6, 5)] {
        inp_send.send(x).unwrap();
    }
    flow.run_tick();

    flow.run_available();
    let out: Vec<_> = collect_ready(&mut out_recv);
    assert_eq!(
        &[(1, 2), (1, 2), (2, 3), (3, 4), (4, 5), (5, 4), (6, 5)],
        &*out
    );
}

#[multiplatform_test]
pub fn test_difference_loop_lifetimes() {
    let (result_nn_send, mut result_nn_recv) = unbounded_channel::<_>();
    let (result_nl_send, mut result_nl_recv) = unbounded_channel::<_>();
    let (result_ln_send, mut result_ln_recv) = unbounded_channel::<_>();
    let (result_ll_send, mut result_ll_recv) = unbounded_channel::<_>();

    let mut df = dfir_syntax! {
        pos = source_stream(iter_batches_stream([1, 2, 2, 3, 2, 4], 2)) -> tee();
        neg = source_stream(iter_batches_stream([3, 1, 2], 1)) -> tee();

        loop {
            pos -> batch() -> [pos]diff_nn;
            neg -> batch() -> [neg]diff_nn;
            diff_nn = difference::<'none, 'none>() -> for_each(|x| result_nn_send.send((context.loop_iter_count(), x)).unwrap());

            pos -> batch() -> [pos]diff_nl;
            neg -> batch() -> [neg]diff_nl;
            diff_nl = difference::<'none, 'loop>() -> for_each(|x| result_nl_send.send((context.loop_iter_count(), x)).unwrap());

            pos -> batch() -> [pos]diff_ln;
            neg -> batch() -> [neg]diff_ln;
            diff_ln = difference::<'loop, 'none>() -> for_each(|x| result_ln_send.send((context.loop_iter_count(), x)).unwrap());

            pos -> batch() -> [pos]diff_ll;
            neg -> batch() -> [neg]diff_ll;
            diff_ll = difference::<'loop, 'loop>() -> for_each(|x| result_ll_send.send((context.loop_iter_count(), x)).unwrap());
        };
    };
    df.run_available();

    assert_eq!(
        &[(0, 1), (0, 2), (1, 2), (1, 3), (2, 4)],
        &*collect_ready::<Vec<_>, _>(&mut result_nn_recv)
    );
    assert_eq!(
        &[(0, 1), (0, 2), (1, 2), (2, 4)],
        &*collect_ready::<Vec<_>, _>(&mut result_nl_recv)
    );
    assert_eq!(
        &[
            (0, 1),
            (0, 2),
            (1, 2),
            (1, 2),
            (1, 3),
            (2, 1),
            (2, 3),
            (2, 4)
        ],
        &*collect_ready::<Vec<_>, _>(&mut result_ln_recv)
    );
    assert_eq!(
        &[(0, 1), (0, 2), (1, 2), (1, 2), (2, 4)],
        &*collect_ready::<Vec<_>, _>(&mut result_ll_recv)
    );
}

#[multiplatform_test]
pub fn test_difference_multiset_loop_lifetimes() {
    let (result_nn_send, mut result_nn_recv) = unbounded_channel::<_>();
    let (result_nl_send, mut result_nl_recv) = unbounded_channel::<_>();
    let (result_ln_send, mut result_ln_recv) = unbounded_channel::<_>();
    let (result_ll_send, mut result_ll_recv) = unbounded_channel::<_>();

    let mut df = dfir_syntax! {
        pos = source_stream(iter_batches_stream([1, 2, 2, 3, 2, 4], 2)) -> tee();
        neg = source_stream(iter_batches_stream([3, 1, 2], 1)) -> tee();

        loop {
            pos -> batch() -> [pos]diff_nn;
            neg -> batch() -> [neg]diff_nn;
            diff_nn = difference_multiset::<'none, 'none>() -> for_each(|x| result_nn_send.send((context.loop_iter_count(), x)).unwrap());

            pos -> batch() -> [pos]diff_nl;
            neg -> batch() -> [neg]diff_nl;
            diff_nl = difference_multiset::<'none, 'loop>() -> for_each(|x| result_nl_send.send((context.loop_iter_count(), x)).unwrap());

            pos -> batch() -> [pos]diff_ln;
            neg -> batch() -> [neg]diff_ln;
            diff_ln = difference_multiset::<'loop, 'none>() -> for_each(|x| result_ln_send.send((context.loop_iter_count(), x)).unwrap());

            pos -> batch() -> [pos]diff_ll;
            neg -> batch() -> [neg]diff_ll;
            diff_ll = difference_multiset::<'loop, 'loop>() -> for_each(|x| result_ll_send.send((context.loop_iter_count(), x)).unwrap());
        };
    };
    df.run_available();

    assert_eq!(
        &[(0, 1), (0, 2), (1, 2), (1, 3), (2, 4)],
        &*collect_ready::<Vec<_>, _>(&mut result_nn_recv)
    );
    assert_eq!(
        &[(0, 1), (0, 2), (1, 2), (2, 4)],
        &*collect_ready::<Vec<_>, _>(&mut result_nl_recv)
    );
    assert_eq!(
        &[
            (0, 1),
            (0, 2),
            (1, 2),
            (1, 2),
            (1, 3),
            (2, 1),
            (2, 3),
            (2, 4)
        ],
        &*collect_ready::<Vec<_>, _>(&mut result_ln_recv)
    );
    assert_eq!(
        &[(0, 1), (0, 2), (1, 2), (1, 2), (2, 4)],
        &*collect_ready::<Vec<_>, _>(&mut result_ll_recv)
    );
}

#[multiplatform_test]
pub fn test_anti_join_types() {
    let (inp_send, inp_recv) = unbounded_channel::<(usize, usize)>();
    let df = dfir_syntax! {
        inp = source_stream(inp_recv) -> tee();
        inp -> [pos]diff;
        inp -> map(|x: (usize, usize)| x.0) -> [neg]diff;
        diff = anti_join_multiset() -> tee();
        // TODO(mingwei): enable this to work without the `<Key = usize, Value = usize>` type arguments
        // https://github.com/hydro-project/hydro/issues/1857
        diff2 = anti_join_multiset::<usize, usize>() -> for_each(|_| {});
        diff -> [pos]diff2;
        diff -> map(|x: (usize, usize)| x.0) -> [neg]diff2;
    };
    let _ = inp_send;
    assert_graphvis_snapshots!(df);
}
