use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Cursor};
use std::rc::Rc;
use std::sync::LazyLock;

use criterion::{Criterion, criterion_group, criterion_main};
use dfir_rs::dfir_syntax;
use dfir_rs::scheduled::graph_ext::GraphExt;
use differential_dataflow::input::Input;
use differential_dataflow::operators::{Iterate, Join, Threshold};

static EDGES: LazyLock<HashMap<usize, Vec<usize>>> = LazyLock::new(|| {
    let cursor = Cursor::new(include_bytes!("reachability_edges.txt"));
    let reader = BufReader::new(cursor);

    let mut edges = HashMap::<_, Vec<_>>::new();
    for line in reader.lines() {
        let line = line.unwrap();
        let mut nums = line.split_whitespace();
        let a = nums.next().unwrap().parse().unwrap();
        let b = nums.next().unwrap().parse().unwrap();
        assert!(nums.next().is_none());
        edges.entry(a).or_default().push(b);
    }
    edges
});
static EDGE_VEC: LazyLock<Vec<(usize, usize)>> = LazyLock::new(|| {
    let cursor = Cursor::new(include_bytes!("reachability_edges.txt"));
    let reader = BufReader::new(cursor);

    reader
        .lines()
        .map(|line| {
            let line = line.unwrap();
            let mut v = line.split_whitespace().map(|n| n.parse::<usize>().unwrap());
            (v.next().unwrap(), v.next().unwrap())
        })
        .collect()
});
static REACHABLE: LazyLock<HashSet<usize>> = LazyLock::new(|| {
    let cursor = Cursor::new(include_bytes!("reachability_reachable.txt"));
    let reader = BufReader::new(cursor);

    let mut set = HashSet::new();
    for line in reader.lines() {
        let line = line.unwrap();
        set.insert(line.parse().unwrap());
    }
    set
});

fn benchmark_timely(c: &mut Criterion) {
    use timely::dataflow::operators::{
        Capture, Concat, ConnectLoop, Feedback, Filter, Map, ToStream,
    };

    let edges = &*EDGES;
    let reachable = &*REACHABLE;

    c.bench_function("reachability/timely", |b| {
        b.iter(|| {
            let edges = edges.clone();
            let receiver = timely::example(|scope| {
                let mut seen = HashSet::new();

                let (handle, stream) = scope.feedback(1);

                let stream_out = (1_usize..=1)
                    .to_stream(scope)
                    .concat(&stream)
                    .flat_map(move |x| edges.get(&x).cloned().into_iter().flatten())
                    .filter(move |x| seen.insert(*x));
                stream_out.connect_loop(handle);

                stream_out.capture()
            });

            let reached: HashSet<_> = receiver
                .iter()
                .filter_map(|e| match e {
                    timely::dataflow::operators::capture::event::Event::Messages(_, vec) => {
                        Some(vec)
                    }
                    _ => None,
                })
                .flatten()
                .collect();

            assert_eq!(&reached, reachable);
        });
    });
}

fn benchmark_differential(c: &mut Criterion) {
    c.bench_function("reachability/differential", |b| {
        b.iter(move || {
            timely::execute_directly(move |worker| {
                let probe = worker.dataflow::<u32, _, _>(|scope| {
                    let edges = scope.new_collection_from(EDGE_VEC.iter().cloned()).1;
                    let roots = scope.new_collection_from(vec![1]).1;

                    let reachable = roots.iterate(|reach| {
                        edges
                            .enter(&reach.scope())
                            .semijoin(reach)
                            .map(|(_src, dst)| dst)
                            .concat(reach)
                            .distinct()
                    });

                    reachable.probe()
                });

                worker.step_while(|| !probe.done());
            });
        });
    });
}

fn benchmark_hydroflow_scheduled(c: &mut Criterion) {
    use dfir_rs::scheduled::graph::Dfir;
    use dfir_rs::scheduled::handoff::{Iter, VecHandoff};
    use dfir_rs::{var_args, var_expr};

    let edges = &*EDGES;
    let reachable = &*REACHABLE;

    c.bench_function("reachability/dfir_rs/scheduled", |b| {
        b.iter(|| {
            // A dataflow that represents graph reachability.
            let mut df = Dfir::new();

            type Hoff = VecHandoff<usize>;
            let (reachable_out, union_lhs) = df.make_edge::<_, Hoff>("reachable_out -> union_lhs");
            let (neighbors_out, union_rhs) = df.make_edge::<_, Hoff>("neighbors_out -> union_rhs");
            let (union_out, distinct_in) = df.make_edge::<_, Hoff>("union_out -> distinct_in");
            let (distinct_out, tee_in) = df.make_edge::<_, Hoff>("distinct_out -> tee_in");
            let (tee_out1, neighbors_in) = df.make_edge::<_, Hoff>("tee_out1 -> neighbors_in");
            let (tee_out2, sink_in) = df.make_edge::<_, Hoff>("tee_out2 -> sink_in");

            df.add_subgraph_source(
                "initially reachable source",
                reachable_out,
                move |_ctx, send| {
                    send.give(Some(1));
                },
            );

            let seen_handle = df.add_state::<RefCell<HashSet<usize>>>(Default::default());
            df.add_subgraph(
                "distinct",
                var_expr!(distinct_in),
                var_expr!(distinct_out),
                async move |context, var_args!(recv), var_args!(send)| {
                    let mut seen_state = unsafe {
                        // SAFETY: handle from `df.add_state(..)`.
                        context.state_ref_unchecked(seen_handle)
                    }
                    .borrow_mut();
                    let iter = recv
                        .take_inner()
                        .into_iter()
                        .filter(|v| seen_state.insert(*v));
                    send.give(Iter(iter));
                },
            );

            df.add_subgraph_2in_out(
                "union",
                union_lhs,
                union_rhs,
                union_out,
                |_ctx, recv1, recv2, send| {
                    send.give(Iter(recv1.take_inner().into_iter()));
                    send.give(Iter(recv2.take_inner().into_iter()));
                },
            );

            df.add_subgraph_in_out(
                "get neighbors",
                neighbors_in,
                neighbors_out,
                move |_ctx, recv, send| {
                    for v in recv.take_inner() {
                        if let Some(neighbors) = edges.get(&v) {
                            send.give(Iter(neighbors.iter().copied()));
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
                    for v in recv.take_inner() {
                        send1.give(Some(v));
                        send2.give(Some(v));
                    }
                },
            );

            let reachable_verts = Rc::new(RefCell::new(HashSet::new()));
            let reachable_inner = reachable_verts.clone();
            df.add_subgraph_sink("output sink", sink_in, move |_ctx, recv| {
                (*reachable_inner).borrow_mut().extend(recv.take_inner());
            });

            df.run_available_sync();

            assert_eq!(&*reachable_verts.borrow(), reachable);
        });
    });
}

fn benchmark_hydroflow(c: &mut Criterion) {
    use dfir_rs::scheduled::graph::Dfir;
    use dfir_rs::scheduled::handoff::VecHandoff;
    use dfir_rs::sinktools::{SinkBuild, ToSinkBuild, for_each};
    use dfir_rs::{var_args, var_expr};

    let edges = &*EDGES;
    let reachable = &*REACHABLE;

    c.bench_function("reachability/dfir_rs", |b| {
        b.iter(|| {
            // A dataflow that represents graph reachability.
            let mut df = Dfir::new();

            let (reachable_out, origins_in) =
                df.make_edge::<_, VecHandoff<usize>>("reachable -> origins");
            let (did_reach_out, possible_reach_in) =
                df.make_edge::<_, VecHandoff<usize>>("did_reach -> possible_reach");
            let (output_out, sink_in) = df.make_edge::<_, VecHandoff<usize>>("output -> sink");

            df.add_subgraph_source(
                "initially reachable source",
                reachable_out,
                move |_ctx, send| {
                    send.give(Some(1));
                },
            );

            let seen_handle = df.add_state::<RefCell<HashSet<usize>>>(Default::default());

            #[expect(
                clippy::await_holding_refcell_ref,
                reason = "only one borrower of `seen_handle` RefCell."
            )]
            df.add_subgraph(
                "main",
                var_expr!(origins_in, possible_reach_in),
                var_expr!(did_reach_out, output_out),
                async move |context,
                            var_args!(origins, did_reach_recv),
                            var_args!(did_reach_send, output)| {
                    let origins = origins.take_inner().into_iter();
                    let possible_reach = did_reach_recv
                        .take_inner()
                        .into_iter()
                        .filter_map(|v| edges.get(&v))
                        .flatten()
                        .copied();

                    let mut seen_state = unsafe {
                        // SAFETY: handle from `df.add_state(..)`.
                        context.state_ref_unchecked(seen_handle)
                    }
                    .borrow_mut();
                    let pull = origins
                        .chain(possible_reach)
                        .filter(|v| seen_state.insert(*v));

                    let pivot = pull.iter_to_sink_build().fanout(
                        for_each(|v| {
                            did_reach_send.give(Some(v));
                        }),
                        for_each(|v| {
                            output.give(Some(v));
                        }),
                    );

                    pivot.await.unwrap();
                },
            );

            let reachable_verts = Rc::new(RefCell::new(HashSet::new()));
            let reachable_inner = reachable_verts.clone();
            df.add_subgraph_sink("output sink", sink_in, move |_ctx, recv| {
                (*reachable_inner).borrow_mut().extend(recv.take_inner());
            });

            df.run_available_sync();

            assert_eq!(&*reachable_verts.borrow(), reachable);
        });
    });
}

fn benchmark_hydroflow_surface_cheating(c: &mut Criterion) {
    c.bench_function("reachability/dfir_rs/surface_cheating", |b| {
        b.iter_batched(
            || {
                let reachable_verts = Rc::new(RefCell::new(HashSet::new()));

                let df = {
                    let reachable_inner = reachable_verts.clone();

                    dfir_syntax! {
                        origin = source_iter([1]);
                        reached_vertices = union();
                        origin -> reached_vertices;

                        my_cheaty_join = reached_vertices -> filter_map(|v| EDGES.get(&v)) -> flatten() -> map(|&v| v);
                        my_cheaty_join -> filter(|&v| reachable_inner.borrow_mut().insert(v)) -> reached_vertices;
                    }
                };

                (df, reachable_verts)
            },
            |(mut df, reachable_verts)| {
                df.run_available_sync();
                assert_eq!(&*reachable_verts.borrow(), &*REACHABLE);
            },
            criterion::BatchSize::LargeInput,
        );
    });
}

fn benchmark_hydroflow_surface(c: &mut Criterion) {
    c.bench_function("reachability/dfir_rs/surface", |b| {
        #[expect(
            clippy::disallowed_methods,
            reason = "nondeterministic iteration order, ok for benchmarking"
        )]
        let edges: Vec<_> = EDGES
            .iter()
            .flat_map(|(&k, v)| v.iter().map(move |v| (k, *v)))
            .collect();

        b.iter_batched(
            || {
                let reachable_verts = Rc::new(RefCell::new(HashSet::new()));

                let df = {
                    let edges = edges.clone();
                    let reachable_inner = reachable_verts.clone();

                    dfir_syntax! {
                        origin = source_iter(vec![1]);
                        stream_of_edges = source_iter(edges);
                        reached_vertices = union();
                        origin -> reached_vertices;

                        my_join_tee = join() -> flat_map(|(src, ((), dst))| [src, dst]) -> tee();
                        reached_vertices -> map(|v| (v, ())) -> [0]my_join_tee;
                        stream_of_edges -> [1]my_join_tee;

                        my_join_tee -> reached_vertices;
                        my_join_tee -> for_each(|x| {
                            reachable_inner.borrow_mut().insert(x);
                        });
                    }
                };

                (df, reachable_verts)
            },
            |(mut df, reachable_verts)| {
                df.run_available_sync();
                assert_eq!(&*reachable_verts.borrow(), &*REACHABLE);
            },
            criterion::BatchSize::LargeInput,
        );
    });
}

criterion_group!(
    reachability,
    benchmark_timely,
    benchmark_differential,
    benchmark_hydroflow_scheduled,
    benchmark_hydroflow,
    benchmark_hydroflow_surface,
    benchmark_hydroflow_surface_cheating,
);
criterion_main!(reachability);
