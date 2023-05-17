use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::mpsc;

use hydroflow::scheduled::graph::Hydroflow;
use hydroflow::scheduled::graph_ext::GraphExt;
use hydroflow::scheduled::handoff::VecHandoff;
use hydroflow::scheduled::port::{RecvCtx, SendCtx};
use hydroflow::{var_args, var_expr};
use multiplatform_test::multiplatform_test;

#[multiplatform_test]
fn map_filter() {
    use std::cell::RefCell;
    use std::rc::Rc;

    use hydroflow::scheduled::handoff::VecHandoff;

    // A simple dataflow with one source feeding into one sink with some processing in the middle.
    let mut df = Hydroflow::new();

    let (source, map_in) = df.make_edge::<_, VecHandoff<i32>>("source -> map_in");
    let (map_out, filter_in) = df.make_edge::<_, VecHandoff<i32>>("map_out -> filter_in");
    let (filter_out, sink) = df.make_edge::<_, VecHandoff<i32>>("filter_out -> sink");

    let data = [1, 2, 3, 4];
    df.add_subgraph(
        "source",
        var_expr!(),
        var_expr!(source),
        move |_ctx, var_args!(), var_args!(send)| {
            for x in data.into_iter() {
                send.give(Some(x));
            }
        },
    );

    df.add_subgraph(
        "map",
        var_expr!(map_in),
        var_expr!(map_out),
        |_ctx, var_args!(recv), var_args!(send)| {
            for x in recv.take_inner().into_iter() {
                send.give(Some(3 * x + 1));
            }
        },
    );

    df.add_subgraph(
        "filter",
        var_expr!(filter_in),
        var_expr!(filter_out),
        |_ctx, var_args!(recv), var_args!(send)| {
            for x in recv.take_inner().into_iter() {
                if x % 2 == 0 {
                    send.give(Some(x));
                }
            }
        },
    );

    let outputs = Rc::new(RefCell::new(Vec::new()));
    let inner_outputs = outputs.clone();
    df.add_subgraph(
        "sink",
        var_expr!(sink),
        var_expr!(),
        move |_ctx, var_args!(recv), var_args!()| {
            for x in recv.take_inner().into_iter() {
                (*inner_outputs).borrow_mut().push(x);
            }
        },
    );

    df.run_available();

    assert_eq!((*outputs).borrow().clone(), vec![4, 10]);
}

#[multiplatform_test]
fn test_basic_variadic() {
    let mut df = Hydroflow::new();
    let (source_send, sink_recv) = df.make_edge::<_, VecHandoff<usize>>("handoff");
    df.add_subgraph_source("source", source_send, move |_ctx, send| {
        send.give(Some(5));
    });

    let val = <Rc<Cell<Option<usize>>>>::default();
    let val_ref = val.clone();

    df.add_subgraph_sink("sink", sink_recv, move |_ctx, recv| {
        for v in recv.take_inner().into_iter() {
            let old_val = val_ref.replace(Some(v));
            assert!(old_val.is_none()); // Only run once.
        }
    });

    df.run_available();

    assert_eq!(Some(5), val.get());
}

#[multiplatform_test]
fn test_basic_n_m() {
    let mut df = Hydroflow::new();

    let (source_send, sink_recv) = df.make_edge::<_, VecHandoff<usize>>("handoff");

    df.add_subgraph_n_m(
        "source",
        vec![],
        vec![source_send],
        move |_ctx, _recv: &[&RecvCtx<VecHandoff<usize>>], send| {
            send[0].give(Some(5));
        },
    );

    let val = <Rc<Cell<Option<usize>>>>::default();
    let val_ref = val.clone();

    df.add_subgraph_n_m(
        "sink",
        vec![sink_recv],
        vec![],
        move |_ctx, recv, _send: &[&SendCtx<VecHandoff<usize>>]| {
            for v in recv[0].take_inner().into_iter() {
                let old_val = val_ref.replace(Some(v));
                assert!(old_val.is_none()); // Only run once.
            }
        },
    );

    df.run_available();

    assert_eq!(Some(5), val.get());
}

#[multiplatform_test]
fn test_cycle() {
    // A dataflow that represents graph reachability.

    let mut edges: HashMap<usize, Vec<usize>> = HashMap::new();
    for (from, to) in [
        (1, 2),
        (1, 3),
        (1, 4),
        (2, 3),
        (2, 5),
        (5, 1),
        (6, 7),
        (7, 8),
    ] {
        edges.entry(from).or_insert_with(Vec::new).push(to);
    }

    let mut df = Hydroflow::new();

    let (reachable, merge_lhs) = df.make_edge::<_, VecHandoff<usize>>("reachable -> merge_lhs");
    let (neighbors_out, merge_rhs) =
        df.make_edge::<_, VecHandoff<usize>>("neighbors_out -> merge_rhs");
    let (merge_out, distinct_in) = df.make_edge::<_, VecHandoff<usize>>("merge_out -> distinct_in");
    let (distinct_out, tee_in) = df.make_edge::<_, VecHandoff<usize>>("distinct_out -> tee_in");
    let (tee_out1, neighbors_in) = df.make_edge::<_, VecHandoff<usize>>("tee_out1 -> neighbors_in");
    let (tee_out2, sink_in) = df.make_edge::<_, VecHandoff<usize>>("tee_out2 -> sink_in");

    let mut initially_reachable = vec![1];
    df.add_subgraph_source(
        "initially reachable source",
        reachable,
        move |_ctx, send| {
            for v in initially_reachable.drain(..) {
                send.give(Some(v));
            }
        },
    );

    df.add_subgraph_2in_out(
        "merge",
        merge_lhs,
        merge_rhs,
        merge_out,
        |_ctx, recv1, recv2, send| {
            for v in (recv1.take_inner().into_iter()).chain(recv2.take_inner().into_iter()) {
                send.give(Some(v));
            }
        },
    );

    let mut seen = HashSet::new();
    df.add_subgraph_in_out(
        "distinct",
        distinct_in,
        distinct_out,
        move |_ctx, recv, send| {
            for v in recv.take_inner().into_iter() {
                if seen.insert(v) {
                    send.give(Some(v));
                }
            }
        },
    );

    df.add_subgraph_in_out(
        "get neighbors",
        neighbors_in,
        neighbors_out,
        move |_ctx, recv, send| {
            for v in recv.take_inner().into_iter() {
                if let Some(neighbors) = edges.get(&v) {
                    for &n in neighbors {
                        send.give(Some(n));
                    }
                }
            }
        },
    );

    df.add_subgraph_in_2out(
        "tee",
        tee_in,
        tee_out1,
        tee_out2,
        |_ctx, recv, send1, send2| {
            for v in recv.take_inner().into_iter() {
                send1.give(Some(v));
                send2.give(Some(v));
            }
        },
    );

    let reachable_verts = Rc::new(RefCell::new(Vec::new()));
    let reachable_inner = reachable_verts.clone();
    df.add_subgraph_sink("sink", sink_in, move |_ctx, recv| {
        for v in recv.take_inner().into_iter() {
            (*reachable_inner).borrow_mut().push(v);
        }
    });

    df.run_available();

    assert_eq!(&*reachable_verts.borrow(), &[1, 2, 3, 4, 5]);
}

// #[test]
// fn test_auto_tee() {
//     use std::cell::RefCell;
//     use std::rc::Rc;

//     use crate::scheduled::handoff::TeeingHandoff;

//     let mut df = Hydroflow::new();

//     let mut data = vec![1, 2, 3, 4];
//     let source = df.add_source(move |send: &SendCtx<TeeingHandoff<_>>| {
//         send.give(std::mem::take(&mut data));
//     });

//     let out1 = Rc::new(RefCell::new(Vec::new()));
//     let out1_inner = out1.clone();

//     let sink1 = df.add_sink(move |recv: &RecvCtx<_>| {
//         for v in recv.take_inner() {
//             out1_inner.borrow_mut().extend(v);
//         }
//     });

//     let out2 = Rc::new(RefCell::new(Vec::new()));
//     let out2_inner = out2.clone();
//     let sink2 = df.add_sink(move |recv: &RecvCtx<_>| {
//         for v in recv.take_inner() {
//             out2_inner.borrow_mut().extend(v);
//         }
//     });

//     df.add_edge(source.clone(), sink1);
//     df.add_edge(source, sink2);

//     df.run_available();

//     assert_eq!((*out1).borrow().clone(), vec![1, 2, 3, 4]);
//     assert_eq!((*out2).borrow().clone(), vec![1, 2, 3, 4]);
// }

#[multiplatform_test]
fn test_input_handle() {
    use std::cell::RefCell;

    use hydroflow::scheduled::graph_ext::GraphExt;
    use hydroflow::scheduled::handoff::VecHandoff;

    let mut df = Hydroflow::new();

    let (send_port, recv_port) = df.make_edge::<_, VecHandoff<usize>>("input handoff");
    let input = df.add_input("input", send_port);

    let vec = Rc::new(RefCell::new(Vec::new()));
    let inner_vec = vec.clone();
    df.add_subgraph_sink("sink", recv_port, move |_ctx, recv| {
        for v in recv.take_inner() {
            (*inner_vec).borrow_mut().push(v);
        }
    });

    input.give(Some(1));
    input.give(Some(2));
    input.give(Some(3));
    input.flush();

    df.run_available();

    assert_eq!((*vec).borrow().clone(), vec![1, 2, 3]);

    input.give(Some(4));
    input.give(Some(5));
    input.give(Some(6));
    input.flush();

    df.run_available();

    assert_eq!((*vec).borrow().clone(), vec![1, 2, 3, 4, 5, 6]);
}

#[test]
// #[multiplatform_test]  // no threads on WASM
fn test_input_handle_thread() {
    use std::cell::RefCell;

    use hydroflow::scheduled::graph_ext::GraphExt;
    use hydroflow::scheduled::handoff::VecHandoff;

    let mut df = Hydroflow::new();

    let (send_port, recv_port) = df.make_edge::<_, VecHandoff<usize>>("channel handoff");
    let input = df.add_channel_input("channel", send_port);

    let vec = Rc::new(RefCell::new(Vec::new()));
    let inner_vec = vec.clone();
    df.add_subgraph_sink("sink", recv_port, move |_ctx, recv| {
        for v in recv.take_inner() {
            (*inner_vec).borrow_mut().push(v);
        }
    });

    let (done, wait) = mpsc::channel();

    std::thread::spawn(move || {
        input.give(Some(1));
        input.give(Some(2));
        input.give(Some(3));
        input.flush();
        done.send(()).unwrap();
    });

    wait.recv().unwrap();

    df.run_available();

    assert_eq!((*vec).borrow().clone(), vec![1, 2, 3]);
}

#[test]
// #[multiplatform_test]   // no threads on WASM
fn test_input_channel() {
    // This test creates two parallel Hydroflow graphs and bounces messages back
    // and forth between them.

    use std::cell::Cell;

    use futures::channel::mpsc::channel;
    use hydroflow::scheduled::graph_ext::GraphExt;
    use hydroflow::scheduled::handoff::VecHandoff;

    let (s1, r1) = channel(8000);
    let (s2, r2) = channel(8000);

    let mut s1_outer = s1.clone();
    let pairs = [(s1, r2), (s2, r1)];

    // logger/recv is a channel that each graph plops their messages into, to be
    // able to trace what happens.
    let (logger, mut recv) = channel(8000);

    for (mut sender, receiver) in pairs {
        let mut logger = logger.clone();
        std::thread::spawn(move || {
            let done = Rc::new(Cell::new(false));
            let done_inner = done.clone();
            let mut df = Hydroflow::new();

            let (in_chan, input) = df.make_edge("stream input handoff");
            df.add_input_from_stream::<_, _, VecHandoff<usize>, _>(
                "stream input",
                in_chan,
                receiver,
            );
            df.add_subgraph_sink("sink", input, move |_ctx, recv| {
                for v in recv.take_inner() {
                    logger.try_send(v).unwrap();
                    if v > 0 && sender.try_send(Some(v - 1)).is_err() {
                        (*done_inner).set(true);
                    }
                }
            });

            while !done.get() {
                df.run_available();
            }
        });
    }

    s1_outer.try_send(Some(10_usize)).unwrap();

    let mut result = Vec::new();
    let expected = vec![10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0];
    loop {
        let val = recv.try_next();
        match val {
            Err(_) => {
                if result.len() >= expected.len() {
                    break;
                }
            }
            Ok(None) => {
                break;
            }
            Ok(Some(v)) => {
                result.push(v);
            }
        }
    }
    assert_eq!(result, expected);
}
