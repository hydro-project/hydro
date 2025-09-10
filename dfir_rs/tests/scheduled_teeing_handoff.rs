use std::cell::RefCell;
use std::rc::Rc;

use dfir_rs::scheduled::graph::Dfir;
use dfir_rs::scheduled::graph_ext::GraphExt;
use dfir_rs::scheduled::handoff::TeeingHandoff;
use multiplatform_test::multiplatform_test;

#[multiplatform_test]
fn test_basic() {
    let mut df = Dfir::new();
    let mut data = vec![1, 2, 3, 4];
    let (source, sink1) = df.make_edge::<_, TeeingHandoff<i32>>("ok");
    let sink2 = sink1.tee(&mut df);
    let sink3 = sink2.tee(&mut df);
    let sink4 = sink3.tee(&mut df);
    let sink5 = sink4.tee(&mut df);
    sink4.drop(&mut df);

    df.add_subgraph_source("source", source, move |_context, send| {
        send.give(std::mem::take(&mut data));
    });
    sink5.drop(&mut df);

    let out1 = Rc::new(RefCell::new(Vec::new()));
    let out1_inner = out1.clone();

    df.add_subgraph_sink("sink1", sink1, move |_context, recv| {
        for v in recv.take_inner() {
            out1_inner.borrow_mut().extend(v);
        }
    });

    let out2 = Rc::new(RefCell::new(Vec::new()));
    let out2_inner = out2.clone();
    df.add_subgraph_sink("sink2", sink2, move |_context, recv| {
        for v in recv.take_inner() {
            out2_inner.borrow_mut().extend(v);
        }
    });

    let out3 = Rc::new(RefCell::new(Vec::new()));
    let out3_inner = out3.clone();
    df.add_subgraph_sink("sink3", sink3, move |_context, recv| {
        for v in recv.take_inner() {
            out3_inner.borrow_mut().extend(v);
        }
    });
    df.run_available_sync();
    assert_eq!((*out1).borrow().clone(), vec![1, 2, 3, 4]);
    assert_eq!((*out2).borrow().clone(), vec![1, 2, 3, 4]);
    assert_eq!((*out3).borrow().clone(), vec![1, 2, 3, 4]);
}

#[multiplatform_test(test, wasm, env_tracing)]
fn test_scheduling() {
    use std::cell::RefCell;
    use std::rc::Rc;

    let mut df = Dfir::new();
    let input = Rc::new(RefCell::new(vec![1, 2, 3, 4]));
    let input_recv = Rc::clone(&input);

    let (source, sink1) = df.make_edge::<_, TeeingHandoff<i32>>("teeing-handoff");
    let sink2 = sink1.tee(&mut df);
    let sink3 = sink2.tee(&mut df);

    let src_sg_id = df.add_subgraph_source("source", source, move |_context, send| {
        let vec = input_recv.take();
        println!("! {:?}", vec);
        send.give(vec);
    });

    let out1 = Rc::new(RefCell::new(Vec::new()));
    let out1_inner = out1.clone();
    df.add_subgraph_sink("sink1", sink1, move |_context, recv| {
        for v in recv.take_inner() {
            out1_inner.borrow_mut().extend(v);
        }
    });

    let out2 = Rc::new(RefCell::new(Vec::new()));
    let out2_inner = out2.clone();
    df.add_subgraph_sink("sink2", sink2, move |_context, recv| {
        for v in recv.take_inner() {
            out2_inner.borrow_mut().extend(v);
        }
    });

    let out3 = Rc::new(RefCell::new(Vec::new()));
    let out3_inner = out3.clone();
    df.add_subgraph_sink("sink2", sink3, move |_context, recv| {
        for v in recv.take_inner() {
            out3_inner.borrow_mut().extend(v);
        }
    });

    df.run_available_sync();
    assert_eq!(&*out1.borrow(), &[1, 2, 3, 4], "out1");
    assert_eq!(&*out2.borrow(), &[1, 2, 3, 4], "out2");
    assert_eq!(&*out3.borrow(), &[1, 2, 3, 4], "out3");

    input.borrow_mut().extend(5..=8);
    df.schedule_subgraph(src_sg_id);

    df.run_available_sync();
    assert_eq!(&*out1.borrow(), &[1, 2, 3, 4, 5, 6, 7, 8], "out1");
    assert_eq!(&*out2.borrow(), &[1, 2, 3, 4, 5, 6, 7, 8], "out2");
    assert_eq!(&*out3.borrow(), &[1, 2, 3, 4, 5, 6, 7, 8], "out3");
}

/// Test with teeing after send port is already used.
#[multiplatform_test(test, wasm, env_tracing)]
fn test_scheduling_tee_after() {
    use std::cell::RefCell;
    use std::rc::Rc;

    let mut df = Dfir::new();
    let input = Rc::new(RefCell::new(vec![1, 2, 3, 4]));
    let input_recv = Rc::clone(&input);

    let (source, sink1) = df.make_edge::<_, TeeingHandoff<i32>>("teeing-handoff");

    let src_sg_id = df.add_subgraph_source("source", source, move |_context, send| {
        let vec = input_recv.take();
        println!("! {:?}", vec);
        send.give(vec);
    });

    let sink2 = sink1.tee(&mut df);
    let sink3 = sink2.tee(&mut df);

    let out1 = Rc::new(RefCell::new(Vec::new()));
    let out1_inner = out1.clone();
    df.add_subgraph_sink("sink1", sink1, move |_context, recv| {
        for v in recv.take_inner() {
            out1_inner.borrow_mut().extend(v);
        }
    });

    let out2 = Rc::new(RefCell::new(Vec::new()));
    let out2_inner = out2.clone();
    df.add_subgraph_sink("sink2", sink2, move |_context, recv| {
        for v in recv.take_inner() {
            out2_inner.borrow_mut().extend(v);
        }
    });

    let out3 = Rc::new(RefCell::new(Vec::new()));
    let out3_inner = out3.clone();
    df.add_subgraph_sink("sink2", sink3, move |_context, recv| {
        for v in recv.take_inner() {
            out3_inner.borrow_mut().extend(v);
        }
    });

    df.run_available_sync();
    assert_eq!(&*out1.borrow(), &[1, 2, 3, 4], "out1");
    assert_eq!(&*out2.borrow(), &[1, 2, 3, 4], "out2");
    assert_eq!(&*out3.borrow(), &[1, 2, 3, 4], "out3");

    input.borrow_mut().extend(5..=8);
    df.schedule_subgraph(src_sg_id);

    df.run_available_sync();
    assert_eq!(&*out1.borrow(), &[1, 2, 3, 4, 5, 6, 7, 8], "out1");
    assert_eq!(&*out2.borrow(), &[1, 2, 3, 4, 5, 6, 7, 8], "out2");
    assert_eq!(&*out3.borrow(), &[1, 2, 3, 4, 5, 6, 7, 8], "out3");
}

/// Test with dropping the initial recv port.
#[multiplatform_test(test, wasm, env_tracing)]
fn test_scheduling_drop() {
    use std::cell::RefCell;
    use std::rc::Rc;

    let mut df = Dfir::new();
    let input = Rc::new(RefCell::new(vec![1, 2, 3, 4]));
    let input_recv = Rc::clone(&input);

    let (source, sink1) = df.make_edge::<_, TeeingHandoff<i32>>("teeing-handoff");

    let src_sg_id = df.add_subgraph_source("source", source, move |_context, send| {
        let vec = input_recv.take();
        println!("! {:?}", vec);
        send.give(vec);
    });

    let sink2 = sink1.tee(&mut df);
    let sink3 = sink2.tee(&mut df);
    let sink4 = sink2.tee(&mut df);
    sink1.drop(&mut df);
    sink4.drop(&mut df);

    let out2 = Rc::new(RefCell::new(Vec::new()));
    let out2_inner = out2.clone();
    df.add_subgraph_sink("sink2", sink2, move |_context, recv| {
        for v in recv.take_inner() {
            out2_inner.borrow_mut().extend(v);
        }
    });

    let out3 = Rc::new(RefCell::new(Vec::new()));
    let out3_inner = out3.clone();
    df.add_subgraph_sink("sink2", sink3, move |_context, recv| {
        for v in recv.take_inner() {
            out3_inner.borrow_mut().extend(v);
        }
    });

    df.run_available_sync();
    assert_eq!(&*out2.borrow(), &[1, 2, 3, 4], "out2");
    assert_eq!(&*out3.borrow(), &[1, 2, 3, 4], "out3");

    input.borrow_mut().extend(5..=8);
    df.schedule_subgraph(src_sg_id);

    df.run_available_sync();
    assert_eq!(&*out2.borrow(), &[1, 2, 3, 4, 5, 6, 7, 8], "out2");
    assert_eq!(&*out3.borrow(), &[1, 2, 3, 4, 5, 6, 7, 8], "out3");
}
