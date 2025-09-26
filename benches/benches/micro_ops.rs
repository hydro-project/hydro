use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};
use dfir_rs::dfir_syntax;
use dfir_rs::pin_project_lite::pin_project;
use dfir_rs::pusherator::sinkerator::{self, Sinkerator};
use rand::SeedableRng;
use rand::distributions::{Distribution, Uniform};
use rand::rngs::StdRng;

fn ops(c: &mut Criterion) {
    let mut rng = StdRng::from_entropy();

    c.bench_function("micro/ops/identity", |b| {
        b.iter_batched_ref(
            || {
                const NUM_INTS: usize = 10_000;
                let dist = Uniform::new(0, 100);
                let data: Vec<usize> = (0..NUM_INTS).map(|_| dist.sample(&mut rng)).collect();

                dfir_syntax! {
                    source_iter(black_box(data)) -> identity() -> for_each(|x| { black_box(x); });
                }
            },
            |df| {
                df.run_available_sync();
            },
            BatchSize::LargeInput,
        )
    });

    c.bench_function("micro/ops/unique", |b| {
        b.iter_batched_ref(
            || {
                const NUM_INTS: usize = 10_000;
                let dist = Uniform::new(0, 100);
                let data: Vec<usize> = (0..NUM_INTS).map(|_| dist.sample(&mut rng)).collect();

                dfir_syntax! {
                    source_iter(data) -> unique() -> for_each(|x| { black_box(x); });
                }
            },
            |df| {
                df.run_available_sync();
            },
            BatchSize::LargeInput,
        )
    });

    c.bench_function("micro/ops/map", |b| {
        b.iter_batched_ref(
            || {
                const NUM_INTS: usize = 10_000;
                let dist = Uniform::new(0, 100);
                let data: Vec<usize> = (0..NUM_INTS).map(|_| dist.sample(&mut rng)).collect();

                dfir_syntax! {
                    source_iter(black_box(data)) -> map(|x| x + 1) -> for_each(|x| { black_box(x); });
                }
            },
            |df| {
                df.run_available_sync();
            },
            BatchSize::LargeInput,
        )
    });

    c.bench_function("micro/ops/flat_map", |b| {
        b.iter_batched_ref(
            || {
                const NUM_INTS: usize = 10_000;
                let dist = Uniform::new(0, 100);
                let data: Vec<usize> = (0..NUM_INTS).map(|_| dist.sample(&mut rng)).collect();

                dfir_syntax! {
                    source_iter(black_box(data)) -> flat_map(|x| [x]) -> for_each(|x| { black_box(x); });
                }
            },
            |df| {
                df.run_available_sync();
            },
            BatchSize::LargeInput,
        )
    });

    c.bench_function("micro/ops/join", |b| {
        b.iter_batched_ref(
            || {
                const NUM_INTS: usize = 10_000;
                let dist = Uniform::new(0, 100);
                let input0: Vec<(usize, ())> =
                    (0..NUM_INTS).map(|_| (dist.sample(&mut rng), ())).collect();
                let input1: Vec<(usize, ())> =
                    (0..NUM_INTS).map(|_| (dist.sample(&mut rng), ())).collect();

                dfir_syntax! {
                    my_join = join();

                    source_iter(black_box(input0)) -> [0]my_join;
                    source_iter(black_box(input1)) -> [1]my_join;

                    my_join -> for_each(|x| { black_box(x); });
                }
            },
            |df| {
                df.run_available_sync();
            },
            BatchSize::LargeInput,
        )
    });

    c.bench_function("micro/ops/difference", |b| {
        b.iter_batched_ref(
            || {
                const NUM_INTS: usize = 10_000;
                let dist = Uniform::new(0, 100);
                let input0: Vec<(usize, ())> =
                    (0..NUM_INTS).map(|_| (dist.sample(&mut rng), ())).collect();
                let input1: Vec<(usize, ())> =
                    (0..NUM_INTS).map(|_| (dist.sample(&mut rng), ())).collect();

                dfir_syntax! {
                    my_difference = difference();

                    source_iter(black_box(input0)) -> [pos]my_difference;
                    source_iter(black_box(input1)) -> [neg]my_difference;

                    my_difference -> for_each(|x| { black_box(x); });
                }
            },
            |df| {
                df.run_available_sync();
            },
            BatchSize::LargeInput,
        )
    });

    c.bench_function("micro/ops/union", |b| {
        b.iter_batched_ref(
            || {
                const NUM_INTS: usize = 10_000;
                let dist = Uniform::new(0, 100);
                let input0: Vec<usize> = (0..NUM_INTS).map(|_| dist.sample(&mut rng)).collect();
                let input1: Vec<usize> = (0..NUM_INTS).map(|_| dist.sample(&mut rng)).collect();

                dfir_syntax! {
                    my_union = union();

                    source_iter(black_box(input0)) -> my_union;
                    source_iter(black_box(input1)) -> my_union;

                    my_union -> for_each(|x| { black_box(x); });
                }
            },
            |df| {
                df.run_available_sync();
            },
            BatchSize::LargeInput,
        )
    });

    c.bench_function("micro/ops/tee", |b| {
        b.iter_batched_ref(
            || {
                const NUM_INTS: usize = 10_000;
                let dist = Uniform::new(0, 100);
                let input0: Vec<usize> = (0..NUM_INTS).map(|_| dist.sample(&mut rng)).collect();

                dfir_syntax! {
                    my_tee = tee();

                    source_iter(black_box(input0)) -> my_tee;

                    my_tee -> for_each(|x| { black_box(x); });
                    my_tee -> for_each(|x| { black_box(x); });
                }
            },
            |df| {
                df.run_available_sync();
            },
            BatchSize::LargeInput,
        )
    });

    c.bench_function("micro/ops/fold", |b| {
        b.iter_batched_ref(
            || {
                const NUM_INTS: usize = 10_000;
                let dist = Uniform::new(0, 100);
                let input0: Vec<usize> = (0..NUM_INTS).map(|_| dist.sample(&mut rng)).collect();

                {
                    dfir_syntax! {
                        source_iter(black_box(input0)) -> fold::<'tick>(|| 0, |accum: &mut _, elem| { *accum += elem }) -> for_each(|x| { black_box(x); });
                    }
                }
            },
            |df| {
                df.run_available_sync();
            },
            BatchSize::LargeInput,
        )
    });

    c.bench_function("micro/ops/sort", |b| {
        b.iter_batched_ref(
            || {
                const NUM_INTS: usize = 10_000;
                let dist = Uniform::new(0, 100);
                let input0: Vec<usize> = (0..NUM_INTS).map(|_| dist.sample(&mut rng)).collect();

                dfir_syntax! {
                    source_iter(black_box(input0)) -> sort() -> for_each(|x| { black_box(x); });
                }
            },
            |df| {
                df.run_available_sync();
            },
            BatchSize::LargeInput,
        )
    });

    // TODO:
    // This should've been called cross_join to be consistent with the rest of the benchmark names.
    // At some point we will have to edit the benchmark history to give it the correct name.
    c.bench_function("micro/ops/crossjoin", |b| {
        b.iter_batched_ref(
            || {
                const NUM_INTS: usize = 1000;
                let dist = Uniform::new(0, 100);
                let input0: Vec<usize> = (0..NUM_INTS).map(|_| dist.sample(&mut rng)).collect();
                let input1: Vec<usize> = (0..NUM_INTS).map(|_| dist.sample(&mut rng)).collect();

                dfir_syntax! {
                    my_crossjoin = cross_join();

                    source_iter(black_box(input0)) -> [0]my_crossjoin;
                    source_iter(black_box(input1)) -> [1]my_crossjoin;

                    my_crossjoin -> for_each(|x| { black_box(x); });
                }
            },
            |df| {
                df.run_available_sync();
            },
            BatchSize::LargeInput,
        )
    });

    c.bench_function("micro/ops/anti_join", |b| {
        b.iter_batched_ref(
            || {
                const NUM_INTS: usize = 1000;
                let dist = Uniform::new(0, 100);
                let input0: Vec<(usize, ())> =
                    (0..NUM_INTS).map(|_| (dist.sample(&mut rng), ())).collect();
                let input1: Vec<usize> = (0..NUM_INTS).map(|_| dist.sample(&mut rng)).collect();

                dfir_syntax! {
                    my_antijoin = anti_join();

                    source_iter(black_box(input0)) -> [pos]my_antijoin;
                    source_iter(black_box(input1)) -> [neg]my_antijoin;

                    my_antijoin -> for_each(|x| { black_box(x); });
                }
            },
            |df| {
                df.run_available_sync();
            },
            BatchSize::LargeInput,
        )
    });

    c.bench_function("micro/ops/next_tick/small", |b| {
        const DATA: [u64; 1024] = [0; 1024];

        let mut df = dfir_syntax! {
            source_iter(black_box(DATA)) -> persist::<'static>()
                -> map(black_box)
                -> defer_tick()
                -> map(black_box)
                -> defer_tick()
                -> map(black_box)
                -> defer_tick()
                -> map(black_box)
                -> defer_tick()
                -> map(black_box)
                -> defer_tick()
                -> map(black_box)
                -> defer_tick()
                -> map(black_box)
                -> defer_tick()
                -> map(black_box)
                -> defer_tick()
                -> map(black_box)
                -> defer_tick()
                -> map(black_box)
                -> defer_tick()
                -> map(black_box)
                -> for_each(|x| { black_box(x); });
        };

        b.iter(|| {
            df.run_tick_sync();
        })
    });

    c.bench_function("micro/ops/next_tick/big", |b| {
        const DATA: [[u8; 8192]; 1] = [[0; 8192]; 1];

        let mut df = dfir_syntax! {
            source_iter(black_box(DATA)) -> persist::<'static>()
                -> defer_tick()
                -> map(black_box)
                -> defer_tick()
                -> map(black_box)
                -> defer_tick()
                -> map(black_box)
                -> defer_tick()
                -> map(black_box)
                -> defer_tick()
                -> map(black_box)
                -> defer_tick()
                -> map(black_box)
                -> defer_tick()
                -> map(black_box)
                -> defer_tick()
                -> map(black_box)
                -> defer_tick()
                -> map(black_box)
                -> defer_tick()
                -> map(black_box)
                -> for_each(|x| { black_box(x); });
        };

        b.iter(|| {
            df.run_tick_sync();
        })
    });

    // TODO(mingwei): rename to `fold_keyed`
    c.bench_function("micro/ops/group_by", |b| {
        b.iter_batched_ref(
            || {
                const NUM_INTS: usize = 1000;
                let dist = Uniform::new(0, 100);
                let input0: Vec<(usize, usize)> = (0..NUM_INTS)
                    .map(|_| (dist.sample(&mut rng), dist.sample(&mut rng)))
                    .collect();

                dfir_syntax! {
                    source_iter(black_box(input0))
                        -> fold_keyed(|| 0, |x: &mut usize, n: usize| {
                            *x += n;
                        })
                        -> for_each(|x| { black_box(x); });
                }
            },
            |df| {
                df.run_available_sync();
            },
            BatchSize::LargeInput,
        )
    });
}

fn sinks(c: &mut Criterion) {
    use std::task::{Context, Poll, Waker};

    use dfir_rs::futures::sink::Sink;
    use dfir_rs::pusherator::sink::{ForEach, Map};

    let mut rng = StdRng::from_entropy();

    c.bench_function("micro/sinks/identity", |b| {
        b.to_async(
            tokio::runtime::Builder::new_current_thread()
                .build()
                .unwrap(),
        )
        .iter_batched(
            || {
                #[inline(always)]
                fn erase<Si, Item>(si: Si) -> impl Sink<Item, Error = Si::Error>
                where
                    Si: Sink<Item>,
                {
                    pin_project! {
                        struct Erase<Si> {
                            #[pin]
                            si: Si,
                        }
                    }
                    impl<Si, Item> Sink<Item> for Erase<Si>
                    where
                        Si: Sink<Item>,
                    {
                        type Error = Si::Error;

                        #[inline(always)]
                        fn poll_ready(
                            self: std::pin::Pin<&mut Self>,
                            cx: &mut Context<'_>,
                        ) -> Poll<Result<(), Self::Error>> {
                            self.project().si.poll_ready(cx)
                        }

                        #[inline(always)]
                        fn start_send(
                            self: std::pin::Pin<&mut Self>,
                            item: Item,
                        ) -> Result<(), Self::Error> {
                            self.project().si.start_send(item)
                        }

                        #[inline(always)]
                        fn poll_flush(
                            self: std::pin::Pin<&mut Self>,
                            cx: &mut Context<'_>,
                        ) -> Poll<Result<(), Self::Error>> {
                            self.project().si.poll_flush(cx)
                        }

                        #[inline(always)]
                        fn poll_close(
                            self: std::pin::Pin<&mut Self>,
                            cx: &mut Context<'_>,
                        ) -> Poll<Result<(), Self::Error>> {
                            self.project().si.poll_close(cx)
                        }
                    }
                    Erase { si }
                }

                const NUM_INTS: usize = 10_000;
                let dist = Uniform::new(0, 100);
                let data: Vec<usize> = (0..NUM_INTS).map(|_| dist.sample(&mut rng)).collect();

                let sink = Map::new(
                    std::convert::identity::<usize>,
                    erase(ForEach::new(|x| {
                        black_box(x);
                    })),
                );

                (data, erase(sink))
            },
            |(data, sink)| async move {
                // // let mut sink = std::pin::pin!(sink);
                // // let cx = &mut Context::from_waker(Waker::noop());
                // // for item in data {
                // //     assert_eq!(Poll::Ready(Ok(())), sink.as_mut().poll_ready(cx));
                // //     sink.as_mut().start_send(item).unwrap();
                // // }
                // // assert_eq!(Poll::Ready(Ok(())), sink.as_mut().poll_flush(cx));
                // use futures::sink::SinkExt;
                // let mut sink = sink;
                // for item in data {
                //     sink.feed(item).await.unwrap();
                // }
                // sink.flush().await.unwrap();
                dfir_rs::pusherator::sink::Pivot::new(data.into_iter(), sink)
                    .await
                    .unwrap();
            },
            BatchSize::LargeInput,
        )
    });

    c.bench_function("micro/sinks_sinkerator/identity", |b| {
        b.to_async(
            tokio::runtime::Builder::new_current_thread()
                .build()
                .unwrap(),
        )
        .iter_batched(
            || {
                #[inline(always)]
                fn erase<Si, Item>(si: Si) -> impl Sinkerator<Item, Error = Si::Error>
                where
                    Si: Sinkerator<Item>,
                {
                    pin_project! {
                        struct Erase<Si> {
                            #[pin]
                            si: Si,
                        }
                    }
                    impl<Si, Item> Sinkerator<Item> for Erase<Si>
                    where
                        Si: Sinkerator<Item>,
                    {
                        type Error = Si::Error;

                        #[inline(always)]
                        fn poll_send(
                            self: std::pin::Pin<&mut Self>,
                            cx: &mut Context<'_>,
                            item: Option<Item>,
                        ) -> Poll<Result<(), Self::Error>> {
                            self.project().si.poll_send(cx, item)
                        }

                        #[inline(always)]
                        fn poll_flush(
                            self: std::pin::Pin<&mut Self>,
                            cx: &mut Context<'_>,
                        ) -> Poll<Result<(), Self::Error>> {
                            self.project().si.poll_flush(cx)
                        }

                        #[inline(always)]
                        fn poll_close(
                            self: std::pin::Pin<&mut Self>,
                            cx: &mut Context<'_>,
                        ) -> Poll<Result<(), Self::Error>> {
                            self.project().si.poll_close(cx)
                        }
                    }
                    Erase { si }
                }

                const NUM_INTS: usize = 10_000;
                let dist = Uniform::new(0, 100);
                let data: Vec<usize> = (0..NUM_INTS).map(|_| dist.sample(&mut rng)).collect();

                let sinkerator = sinkerator::Map::new(
                    std::convert::identity::<usize>,
                    erase(sinkerator::ForEach::new(|x| {
                        black_box(x);
                    })),
                );

                (data, erase(sinkerator))
            },
            |(data, sinkerator)| async move {
                sinkerator::Pivot::new(data.into_iter(), sinkerator)
                    .await
                    .unwrap();
            },
            BatchSize::LargeInput,
        )
    });
}

fn sinkerators(c: &mut Criterion) {
    let mut rng = StdRng::from_entropy();

    c.bench_function("micro/codegen_sinkerators/identity", |b| {
        b.iter_batched_ref(
            || {
                const NUM_INTS: usize = 10_000;
                let dist = Uniform::new(0, 100);
                let data: Vec<usize> = (0..NUM_INTS).map(|_| dist.sample(&mut rng)).collect();

                {
                    #[allow(unused_qualifications,clippy::await_holding_refcell_ref)]
                    {
                        use::dfir_rs::{
                            var_expr,var_args
                        };
                        let mut df =  ::dfir_rs::scheduled::graph::Dfir::new();
                        df.__assign_meta_graph("{\"nodes\":[{\"value\":null,\"version\":0},{\"value\":{\"Operator\":\"source_iter (black_box (data))\"},\"version\":1},{\"value\":{\"Operator\":\"identity ()\"},\"version\":1},{\"value\":{\"Operator\":\"for_each (| x | {black_box (x) ;})\"},\"version\":1}],\"operator_tag\":[{\"value\":null,\"version\":0}],\"graph\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":2,\"version\":1},{\"idx\":3,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":1,\"version\":1},{\"idx\":2,\"version\":1}],\"version\":1}],\"ports\":[{\"value\":null,\"version\":0},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1}],\"node_loops\":[{\"value\":null,\"version\":0}],\"loop_nodes\":[{\"value\":null,\"version\":0}],\"loop_parent\":[{\"value\":null,\"version\":0}],\"root_loops\":[],\"loop_children\":[{\"value\":null,\"version\":0}],\"node_subgraph\":[{\"value\":null,\"version\":0},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1}],\"subgraph_nodes\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":1,\"version\":1},{\"idx\":2,\"version\":1},{\"idx\":3,\"version\":1}],\"version\":1}],\"subgraph_stratum\":[{\"value\":null,\"version\":0},{\"value\":0,\"version\":1}],\"node_singleton_references\":[{\"value\":null,\"version\":0},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1}],\"node_varnames\":[{\"value\":null,\"version\":0}],\"subgraph_laziness\":[{\"value\":null,\"version\":0}]}");
                        df.__assign_diagnostics("[]");
                        #[allow(non_snake_case)]
                        #[inline(always)]
                        fn op_1v1__source_iter__loc_nopath_1_0_1_0<T>(thunk:impl FnOnce() -> T) -> T {
                            thunk()
                        }
                        let mut sg_1v1_node_1v1_iter = {
                            #[inline(always)]
                            fn check_iter<IntoIter: ::std::iter::IntoIterator<Item = Item> ,Item>(into_iter:IntoIter) -> impl ::std::iter::Iterator<Item = Item>{
                                ::std::iter::IntoIterator::into_iter(into_iter)
                            }
                            check_iter(black_box(data))
                        };
                        #[allow(non_snake_case)]
                        #[inline(always)]
                        fn op_2v1__identity__loc_nopath_1_0_1_0<T>(thunk:impl FnOnce() -> T) -> T {
                            thunk()
                        }
                        #[allow(non_snake_case)]
                        #[inline(always)]
                        fn op_3v1__for_each__loc_nopath_1_0_1_0<T>(thunk:impl FnOnce() -> T) -> T {
                            thunk()
                        }
                        let sgid_1v1 = df.add_subgraph_full("Subgraph GraphSubgraphId(1v1)",0,(),(),false,None,async move|context,(),()|{
                            let op_1v1 = sg_1v1_node_1v1_iter.by_ref();
                            let op_1v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_1v1__source_iter__loc_nopath_1_0_1_0<Item,Input: ::std::iter::Iterator<Item = Item>>(input:Input) -> impl ::std::iter::Iterator<Item = Item>{
                                    #[repr(transparent)]
                                    struct Pull<Item,Input: ::std::iter::Iterator<Item = Item>>{
                                        inner:Input
                                    }
                                    impl <Item,Input: ::std::iter::Iterator<Item = Item>>Iterator for Pull<Item,Input>{
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item>{
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize,Option<usize>){
                                            self.inner.size_hint()
                                        }
                                    }

                                    Pull {
                                        inner:input
                                    }
                                }
                                op_1v1__source_iter__loc_nopath_1_0_1_0(op_1v1)
                            };
                            let op_2v1 = {
                                fn check_input<Iter: ::std::iter::Iterator<Item = Item> ,Item>(iter:Iter) -> impl ::std::iter::Iterator<Item = Item>{
                                    iter
                                }
                                check_input:: <_,_>(op_1v1)
                            };
                            let op_2v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_2v1__identity__loc_nopath_1_0_1_0<Item,Input: ::std::iter::Iterator<Item = Item>>(input:Input) -> impl ::std::iter::Iterator<Item = Item>{
                                    #[repr(transparent)]
                                    struct Pull<Item,Input: ::std::iter::Iterator<Item = Item>>{
                                        inner:Input
                                    }
                                    impl <Item,Input: ::std::iter::Iterator<Item = Item>>Iterator for Pull<Item,Input>{
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item>{
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize,Option<usize>){
                                            self.inner.size_hint()
                                        }
                                    }

                                    Pull {
                                        inner:input
                                    }
                                }
                                op_2v1__identity__loc_nopath_1_0_1_0(op_2v1)
                            };
                            let op_3v1 =  ::dfir_rs::pusherator::sinkerator::ForEach::new(|x|{
                                black_box(x);
                            });
                            // let op_3v1 = {
                            //     #[allow(non_snake_case)]
                            //     #[inline(always)]
                            //     pub fn op_3v1__for_each__loc_nopath_1_0_1_0<Item,Si>(sink:Si) -> impl ::dfir_rs::futures::sink::Sink<Item,Error =  ::dfir_rs::Never>where Si: ::dfir_rs::futures::sink::Sink<Item,Error =  ::dfir_rs::Never>{
                            //         sink
                            //     }
                            //     op_3v1__for_each__loc_nopath_1_0_1_0(op_3v1)
                            // };
                            #[inline(always)]
                            async fn pivot_run_sg_1v1<Pull,Push,Item>(pull:Pull,push:Push)
                            where
                                Pull: ::std::iter::Iterator<Item = Item>,
                                Push: dfir_rs::pusherator::sinkerator::Sinkerator<Item, Error = ::dfir_rs::Never>,
                            {
                                use dfir_rs::pusherator::sinkerator::Sinkerator;

                                // FAST!!!!!
                                struct Pivot2<Pull, Push> {
                                    pull: Pull,
                                    push: Push,
                                    pending: bool,
                                }
                                impl<Pull, Push> Future for Pivot2<Pull, Push>
                                where
                                    Pull: Iterator,
                                    Push: Sinkerator<Pull::Item, Error = ::dfir_rs::Never>,
                                {
                                    type Output = ();

                                    fn poll(
                                        mut self: std::pin::Pin<&mut Self>,
                                        cx: &mut std::task::Context<'_>,
                                    ) -> std::task::Poll<Self::Output> {
                                        let mut this = unsafe { self.get_unchecked_mut() };
                                        let mut pull = this.pull.by_ref();
                                        let mut push = unsafe { std::pin::Pin::new_unchecked(&mut this.push) };

                                        if this.pending {
                                            std::task::ready!(push.as_mut().poll_send(cx, None)).unwrap();
                                            this.pending = false;
                                        }

                                        loop {
                                            match pull.next() {
                                                None => return std::task::Poll::Ready(()),
                                                Some(item) => {
                                                    match push.as_mut().poll_send(cx, Some(item)) {
                                                        std::task::Poll::Ready(result) => {
                                                            result.unwrap();
                                                        },
                                                        std::task::Poll::Pending => {
                                                            this.pending = true;
                                                            return std::task::Poll::Pending;
                                                        },
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                Pivot2 {
                                    pull, push, pending: false
                                }.await

                                // // FAST, excitingly:
                                // struct Pivot2<Pull, Push> {
                                //     pull: Pull,
                                //     push: Push,
                                // }
                                // impl<Pull, Push> Future for Pivot2<Pull, Push>
                                // where
                                //     Pull: Iterator,
                                //     Push: Sinkerator<Pull::Item, Error = ::dfir_rs::Never>,
                                // {
                                //     type Output = ();

                                //     fn poll(
                                //         mut self: std::pin::Pin<&mut Self>,
                                //         cx: &mut std::task::Context<'_>,
                                //     ) -> std::task::Poll<Self::Output> {
                                //         let mut this = unsafe { self.get_unchecked_mut() };
                                //         let mut pull = this.pull.by_ref();
                                //         let mut push = unsafe { std::pin::Pin::new_unchecked(&mut this.push) };
                                //         loop {
                                //             match pull.next() {
                                //                 None => return std::task::Poll::Ready(()),
                                //                 Some(item) => {
                                //                     assert_eq!(std::task::Poll::Ready(Ok(())),
                                //                         push.as_mut().poll_send(cx, Some(item)));
                                //                 }
                                //             }
                                //         }
                                //     }
                                // }
                                // Pivot2 {
                                //     pull, push
                                // }.await

                                // // SLOW, interestingly.
                                // let mut pull = pull;
                                // let mut push = std::pin::pin!(push);
                                // std::future::poll_fn(|cx| {
                                //     for item in pull.by_ref() {
                                //         assert_eq!(std::task::Poll::Ready(Ok(())),
                                //             push.as_mut().poll_send(cx, Some(item)));
                                //     }
                                //     std::task::Poll::Ready(())
                                // }).await;

                                // // FAST:
                                // let mut push = std::pin::pin!(push);
                                // for item in pull {
                                //     assert_eq!(std::task::Poll::Ready(Ok(())),
                                //         push.as_mut().poll_send(&mut std::task::Context::from_waker(std::task::Waker::noop()), Some(item)));
                                // }

                                // // SLOW:
                                // let mut pull = pull;
                                // let mut push = std::pin::pin!(push);
                                // // None - end of stream
                                // // Some(None) - pending happened
                                // // Some(Some(item)) - unreachable??
                                // let mut item = pull.next().map(Some);
                                // std::future::poll_fn(|cx| {
                                //     while let Some(maybe) = item.as_mut() {
                                //         std::task::ready!(push.as_mut().poll_send(cx, maybe.take())?);
                                //         item = pull.next().map(Some);
                                //     }
                                //     std::task::Poll::Ready(Result::<(), ::dfir_rs::Never>::Ok(()))
                                // }).await.unwrap();

                                // // SLOW:
                                // for item in pull {
                                //     let mut item = Some(item);
                                //     std::future::poll_fn(|cx| {
                                //         push.as_mut().poll_send(cx, item.take())
                                //     }).await.unwrap();
                                // }
                                // // MINGWEI: no flush for now.
                                // // ::dfir_rs::futures::sink::SinkExt::flush(&mut push).await.unwrap();
                            }
                            (pivot_run_sg_1v1)(op_2v1,op_3v1).await;
                        },);
                        df
                    }
                }
            },
            |df| {
                df.run_available_sync();
            },
            BatchSize::LargeInput,
        )
    });
}

criterion_group!(micro_ops, ops, sinks, sinkerators,);
criterion_main!(micro_ops);
