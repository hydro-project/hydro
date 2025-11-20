use dfir_rs::dfir_syntax;
use dfir_rs::util::{collect_ready, unbounded_channel};
use multiplatform_test::multiplatform_test;

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
    flow.run_tick_sync();

    for x in [(3, 2), (4, 3), (5, 4), (6, 5)] {
        inp_send.send(x).unwrap();
    }
    flow.run_tick_sync();

    flow.run_available_sync();
    let out: Vec<_> = collect_ready(&mut out_recv);
    assert_eq!(
        &[(1, 2), (1, 2), (2, 3), (3, 4), (4, 5), (5, 4), (6, 5)],
        &*out
    );
}

#[multiplatform_test]
pub fn test_anti_join() {
    let (inp_send, inp_recv) = unbounded_channel::<(usize, usize)>();
    let (out_send, mut out_recv) = unbounded_channel::<(usize, usize)>();
    let mut flow = dfir_syntax! {
        inp = source_stream(inp_recv) -> tee();
        diff = anti_join() -> sort() -> for_each(|x| out_send.send(x).unwrap());
        inp -> [pos]diff;
        inp -> defer_tick() -> map(|x: (usize, usize)| x.0) -> [neg]diff;
    };

    for x in [(1, 2), (1, 2), (2, 3), (3, 4), (4, 5)] {
        inp_send.send(x).unwrap();
    }
    flow.run_tick_sync();

    for x in [(3, 2), (4, 3), (5, 4), (6, 5)] {
        inp_send.send(x).unwrap();
    }
    flow.run_tick_sync();

    flow.run_available_sync();
    let out: Vec<_> = collect_ready(&mut out_recv);
    assert_eq!(
        &[(1, 2), (1, 2), (2, 3), (3, 4), (4, 5), (5, 4), (6, 5)],
        &*out
    );
}

#[multiplatform_test]
pub fn test_anti_join_static() {
    let (pos_send, pos_recv) = unbounded_channel::<(usize, usize)>();
    let (neg_send, neg_recv) = unbounded_channel::<usize>();
    let (out_send, mut out_recv) = unbounded_channel::<(usize, usize)>();
    let mut flow = dfir_syntax! {
        pos = source_stream(pos_recv);
        neg = source_stream(neg_recv);
        pos -> [pos]diff_static;
        neg -> [neg]diff_static;
        diff_static = anti_join::<'static>() -> sort() -> for_each(|x| out_send.send(x).unwrap());
    };

    for x in [(1, 2), (1, 2), (200, 3), (300, 4), (400, 5), (5, 6)] {
        pos_send.send(x).unwrap();
    }
    for x in [200, 300] {
        neg_send.send(x).unwrap();
    }
    flow.run_tick_sync();
    let out: Vec<_> = collect_ready(&mut out_recv);
    assert_eq!(&[(1, 2), (1, 2), (5, 6), (400, 5)], &*out);

    neg_send.send(400).unwrap();

    flow.run_available_sync();
    let out: Vec<_> = collect_ready(&mut out_recv);
    assert_eq!(&[(1, 2), (5, 6)], &*out);
}

#[multiplatform_test]
pub fn test_anti_join_tick_static() {
    let (pos_send, pos_recv) = unbounded_channel::<(usize, usize)>();
    let (neg_send, neg_recv) = unbounded_channel::<usize>();
    let (out_send, mut out_recv) = unbounded_channel::<(usize, usize)>();
    let mut flow = dfir_syntax! {
        pos = source_stream(pos_recv);
        neg = source_stream(neg_recv);
        pos -> [pos]diff_static;
        neg -> [neg]diff_static;
        diff_static = anti_join::<'tick, 'static>() -> sort() -> for_each(|x| out_send.send(x).unwrap());
    };

    for x in [(1, 2), (1, 2), (200, 3), (300, 4), (400, 5), (5, 6)] {
        pos_send.send(x).unwrap();
    }
    for x in [200, 300] {
        neg_send.send(x).unwrap();
    }
    flow.run_tick_sync();
    let out: Vec<_> = collect_ready(&mut out_recv);
    assert_eq!(&[(1, 2), (1, 2), (5, 6), (400, 5)], &*out);

    for x in [(10, 10), (10, 10), (200, 5)] {
        pos_send.send(x).unwrap();
    }

    flow.run_available_sync();
    let out: Vec<_> = collect_ready(&mut out_recv);
    assert_eq!(&[(10, 10), (10, 10)], &*out);
}

#[multiplatform_test]
pub fn test_anti_join_multiset_tick_static() {
    let (pos_send, pos_recv) = unbounded_channel::<(usize, usize)>();
    let (neg_send, neg_recv) = unbounded_channel::<usize>();
    let (out_send, mut out_recv) = unbounded_channel::<(usize, usize)>();
    let mut flow = dfir_syntax! {
        pos = source_stream(pos_recv);
        neg = source_stream(neg_recv);
        pos -> [pos]diff_static;
        neg -> [neg]diff_static;
        diff_static = anti_join_multiset::<'tick, 'static>() -> sort() -> for_each(|x| out_send.send(x).unwrap());
    };

    for x in [(1, 2), (1, 2), (200, 3), (300, 4), (400, 5), (5, 6)] {
        pos_send.send(x).unwrap();
    }
    for x in [200, 300] {
        neg_send.send(x).unwrap();
    }
    flow.run_tick_sync();
    let out: Vec<_> = collect_ready(&mut out_recv);
    assert_eq!(&[(1, 2), (1, 2), (5, 6), (400, 5),], &*out);

    for x in [(10, 10), (10, 10), (200, 5)] {
        pos_send.send(x).unwrap();
    }

    flow.run_available_sync();
    let out: Vec<_> = collect_ready(&mut out_recv);
    assert_eq!(&[(10, 10), (10, 10)], &*out);
}

#[multiplatform_test]
pub fn test_anti_join_multiset_static() {
    let (pos_send, pos_recv) = unbounded_channel::<(usize, usize)>();
    let (neg_send, neg_recv) = unbounded_channel::<usize>();
    let (out_send, mut out_recv) = unbounded_channel::<(usize, usize)>();
    let mut flow = dfir_syntax! {
        pos = source_stream(pos_recv);
        neg = source_stream(neg_recv);
        pos -> [pos]diff_static;
        neg -> [neg]diff_static;
        diff_static = anti_join_multiset::<'static>() -> sort() -> for_each(|x| out_send.send(x).unwrap());
    };

    for x in [(1, 2), (1, 2), (200, 3), (300, 4), (400, 5), (5, 6)] {
        pos_send.send(x).unwrap();
    }
    for x in [200, 300] {
        neg_send.send(x).unwrap();
    }
    flow.run_tick_sync();
    let out: Vec<_> = collect_ready(&mut out_recv);
    assert_eq!(&[(1, 2), (1, 2), (5, 6), (400, 5)], &*out);

    neg_send.send(400).unwrap();

    flow.run_available_sync();
    let out: Vec<_> = collect_ready(&mut out_recv);
    assert_eq!(&[(1, 2), (1, 2), (5, 6)], &*out);
}
