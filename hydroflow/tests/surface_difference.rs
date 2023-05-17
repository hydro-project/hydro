use hydroflow::assert_graphvis_snapshots;
use hydroflow::util::collect_ready;
use multiplatform_test::multiplatform_test;

#[multiplatform_test]
pub fn test_diff_timing() {
    // An edge in the input data = a pair of `usize` vertex IDs.
    let (pos_send, pos_recv) = hydroflow::util::unbounded_channel::<usize>();
    let (neg_send, neg_recv) = hydroflow::util::unbounded_channel::<usize>();

    let mut df = hydroflow::hydroflow_syntax! {
        diff = difference() -> for_each(|x| println!("diff: {:?}", x));

        poss = source_stream(pos_recv); //-> tee();
        poss -> [pos]diff;
        // if you enable the comment below it produces the right answer
        // poss -> for_each(|x| println!("pos: {:?}", x));

        negs = source_stream(neg_recv) -> tee();
        negs -> [neg]diff;
        negs -> for_each(|x| println!("neg: {:?}", x));

    };
    assert_graphvis_snapshots!(df);

    df.run_tick();
    println!("{}x{}", df.current_tick(), df.current_stratum());

    println!("A");

    pos_send.send(1).unwrap();
    pos_send.send(2).unwrap();
    pos_send.send(3).unwrap();
    pos_send.send(4).unwrap();
    neg_send.send(2).unwrap();
    neg_send.send(3).unwrap();
    df.run_tick();

    println!("B");
    neg_send.send(1).unwrap();
    df.run_tick();
}

#[multiplatform_test]
pub fn test_diff_static() {
    // An edge in the input data = a pair of `usize` vertex IDs.
    let (pos_send, pos_recv) = hydroflow::util::unbounded_channel::<usize>();
    let (neg_send, neg_recv) = hydroflow::util::unbounded_channel::<usize>();

    let (output_send, mut output_recv) = hydroflow::util::unbounded_channel::<usize>();

    let mut df = hydroflow::hydroflow_syntax! {
        diff = difference::<'tick, 'static>() -> for_each(|v| output_send.send(v).unwrap());

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
    pos_send.send(2).unwrap();

    neg_send.send(2).unwrap();

    df.run_tick();

    assert_eq!(&[1], &*collect_ready::<Vec<_>, _>(&mut output_recv));

    pos_send.send(1).unwrap();
    pos_send.send(2).unwrap();
    pos_send.send(3).unwrap();

    df.run_tick();

    assert_eq!(&[1, 3], &*collect_ready::<Vec<_>, _>(&mut output_recv));
}
