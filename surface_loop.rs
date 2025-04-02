#![feature(prelude_import)]
#[prelude_import]
use std::prelude::rust_2024::*;
#[macro_use]
extern crate std;
use dfir_rs::util::{collect_ready, iter_batches_stream};
use dfir_rs::{assert_graphvis_snapshots, dfir_syntax};
use multiplatform_test::multiplatform_test;
extern crate test;
#[cfg(test)]
#[rustc_test_marker = "test_flo_syntax"]
#[doc(hidden)]
pub const test_flo_syntax: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_flo_syntax"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "dfir_rs\\tests\\surface_loop.rs",
        start_line: 6usize,
        start_col: 8usize,
        end_line: 6usize,
        end_col: 23usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_flo_syntax()),
    ),
};
const _: () = {};
pub fn test_flo_syntax() {
    {
        let subscriber = tracing_subscriber::FmtSubscriber::builder()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_test_writer()
            .finish();
        let _ = tracing::subscriber::set_global_default(subscriber);
    };
    {
        let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<_>();
        let mut df = {
            {
                #[allow(unused_qualifications)]
                {
                    use ::dfir_rs::{var_expr, var_args};
                    let mut df = ::dfir_rs::scheduled::graph::Dfir::new();
                    df.__assign_meta_graph(
                        "{\"nodes\":[{\"value\":null,\"version\":0},{\"value\":{\"Operator\":\"source_iter([\\\"alice\\\", \\\"bob\\\"])\"},\"version\":1},{\"value\":{\"Operator\":\"source_stream(iter_batches_stream(0 .. 12, 3))\"},\"version\":1},{\"value\":{\"Operator\":\"prefix()\"},\"version\":1},{\"value\":{\"Operator\":\"batch()\"},\"version\":1},{\"value\":{\"Operator\":\"cross_join()\"},\"version\":1},{\"value\":{\"Operator\":\"map(| item | (context.loop_iter_count(), item))\"},\"version\":1},{\"value\":{\"Operator\":\"for_each(| x | result_send.send(x).unwrap())\"},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1}],\"operator_tag\":[{\"value\":null,\"version\":0}],\"graph\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":3,\"version\":1},{\"idx\":5,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":1,\"version\":1},{\"idx\":8,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":4,\"version\":1},{\"idx\":5,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":2,\"version\":1},{\"idx\":9,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":6,\"version\":1},{\"idx\":7,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":5,\"version\":1},{\"idx\":6,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":8,\"version\":1},{\"idx\":3,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":9,\"version\":1},{\"idx\":4,\"version\":1}],\"version\":1}],\"ports\":[{\"value\":null,\"version\":0},{\"value\":[\"Elided\",{\"Int\":\"0\"}],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",{\"Int\":\"1\"}],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1}],\"node_loops\":[{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1}],\"loop_nodes\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":3,\"version\":1},{\"idx\":4,\"version\":1},{\"idx\":5,\"version\":1},{\"idx\":6,\"version\":1},{\"idx\":7,\"version\":1}],\"version\":1}],\"loop_parent\":[{\"value\":null,\"version\":0}],\"root_loops\":[{\"idx\":1,\"version\":1}],\"loop_children\":[{\"value\":null,\"version\":0},{\"value\":[],\"version\":1}],\"node_subgraph\":[{\"value\":null,\"version\":0},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1}],\"subgraph_nodes\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":1,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":2,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":3,\"version\":1},{\"idx\":4,\"version\":1},{\"idx\":5,\"version\":1},{\"idx\":6,\"version\":1},{\"idx\":7,\"version\":1}],\"version\":1}],\"subgraph_stratum\":[{\"value\":null,\"version\":0},{\"value\":0,\"version\":1},{\"value\":0,\"version\":1},{\"value\":1,\"version\":1}],\"node_singleton_references\":[{\"value\":null,\"version\":0},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1}],\"node_varnames\":[{\"value\":null,\"version\":0},{\"value\":\"users\",\"version\":1},{\"value\":\"messages\",\"version\":1},{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":\"cp\",\"version\":1},{\"value\":\"cp\",\"version\":1},{\"value\":\"cp\",\"version\":1}],\"subgraph_laziness\":[{\"value\":null,\"version\":0}]}",
                    );
                    df.__assign_diagnostics("[]");
                    let (hoff_8v1_send, hoff_8v1_recv) = df
                        .make_edge::<
                            _,
                            ::dfir_rs::scheduled::handoff::VecHandoff<_>,
                        >("handoff GraphNodeId(8v1)");
                    let (hoff_9v1_send, hoff_9v1_recv) = df
                        .make_edge::<
                            _,
                            ::dfir_rs::scheduled::handoff::VecHandoff<_>,
                        >("handoff GraphNodeId(9v1)");
                    let loop_1v1 = df.add_loop(None);
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_1v1__source_iter__loc_dfir_rs_tests_surface_loop_rs_10_17_10_46<
                        T,
                    >(thunk: impl FnOnce() -> T) -> T {
                        thunk()
                    }
                    let mut sg_1v1_node_1v1_iter = {
                        #[inline(always)]
                        fn check_iter<
                            IntoIter: ::std::iter::IntoIterator<Item = Item>,
                            Item,
                        >(
                            into_iter: IntoIter,
                        ) -> impl ::std::iter::Iterator<Item = Item> {
                            ::std::iter::IntoIterator::into_iter(into_iter)
                        }
                        check_iter(["alice", "bob"])
                    };
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_2v1__source_stream__loc_dfir_rs_tests_surface_loop_rs_11_20_11_64<
                        T,
                    >(thunk: impl FnOnce() -> T) -> T {
                        thunk()
                    }
                    let mut sg_2v1_node_2v1_stream = {
                        #[inline(always)]
                        fn check_stream<
                            Stream: ::dfir_rs::futures::stream::Stream<Item = Item>
                                + ::std::marker::Unpin,
                            Item,
                        >(
                            stream: Stream,
                        ) -> impl ::dfir_rs::futures::stream::Stream<
                            Item = Item,
                        > + ::std::marker::Unpin {
                            stream
                        }
                        check_stream(iter_batches_stream(0..12, 3))
                    };
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_3v1__prefix__loc_dfir_rs_tests_surface_loop_rs_13_22_13_30<T>(
                        thunk: impl FnOnce() -> T,
                    ) -> T {
                        thunk()
                    }
                    #[allow(clippy::redundant_closure_call)]
                    let singleton_op_3v1 = df
                        .add_state(::std::cell::RefCell::new(::std::vec::Vec::new()));
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_4v1__batch__loc_dfir_rs_tests_surface_loop_rs_14_25_14_32<T>(
                        thunk: impl FnOnce() -> T,
                    ) -> T {
                        thunk()
                    }
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_5v1__cross_join__loc_dfir_rs_tests_surface_loop_rs_15_18_15_30<
                        T,
                    >(thunk: impl FnOnce() -> T) -> T {
                        thunk()
                    }
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_6v1__map__loc_dfir_rs_tests_surface_loop_rs_16_20_16_65<T>(
                        thunk: impl FnOnce() -> T,
                    ) -> T {
                        thunk()
                    }
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_7v1__for_each__loc_dfir_rs_tests_surface_loop_rs_17_20_17_62<
                        T,
                    >(thunk: impl FnOnce() -> T) -> T {
                        thunk()
                    }
                    let sgid_1v1 = df
                        .add_subgraph_full(
                            "Subgraph GraphSubgraphId(1v1)",
                            0,
                            (),
                            (hoff_8v1_send, ()),
                            false,
                            None,
                            move |context, (), (hoff_8v1_send, ())| {
                                let hoff_8v1_send = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                    v|
                                {
                                    hoff_8v1_send.give(Some(v));
                                });
                                let op_1v1 = sg_1v1_node_1v1_iter.by_ref();
                                let op_1v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_1v1__source_iter__loc_dfir_rs_tests_surface_loop_rs_10_17_10_46<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_1v1__source_iter__loc_dfir_rs_tests_surface_loop_rs_10_17_10_46(
                                        op_1v1,
                                    )
                                };
                                #[inline(always)]
                                fn pivot_run_sg_1v1<
                                    Pull: ::std::iter::Iterator<Item = Item>,
                                    Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    Item,
                                >(pull: Pull, push: Push) {
                                    ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                                }
                                pivot_run_sg_1v1(op_1v1, hoff_8v1_send);
                            },
                        );
                    let sgid_2v1 = df
                        .add_subgraph_full(
                            "Subgraph GraphSubgraphId(2v1)",
                            0,
                            (),
                            (hoff_9v1_send, ()),
                            false,
                            None,
                            move |context, (), (hoff_9v1_send, ())| {
                                let hoff_9v1_send = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                    v|
                                {
                                    hoff_9v1_send.give(Some(v));
                                });
                                let op_2v1 = std::iter::from_fn(|| {
                                    match ::dfir_rs::futures::stream::Stream::poll_next(
                                        ::std::pin::Pin::new(&mut sg_2v1_node_2v1_stream),
                                        &mut std::task::Context::from_waker(&context.waker()),
                                    ) {
                                        std::task::Poll::Ready(maybe) => maybe,
                                        std::task::Poll::Pending => None,
                                    }
                                });
                                let op_2v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_2v1__source_stream__loc_dfir_rs_tests_surface_loop_rs_11_20_11_64<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_2v1__source_stream__loc_dfir_rs_tests_surface_loop_rs_11_20_11_64(
                                        op_2v1,
                                    )
                                };
                                #[inline(always)]
                                fn pivot_run_sg_2v1<
                                    Pull: ::std::iter::Iterator<Item = Item>,
                                    Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    Item,
                                >(pull: Pull, push: Push) {
                                    ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                                }
                                pivot_run_sg_2v1(op_2v1, hoff_9v1_send);
                            },
                        );
                    let sgid_3v1 = df
                        .add_subgraph_full(
                            "Subgraph GraphSubgraphId(3v1)",
                            1,
                            (hoff_8v1_recv, (hoff_9v1_recv, ())),
                            (),
                            false,
                            Some(loop_1v1),
                            move |context, (hoff_8v1_recv, (hoff_9v1_recv, ())), ()| {
                                let mut hoff_8v1_recv = hoff_8v1_recv.borrow_mut_swap();
                                let hoff_8v1_recv = hoff_8v1_recv.drain(..);
                                let mut hoff_9v1_recv = hoff_9v1_recv.borrow_mut_swap();
                                let hoff_9v1_recv = hoff_9v1_recv.drain(..);
                                let mut sg_3v1_node_3v1_vec = unsafe {
                                    context.state_ref_unchecked(singleton_op_3v1)
                                }
                                    .borrow_mut();
                                ::std::iter::Extend::extend(
                                    &mut *sg_3v1_node_3v1_vec,
                                    hoff_8v1_recv,
                                );
                                let op_3v1 = ::std::iter::IntoIterator::into_iter(
                                    ::std::clone::Clone::clone(&*sg_3v1_node_3v1_vec),
                                );
                                let op_3v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_3v1__prefix__loc_dfir_rs_tests_surface_loop_rs_13_22_13_30<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_3v1__prefix__loc_dfir_rs_tests_surface_loop_rs_13_22_13_30(
                                        op_3v1,
                                    )
                                };
                                let op_4v1 = {
                                    fn check_input<
                                        Iter: ::std::iter::Iterator<Item = Item>,
                                        Item,
                                    >(iter: Iter) -> impl ::std::iter::Iterator<Item = Item> {
                                        iter
                                    }
                                    check_input::<_, _>(hoff_9v1_recv)
                                };
                                let op_4v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_4v1__batch__loc_dfir_rs_tests_surface_loop_rs_14_25_14_32<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_4v1__batch__loc_dfir_rs_tests_surface_loop_rs_14_25_14_32(
                                        op_4v1,
                                    )
                                };
                                let op_3v1 = op_3v1.map(|a| ((), a));
                                let op_4v1 = op_4v1.map(|b| ((), b));
                                let mut sg_3v1_node_5v1_joindata_lhs_borrow = ::std::default::Default::default();
                                let mut sg_3v1_node_5v1_joindata_rhs_borrow = ::std::default::Default::default();
                                let op_5v1 = {
                                    #[inline(always)]
                                    fn check_inputs<'a, K, I1, V1, I2, V2>(
                                        lhs: I1,
                                        rhs: I2,
                                        lhs_state: &'a mut ::dfir_rs::compiled::pull::HalfSetJoinState<
                                            K,
                                            V1,
                                            V2,
                                        >,
                                        rhs_state: &'a mut ::dfir_rs::compiled::pull::HalfSetJoinState<
                                            K,
                                            V2,
                                            V1,
                                        >,
                                        is_new_tick: bool,
                                    ) -> impl 'a + Iterator<Item = (K, (V1, V2))>
                                    where
                                        K: Eq + std::hash::Hash + Clone,
                                        V1: Clone + ::std::cmp::Eq,
                                        V2: Clone + ::std::cmp::Eq,
                                        I1: 'a + Iterator<Item = (K, V1)>,
                                        I2: 'a + Iterator<Item = (K, V2)>,
                                    {
                                        op_5v1__cross_join__loc_dfir_rs_tests_surface_loop_rs_15_18_15_30(||
                                        ::dfir_rs::compiled::pull::symmetric_hash_join_into_iter(
                                            lhs,
                                            rhs,
                                            lhs_state,
                                            rhs_state,
                                            is_new_tick,
                                        ))
                                    }
                                    check_inputs(
                                        op_3v1,
                                        op_4v1,
                                        &mut sg_3v1_node_5v1_joindata_lhs_borrow,
                                        &mut sg_3v1_node_5v1_joindata_rhs_borrow,
                                        true,
                                    )
                                };
                                let op_5v1 = op_5v1.map(|((), (a, b))| (a, b));
                                let op_5v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_5v1__cross_join__loc_dfir_rs_tests_surface_loop_rs_15_18_15_30<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_5v1__cross_join__loc_dfir_rs_tests_surface_loop_rs_15_18_15_30(
                                        op_5v1,
                                    )
                                };
                                #[allow(
                                    clippy::map_clone,
                                    reason = "dfir has no explicit `cloned`/`copied` operator"
                                )]
                                let op_6v1 = op_5v1
                                    .map(|item| (context.loop_iter_count(), item));
                                let op_6v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_6v1__map__loc_dfir_rs_tests_surface_loop_rs_16_20_16_65<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_6v1__map__loc_dfir_rs_tests_surface_loop_rs_16_20_16_65(
                                        op_6v1,
                                    )
                                };
                                let op_7v1 = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                    x|
                                result_send.send(x).unwrap());
                                let op_7v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_7v1__for_each__loc_dfir_rs_tests_surface_loop_rs_17_20_17_62<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    >(
                                        input: Input,
                                    ) -> impl ::dfir_rs::pusherator::Pusherator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Push<
                                            Item,
                                            Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                        > ::dfir_rs::pusherator::Pusherator for Push<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn give(&mut self, item: Self::Item) {
                                                self.inner.give(item)
                                            }
                                        }
                                        Push { inner: input }
                                    }
                                    op_7v1__for_each__loc_dfir_rs_tests_surface_loop_rs_17_20_17_62(
                                        op_7v1,
                                    )
                                };
                                #[inline(always)]
                                fn pivot_run_sg_3v1<
                                    Pull: ::std::iter::Iterator<Item = Item>,
                                    Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    Item,
                                >(pull: Pull, push: Push) {
                                    ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                                }
                                pivot_run_sg_3v1(op_6v1, op_7v1);
                                context.allow_another_iteration();
                                context.allow_another_iteration();
                            },
                        );
                    df
                }
            }
        };
        {
            #[cfg(not(target_arch = "wasm32"))]
            {
                {
                    let mut settings = ::insta::Settings::clone_current();
                    settings._private_inner_mut().snapshot_suffix("graphvis_mermaid");
                    settings
                        .bind(|| {
                            ::insta::_macro_support::assert_snapshot(
                                    ::insta::_macro_support::AutoName.into(),
                                    #[allow(clippy::redundant_closure_call)]
                                    &(|v| ::alloc::__export::must_use({
                                        let res = ::alloc::fmt::format(format_args!("{0}", v));
                                        res
                                    }))(
                                        &df.meta_graph().unwrap().to_mermaid(&Default::default()),
                                    ),
                                    "F:\\Projects\\hydroflow\\dfir_rs",
                                    {
                                        fn f() {}
                                        fn type_name_of_val<T>(_: T) -> &'static str {
                                            std::any::type_name::<T>()
                                        }
                                        let mut name = type_name_of_val(f)
                                            .strip_suffix("::f")
                                            .unwrap_or("");
                                        while let Some(rest) = name.strip_suffix("::{{closure}}") {
                                            name = rest;
                                        }
                                        name
                                    },
                                    "surface_loop",
                                    "dfir_rs\\tests\\surface_loop.rs",
                                    20u32,
                                    "df.meta_graph().unwrap().to_mermaid(& Default :: default())",
                                )
                                .unwrap();
                        })
                };
                {
                    let mut settings = ::insta::Settings::clone_current();
                    settings._private_inner_mut().snapshot_suffix("graphvis_dot");
                    settings
                        .bind(|| {
                            ::insta::_macro_support::assert_snapshot(
                                    ::insta::_macro_support::AutoName.into(),
                                    #[allow(clippy::redundant_closure_call)]
                                    &(|v| ::alloc::__export::must_use({
                                        let res = ::alloc::fmt::format(format_args!("{0}", v));
                                        res
                                    }))(&df.meta_graph().unwrap().to_dot(&Default::default())),
                                    "F:\\Projects\\hydroflow\\dfir_rs",
                                    {
                                        fn f() {}
                                        fn type_name_of_val<T>(_: T) -> &'static str {
                                            std::any::type_name::<T>()
                                        }
                                        let mut name = type_name_of_val(f)
                                            .strip_suffix("::f")
                                            .unwrap_or("");
                                        while let Some(rest) = name.strip_suffix("::{{closure}}") {
                                            name = rest;
                                        }
                                        name
                                    },
                                    "surface_loop",
                                    "dfir_rs\\tests\\surface_loop.rs",
                                    20u32,
                                    "df.meta_graph().unwrap().to_dot(& Default :: default())",
                                )
                                .unwrap();
                        })
                };
            }
        };
        df.run_available();
        match (
            &&[
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
            ],
            &&*collect_ready::<Vec<_>, _>(&mut result_recv),
        ) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
}
extern crate test;
#[cfg(test)]
#[rustc_test_marker = "test_flo_nested"]
#[doc(hidden)]
pub const test_flo_nested: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_flo_nested"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "dfir_rs\\tests\\surface_loop.rs",
        start_line: 55usize,
        start_col: 8usize,
        end_line: 55usize,
        end_col: 23usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_flo_nested()),
    ),
};
const _: () = {};
pub fn test_flo_nested() {
    {
        let subscriber = tracing_subscriber::FmtSubscriber::builder()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_test_writer()
            .finish();
        let _ = tracing::subscriber::set_global_default(subscriber);
    };
    {
        let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<_>();
        let mut df = {
            {
                #[allow(unused_qualifications)]
                {
                    use ::dfir_rs::{var_expr, var_args};
                    let mut df = ::dfir_rs::scheduled::graph::Dfir::new();
                    df.__assign_meta_graph(
                        "{\"nodes\":[{\"value\":null,\"version\":0},{\"value\":{\"Operator\":\"source_iter([\\\"alice\\\", \\\"bob\\\"])\"},\"version\":1},{\"value\":{\"Operator\":\"source_stream(iter_batches_stream(0 .. 12, 3))\"},\"version\":1},{\"value\":{\"Operator\":\"prefix()\"},\"version\":1},{\"value\":{\"Operator\":\"batch()\"},\"version\":1},{\"value\":{\"Operator\":\"cross_join()\"},\"version\":1},{\"value\":{\"Operator\":\"all_once()\"},\"version\":1},{\"value\":{\"Operator\":\"map(| item | (context.current_tick().0, item))\"},\"version\":1},{\"value\":{\"Operator\":\"for_each(| x | result_send.send(x).unwrap())\"},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1}],\"operator_tag\":[{\"value\":null,\"version\":0}],\"graph\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":3,\"version\":1},{\"idx\":5,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":1,\"version\":1},{\"idx\":9,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":4,\"version\":1},{\"idx\":5,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":2,\"version\":1},{\"idx\":10,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":7,\"version\":1},{\"idx\":8,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":6,\"version\":1},{\"idx\":7,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":5,\"version\":1},{\"idx\":11,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":9,\"version\":1},{\"idx\":3,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":10,\"version\":1},{\"idx\":4,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":11,\"version\":1},{\"idx\":6,\"version\":1}],\"version\":1}],\"ports\":[{\"value\":null,\"version\":0},{\"value\":[\"Elided\",{\"Int\":\"0\"}],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",{\"Int\":\"1\"}],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1}],\"node_loops\":[{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1}],\"loop_nodes\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":3,\"version\":1},{\"idx\":4,\"version\":1},{\"idx\":5,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":6,\"version\":1},{\"idx\":7,\"version\":1},{\"idx\":8,\"version\":1}],\"version\":1}],\"loop_parent\":[{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":{\"idx\":1,\"version\":1},\"version\":1}],\"root_loops\":[{\"idx\":1,\"version\":1}],\"loop_children\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":2,\"version\":1}],\"version\":1},{\"value\":[],\"version\":1}],\"node_subgraph\":[{\"value\":null,\"version\":0},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1},{\"value\":{\"idx\":4,\"version\":1},\"version\":1},{\"value\":{\"idx\":4,\"version\":1},\"version\":1},{\"value\":{\"idx\":4,\"version\":1},\"version\":1}],\"subgraph_nodes\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":1,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":2,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":3,\"version\":1},{\"idx\":4,\"version\":1},{\"idx\":5,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":6,\"version\":1},{\"idx\":7,\"version\":1},{\"idx\":8,\"version\":1}],\"version\":1}],\"subgraph_stratum\":[{\"value\":null,\"version\":0},{\"value\":0,\"version\":1},{\"value\":0,\"version\":1},{\"value\":1,\"version\":1},{\"value\":2,\"version\":1}],\"node_singleton_references\":[{\"value\":null,\"version\":0},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1}],\"node_varnames\":[{\"value\":null,\"version\":0},{\"value\":\"users\",\"version\":1},{\"value\":\"messages\",\"version\":1},{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":\"cp\",\"version\":1}],\"subgraph_laziness\":[{\"value\":null,\"version\":0}]}",
                    );
                    df.__assign_diagnostics("[]");
                    let (hoff_9v1_send, hoff_9v1_recv) = df
                        .make_edge::<
                            _,
                            ::dfir_rs::scheduled::handoff::VecHandoff<_>,
                        >("handoff GraphNodeId(9v1)");
                    let (hoff_10v1_send, hoff_10v1_recv) = df
                        .make_edge::<
                            _,
                            ::dfir_rs::scheduled::handoff::VecHandoff<_>,
                        >("handoff GraphNodeId(10v1)");
                    let (hoff_11v1_send, hoff_11v1_recv) = df
                        .make_edge::<
                            _,
                            ::dfir_rs::scheduled::handoff::VecHandoff<_>,
                        >("handoff GraphNodeId(11v1)");
                    let loop_1v1 = df.add_loop(None);
                    let loop_2v1 = df.add_loop(Some(loop_1v1));
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_1v1__source_iter__loc_dfir_rs_tests_surface_loop_rs_59_17_59_46<
                        T,
                    >(thunk: impl FnOnce() -> T) -> T {
                        thunk()
                    }
                    let mut sg_1v1_node_1v1_iter = {
                        #[inline(always)]
                        fn check_iter<
                            IntoIter: ::std::iter::IntoIterator<Item = Item>,
                            Item,
                        >(
                            into_iter: IntoIter,
                        ) -> impl ::std::iter::Iterator<Item = Item> {
                            ::std::iter::IntoIterator::into_iter(into_iter)
                        }
                        check_iter(["alice", "bob"])
                    };
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_2v1__source_stream__loc_dfir_rs_tests_surface_loop_rs_60_20_60_64<
                        T,
                    >(thunk: impl FnOnce() -> T) -> T {
                        thunk()
                    }
                    let mut sg_2v1_node_2v1_stream = {
                        #[inline(always)]
                        fn check_stream<
                            Stream: ::dfir_rs::futures::stream::Stream<Item = Item>
                                + ::std::marker::Unpin,
                            Item,
                        >(
                            stream: Stream,
                        ) -> impl ::dfir_rs::futures::stream::Stream<
                            Item = Item,
                        > + ::std::marker::Unpin {
                            stream
                        }
                        check_stream(iter_batches_stream(0..12, 3))
                    };
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_3v1__prefix__loc_dfir_rs_tests_surface_loop_rs_62_22_62_30<T>(
                        thunk: impl FnOnce() -> T,
                    ) -> T {
                        thunk()
                    }
                    #[allow(clippy::redundant_closure_call)]
                    let singleton_op_3v1 = df
                        .add_state(::std::cell::RefCell::new(::std::vec::Vec::new()));
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_4v1__batch__loc_dfir_rs_tests_surface_loop_rs_63_25_63_32<T>(
                        thunk: impl FnOnce() -> T,
                    ) -> T {
                        thunk()
                    }
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_5v1__cross_join__loc_dfir_rs_tests_surface_loop_rs_64_18_64_30<
                        T,
                    >(thunk: impl FnOnce() -> T) -> T {
                        thunk()
                    }
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_6v1__all_once__loc_dfir_rs_tests_surface_loop_rs_67_24_67_34<
                        T,
                    >(thunk: impl FnOnce() -> T) -> T {
                        thunk()
                    }
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_7v1__map__loc_dfir_rs_tests_surface_loop_rs_68_24_68_68<T>(
                        thunk: impl FnOnce() -> T,
                    ) -> T {
                        thunk()
                    }
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_8v1__for_each__loc_dfir_rs_tests_surface_loop_rs_69_24_69_66<
                        T,
                    >(thunk: impl FnOnce() -> T) -> T {
                        thunk()
                    }
                    let sgid_1v1 = df
                        .add_subgraph_full(
                            "Subgraph GraphSubgraphId(1v1)",
                            0,
                            (),
                            (hoff_9v1_send, ()),
                            false,
                            None,
                            move |context, (), (hoff_9v1_send, ())| {
                                let hoff_9v1_send = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                    v|
                                {
                                    hoff_9v1_send.give(Some(v));
                                });
                                let op_1v1 = sg_1v1_node_1v1_iter.by_ref();
                                let op_1v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_1v1__source_iter__loc_dfir_rs_tests_surface_loop_rs_59_17_59_46<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_1v1__source_iter__loc_dfir_rs_tests_surface_loop_rs_59_17_59_46(
                                        op_1v1,
                                    )
                                };
                                #[inline(always)]
                                fn pivot_run_sg_1v1<
                                    Pull: ::std::iter::Iterator<Item = Item>,
                                    Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    Item,
                                >(pull: Pull, push: Push) {
                                    ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                                }
                                pivot_run_sg_1v1(op_1v1, hoff_9v1_send);
                            },
                        );
                    let sgid_2v1 = df
                        .add_subgraph_full(
                            "Subgraph GraphSubgraphId(2v1)",
                            0,
                            (),
                            (hoff_10v1_send, ()),
                            false,
                            None,
                            move |context, (), (hoff_10v1_send, ())| {
                                let hoff_10v1_send = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                    v|
                                {
                                    hoff_10v1_send.give(Some(v));
                                });
                                let op_2v1 = std::iter::from_fn(|| {
                                    match ::dfir_rs::futures::stream::Stream::poll_next(
                                        ::std::pin::Pin::new(&mut sg_2v1_node_2v1_stream),
                                        &mut std::task::Context::from_waker(&context.waker()),
                                    ) {
                                        std::task::Poll::Ready(maybe) => maybe,
                                        std::task::Poll::Pending => None,
                                    }
                                });
                                let op_2v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_2v1__source_stream__loc_dfir_rs_tests_surface_loop_rs_60_20_60_64<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_2v1__source_stream__loc_dfir_rs_tests_surface_loop_rs_60_20_60_64(
                                        op_2v1,
                                    )
                                };
                                #[inline(always)]
                                fn pivot_run_sg_2v1<
                                    Pull: ::std::iter::Iterator<Item = Item>,
                                    Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    Item,
                                >(pull: Pull, push: Push) {
                                    ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                                }
                                pivot_run_sg_2v1(op_2v1, hoff_10v1_send);
                            },
                        );
                    let sgid_3v1 = df
                        .add_subgraph_full(
                            "Subgraph GraphSubgraphId(3v1)",
                            1,
                            (hoff_9v1_recv, (hoff_10v1_recv, ())),
                            (hoff_11v1_send, ()),
                            false,
                            Some(loop_1v1),
                            move |
                                context,
                                (hoff_9v1_recv, (hoff_10v1_recv, ())),
                                (hoff_11v1_send, ())|
                            {
                                let mut hoff_9v1_recv = hoff_9v1_recv.borrow_mut_swap();
                                let hoff_9v1_recv = hoff_9v1_recv.drain(..);
                                let mut hoff_10v1_recv = hoff_10v1_recv.borrow_mut_swap();
                                let hoff_10v1_recv = hoff_10v1_recv.drain(..);
                                let hoff_11v1_send = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                    v|
                                {
                                    hoff_11v1_send.give(Some(v));
                                });
                                let mut sg_3v1_node_3v1_vec = unsafe {
                                    context.state_ref_unchecked(singleton_op_3v1)
                                }
                                    .borrow_mut();
                                ::std::iter::Extend::extend(
                                    &mut *sg_3v1_node_3v1_vec,
                                    hoff_9v1_recv,
                                );
                                let op_3v1 = ::std::iter::IntoIterator::into_iter(
                                    ::std::clone::Clone::clone(&*sg_3v1_node_3v1_vec),
                                );
                                let op_3v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_3v1__prefix__loc_dfir_rs_tests_surface_loop_rs_62_22_62_30<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_3v1__prefix__loc_dfir_rs_tests_surface_loop_rs_62_22_62_30(
                                        op_3v1,
                                    )
                                };
                                let op_4v1 = {
                                    fn check_input<
                                        Iter: ::std::iter::Iterator<Item = Item>,
                                        Item,
                                    >(iter: Iter) -> impl ::std::iter::Iterator<Item = Item> {
                                        iter
                                    }
                                    check_input::<_, _>(hoff_10v1_recv)
                                };
                                let op_4v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_4v1__batch__loc_dfir_rs_tests_surface_loop_rs_63_25_63_32<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_4v1__batch__loc_dfir_rs_tests_surface_loop_rs_63_25_63_32(
                                        op_4v1,
                                    )
                                };
                                let op_3v1 = op_3v1.map(|a| ((), a));
                                let op_4v1 = op_4v1.map(|b| ((), b));
                                let mut sg_3v1_node_5v1_joindata_lhs_borrow = ::std::default::Default::default();
                                let mut sg_3v1_node_5v1_joindata_rhs_borrow = ::std::default::Default::default();
                                let op_5v1 = {
                                    #[inline(always)]
                                    fn check_inputs<'a, K, I1, V1, I2, V2>(
                                        lhs: I1,
                                        rhs: I2,
                                        lhs_state: &'a mut ::dfir_rs::compiled::pull::HalfSetJoinState<
                                            K,
                                            V1,
                                            V2,
                                        >,
                                        rhs_state: &'a mut ::dfir_rs::compiled::pull::HalfSetJoinState<
                                            K,
                                            V2,
                                            V1,
                                        >,
                                        is_new_tick: bool,
                                    ) -> impl 'a + Iterator<Item = (K, (V1, V2))>
                                    where
                                        K: Eq + std::hash::Hash + Clone,
                                        V1: Clone + ::std::cmp::Eq,
                                        V2: Clone + ::std::cmp::Eq,
                                        I1: 'a + Iterator<Item = (K, V1)>,
                                        I2: 'a + Iterator<Item = (K, V2)>,
                                    {
                                        op_5v1__cross_join__loc_dfir_rs_tests_surface_loop_rs_64_18_64_30(||
                                        ::dfir_rs::compiled::pull::symmetric_hash_join_into_iter(
                                            lhs,
                                            rhs,
                                            lhs_state,
                                            rhs_state,
                                            is_new_tick,
                                        ))
                                    }
                                    check_inputs(
                                        op_3v1,
                                        op_4v1,
                                        &mut sg_3v1_node_5v1_joindata_lhs_borrow,
                                        &mut sg_3v1_node_5v1_joindata_rhs_borrow,
                                        true,
                                    )
                                };
                                let op_5v1 = op_5v1.map(|((), (a, b))| (a, b));
                                let op_5v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_5v1__cross_join__loc_dfir_rs_tests_surface_loop_rs_64_18_64_30<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_5v1__cross_join__loc_dfir_rs_tests_surface_loop_rs_64_18_64_30(
                                        op_5v1,
                                    )
                                };
                                #[inline(always)]
                                fn pivot_run_sg_3v1<
                                    Pull: ::std::iter::Iterator<Item = Item>,
                                    Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    Item,
                                >(pull: Pull, push: Push) {
                                    ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                                }
                                pivot_run_sg_3v1(op_5v1, hoff_11v1_send);
                                context.allow_another_iteration();
                                context.allow_another_iteration();
                            },
                        );
                    let sgid_4v1 = df
                        .add_subgraph_full(
                            "Subgraph GraphSubgraphId(4v1)",
                            2,
                            (hoff_11v1_recv, ()),
                            (),
                            false,
                            Some(loop_2v1),
                            move |context, (hoff_11v1_recv, ()), ()| {
                                let mut hoff_11v1_recv = hoff_11v1_recv.borrow_mut_swap();
                                let hoff_11v1_recv = hoff_11v1_recv.drain(..);
                                let op_6v1 = {
                                    fn check_input<
                                        Iter: ::std::iter::Iterator<Item = Item>,
                                        Item,
                                    >(iter: Iter) -> impl ::std::iter::Iterator<Item = Item> {
                                        iter
                                    }
                                    check_input::<_, _>(hoff_11v1_recv)
                                };
                                let op_6v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_6v1__all_once__loc_dfir_rs_tests_surface_loop_rs_67_24_67_34<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_6v1__all_once__loc_dfir_rs_tests_surface_loop_rs_67_24_67_34(
                                        op_6v1,
                                    )
                                };
                                #[allow(
                                    clippy::map_clone,
                                    reason = "dfir has no explicit `cloned`/`copied` operator"
                                )]
                                let op_7v1 = op_6v1
                                    .map(|item| (context.current_tick().0, item));
                                let op_7v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_7v1__map__loc_dfir_rs_tests_surface_loop_rs_68_24_68_68<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_7v1__map__loc_dfir_rs_tests_surface_loop_rs_68_24_68_68(
                                        op_7v1,
                                    )
                                };
                                let op_8v1 = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                    x|
                                result_send.send(x).unwrap());
                                let op_8v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_8v1__for_each__loc_dfir_rs_tests_surface_loop_rs_69_24_69_66<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    >(
                                        input: Input,
                                    ) -> impl ::dfir_rs::pusherator::Pusherator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Push<
                                            Item,
                                            Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                        > ::dfir_rs::pusherator::Pusherator for Push<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn give(&mut self, item: Self::Item) {
                                                self.inner.give(item)
                                            }
                                        }
                                        Push { inner: input }
                                    }
                                    op_8v1__for_each__loc_dfir_rs_tests_surface_loop_rs_69_24_69_66(
                                        op_8v1,
                                    )
                                };
                                #[inline(always)]
                                fn pivot_run_sg_4v1<
                                    Pull: ::std::iter::Iterator<Item = Item>,
                                    Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    Item,
                                >(pull: Pull, push: Push) {
                                    ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                                }
                                pivot_run_sg_4v1(op_7v1, op_8v1);
                            },
                        );
                    df
                }
            }
        };
        {
            #[cfg(not(target_arch = "wasm32"))]
            {
                {
                    let mut settings = ::insta::Settings::clone_current();
                    settings._private_inner_mut().snapshot_suffix("graphvis_mermaid");
                    settings
                        .bind(|| {
                            ::insta::_macro_support::assert_snapshot(
                                    ::insta::_macro_support::AutoName.into(),
                                    #[allow(clippy::redundant_closure_call)]
                                    &(|v| ::alloc::__export::must_use({
                                        let res = ::alloc::fmt::format(format_args!("{0}", v));
                                        res
                                    }))(
                                        &df.meta_graph().unwrap().to_mermaid(&Default::default()),
                                    ),
                                    "F:\\Projects\\hydroflow\\dfir_rs",
                                    {
                                        fn f() {}
                                        fn type_name_of_val<T>(_: T) -> &'static str {
                                            std::any::type_name::<T>()
                                        }
                                        let mut name = type_name_of_val(f)
                                            .strip_suffix("::f")
                                            .unwrap_or("");
                                        while let Some(rest) = name.strip_suffix("::{{closure}}") {
                                            name = rest;
                                        }
                                        name
                                    },
                                    "surface_loop",
                                    "dfir_rs\\tests\\surface_loop.rs",
                                    73u32,
                                    "df.meta_graph().unwrap().to_mermaid(& Default :: default())",
                                )
                                .unwrap();
                        })
                };
                {
                    let mut settings = ::insta::Settings::clone_current();
                    settings._private_inner_mut().snapshot_suffix("graphvis_dot");
                    settings
                        .bind(|| {
                            ::insta::_macro_support::assert_snapshot(
                                    ::insta::_macro_support::AutoName.into(),
                                    #[allow(clippy::redundant_closure_call)]
                                    &(|v| ::alloc::__export::must_use({
                                        let res = ::alloc::fmt::format(format_args!("{0}", v));
                                        res
                                    }))(&df.meta_graph().unwrap().to_dot(&Default::default())),
                                    "F:\\Projects\\hydroflow\\dfir_rs",
                                    {
                                        fn f() {}
                                        fn type_name_of_val<T>(_: T) -> &'static str {
                                            std::any::type_name::<T>()
                                        }
                                        let mut name = type_name_of_val(f)
                                            .strip_suffix("::f")
                                            .unwrap_or("");
                                        while let Some(rest) = name.strip_suffix("::{{closure}}") {
                                            name = rest;
                                        }
                                        name
                                    },
                                    "surface_loop",
                                    "dfir_rs\\tests\\surface_loop.rs",
                                    73u32,
                                    "df.meta_graph().unwrap().to_dot(& Default :: default())",
                                )
                                .unwrap();
                        })
                };
            }
        };
        df.run_available();
        match (
            &&[
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
            ],
            &&*collect_ready::<Vec<_>, _>(&mut result_recv),
        ) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
}
extern crate test;
#[cfg(test)]
#[rustc_test_marker = "test_flo_repeat_n"]
#[doc(hidden)]
pub const test_flo_repeat_n: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_flo_repeat_n"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "dfir_rs\\tests\\surface_loop.rs",
        start_line: 108usize,
        start_col: 8usize,
        end_line: 108usize,
        end_col: 25usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_flo_repeat_n()),
    ),
};
pub fn test_flo_repeat_n() {
    {
        let subscriber = tracing_subscriber::FmtSubscriber::builder()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_test_writer()
            .finish();
        let _ = tracing::subscriber::set_global_default(subscriber);
    };
    {
        let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<_>();
        let mut df = {
            {
                #[allow(unused_qualifications)]
                {
                    use ::dfir_rs::{var_expr, var_args};
                    let mut df = ::dfir_rs::scheduled::graph::Dfir::new();
                    df.__assign_meta_graph(
                        "{\"nodes\":[{\"value\":null,\"version\":0},{\"value\":{\"Operator\":\"source_iter([\\\"alice\\\", \\\"bob\\\"])\"},\"version\":1},{\"value\":{\"Operator\":\"source_stream(iter_batches_stream(0 .. 9, 3))\"},\"version\":1},{\"value\":{\"Operator\":\"prefix()\"},\"version\":1},{\"value\":{\"Operator\":\"batch()\"},\"version\":1},{\"value\":{\"Operator\":\"cross_join()\"},\"version\":1},{\"value\":{\"Operator\":\"repeat_n(2)\"},\"version\":1},{\"value\":{\"Operator\":\"inspect(| x | println! (\\\"{:?} {}\\\", x, context.loop_iter_count()))\"},\"version\":1},{\"value\":{\"Operator\":\"for_each(| x | result_send.send(x).unwrap())\"},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1}],\"operator_tag\":[{\"value\":null,\"version\":0}],\"graph\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":3,\"version\":1},{\"idx\":5,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":1,\"version\":1},{\"idx\":9,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":4,\"version\":1},{\"idx\":5,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":2,\"version\":1},{\"idx\":10,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":7,\"version\":1},{\"idx\":8,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":6,\"version\":1},{\"idx\":7,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":5,\"version\":1},{\"idx\":11,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":9,\"version\":1},{\"idx\":3,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":10,\"version\":1},{\"idx\":4,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":11,\"version\":1},{\"idx\":6,\"version\":1}],\"version\":1}],\"ports\":[{\"value\":null,\"version\":0},{\"value\":[\"Elided\",{\"Int\":\"0\"}],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",{\"Int\":\"1\"}],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1}],\"node_loops\":[{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1}],\"loop_nodes\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":3,\"version\":1},{\"idx\":4,\"version\":1},{\"idx\":5,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":6,\"version\":1},{\"idx\":7,\"version\":1},{\"idx\":8,\"version\":1}],\"version\":1}],\"loop_parent\":[{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":{\"idx\":1,\"version\":1},\"version\":1}],\"root_loops\":[{\"idx\":1,\"version\":1}],\"loop_children\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":2,\"version\":1}],\"version\":1},{\"value\":[],\"version\":1}],\"node_subgraph\":[{\"value\":null,\"version\":0},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1},{\"value\":{\"idx\":4,\"version\":1},\"version\":1},{\"value\":{\"idx\":4,\"version\":1},\"version\":1},{\"value\":{\"idx\":4,\"version\":1},\"version\":1}],\"subgraph_nodes\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":1,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":2,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":3,\"version\":1},{\"idx\":4,\"version\":1},{\"idx\":5,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":6,\"version\":1},{\"idx\":7,\"version\":1},{\"idx\":8,\"version\":1}],\"version\":1}],\"subgraph_stratum\":[{\"value\":null,\"version\":0},{\"value\":0,\"version\":1},{\"value\":0,\"version\":1},{\"value\":1,\"version\":1},{\"value\":2,\"version\":1}],\"node_singleton_references\":[{\"value\":null,\"version\":0},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1}],\"node_varnames\":[{\"value\":null,\"version\":0},{\"value\":\"users\",\"version\":1},{\"value\":\"messages\",\"version\":1},{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":\"cp\",\"version\":1}],\"subgraph_laziness\":[{\"value\":null,\"version\":0}]}",
                    );
                    df.__assign_diagnostics("[]");
                    let (hoff_9v1_send, hoff_9v1_recv) = df
                        .make_edge::<
                            _,
                            ::dfir_rs::scheduled::handoff::VecHandoff<_>,
                        >("handoff GraphNodeId(9v1)");
                    let (hoff_10v1_send, hoff_10v1_recv) = df
                        .make_edge::<
                            _,
                            ::dfir_rs::scheduled::handoff::VecHandoff<_>,
                        >("handoff GraphNodeId(10v1)");
                    let (hoff_11v1_send, hoff_11v1_recv) = df
                        .make_edge::<
                            _,
                            ::dfir_rs::scheduled::handoff::VecHandoff<_>,
                        >("handoff GraphNodeId(11v1)");
                    let loop_1v1 = df.add_loop(None);
                    let loop_2v1 = df.add_loop(Some(loop_1v1));
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_1v1__source_iter__loc_dfir_rs_tests_surface_loop_rs_112_17_112_46<
                        T,
                    >(thunk: impl FnOnce() -> T) -> T {
                        thunk()
                    }
                    let mut sg_1v1_node_1v1_iter = {
                        #[inline(always)]
                        fn check_iter<
                            IntoIter: ::std::iter::IntoIterator<Item = Item>,
                            Item,
                        >(
                            into_iter: IntoIter,
                        ) -> impl ::std::iter::Iterator<Item = Item> {
                            ::std::iter::IntoIterator::into_iter(into_iter)
                        }
                        check_iter(["alice", "bob"])
                    };
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_2v1__source_stream__loc_dfir_rs_tests_surface_loop_rs_113_20_113_63<
                        T,
                    >(thunk: impl FnOnce() -> T) -> T {
                        thunk()
                    }
                    let mut sg_2v1_node_2v1_stream = {
                        #[inline(always)]
                        fn check_stream<
                            Stream: ::dfir_rs::futures::stream::Stream<Item = Item>
                                + ::std::marker::Unpin,
                            Item,
                        >(
                            stream: Stream,
                        ) -> impl ::dfir_rs::futures::stream::Stream<
                            Item = Item,
                        > + ::std::marker::Unpin {
                            stream
                        }
                        check_stream(iter_batches_stream(0..9, 3))
                    };
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_3v1__prefix__loc_dfir_rs_tests_surface_loop_rs_115_22_115_30<
                        T,
                    >(thunk: impl FnOnce() -> T) -> T {
                        thunk()
                    }
                    #[allow(clippy::redundant_closure_call)]
                    let singleton_op_3v1 = df
                        .add_state(::std::cell::RefCell::new(::std::vec::Vec::new()));
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_4v1__batch__loc_dfir_rs_tests_surface_loop_rs_116_25_116_32<T>(
                        thunk: impl FnOnce() -> T,
                    ) -> T {
                        thunk()
                    }
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_5v1__cross_join__loc_dfir_rs_tests_surface_loop_rs_117_18_117_30<
                        T,
                    >(thunk: impl FnOnce() -> T) -> T {
                        thunk()
                    }
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_6v1__repeat_n__loc_dfir_rs_tests_surface_loop_rs_119_23_119_34<
                        T,
                    >(thunk: impl FnOnce() -> T) -> T {
                        thunk()
                    }
                    #[allow(clippy::redundant_closure_call)]
                    let singleton_op_6v1 = df
                        .add_state(::std::cell::RefCell::new(::std::vec::Vec::new()));
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_7v1__inspect__loc_dfir_rs_tests_surface_loop_rs_120_24_120_86<
                        T,
                    >(thunk: impl FnOnce() -> T) -> T {
                        thunk()
                    }
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_8v1__for_each__loc_dfir_rs_tests_surface_loop_rs_121_24_121_66<
                        T,
                    >(thunk: impl FnOnce() -> T) -> T {
                        thunk()
                    }
                    let sgid_1v1 = df
                        .add_subgraph_full(
                            "Subgraph GraphSubgraphId(1v1)",
                            0,
                            (),
                            (hoff_9v1_send, ()),
                            false,
                            None,
                            move |context, (), (hoff_9v1_send, ())| {
                                let hoff_9v1_send = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                    v|
                                {
                                    hoff_9v1_send.give(Some(v));
                                });
                                let op_1v1 = sg_1v1_node_1v1_iter.by_ref();
                                let op_1v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_1v1__source_iter__loc_dfir_rs_tests_surface_loop_rs_112_17_112_46<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_1v1__source_iter__loc_dfir_rs_tests_surface_loop_rs_112_17_112_46(
                                        op_1v1,
                                    )
                                };
                                #[inline(always)]
                                fn pivot_run_sg_1v1<
                                    Pull: ::std::iter::Iterator<Item = Item>,
                                    Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    Item,
                                >(pull: Pull, push: Push) {
                                    ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                                }
                                pivot_run_sg_1v1(op_1v1, hoff_9v1_send);
                            },
                        );
                    let sgid_2v1 = df
                        .add_subgraph_full(
                            "Subgraph GraphSubgraphId(2v1)",
                            0,
                            (),
                            (hoff_10v1_send, ()),
                            false,
                            None,
                            move |context, (), (hoff_10v1_send, ())| {
                                let hoff_10v1_send = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                    v|
                                {
                                    hoff_10v1_send.give(Some(v));
                                });
                                let op_2v1 = std::iter::from_fn(|| {
                                    match ::dfir_rs::futures::stream::Stream::poll_next(
                                        ::std::pin::Pin::new(&mut sg_2v1_node_2v1_stream),
                                        &mut std::task::Context::from_waker(&context.waker()),
                                    ) {
                                        std::task::Poll::Ready(maybe) => maybe,
                                        std::task::Poll::Pending => None,
                                    }
                                });
                                let op_2v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_2v1__source_stream__loc_dfir_rs_tests_surface_loop_rs_113_20_113_63<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_2v1__source_stream__loc_dfir_rs_tests_surface_loop_rs_113_20_113_63(
                                        op_2v1,
                                    )
                                };
                                #[inline(always)]
                                fn pivot_run_sg_2v1<
                                    Pull: ::std::iter::Iterator<Item = Item>,
                                    Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    Item,
                                >(pull: Pull, push: Push) {
                                    ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                                }
                                pivot_run_sg_2v1(op_2v1, hoff_10v1_send);
                            },
                        );
                    let sgid_3v1 = df
                        .add_subgraph_full(
                            "Subgraph GraphSubgraphId(3v1)",
                            1,
                            (hoff_9v1_recv, (hoff_10v1_recv, ())),
                            (hoff_11v1_send, ()),
                            false,
                            Some(loop_1v1),
                            move |
                                context,
                                (hoff_9v1_recv, (hoff_10v1_recv, ())),
                                (hoff_11v1_send, ())|
                            {
                                let mut hoff_9v1_recv = hoff_9v1_recv.borrow_mut_swap();
                                let hoff_9v1_recv = hoff_9v1_recv.drain(..);
                                let mut hoff_10v1_recv = hoff_10v1_recv.borrow_mut_swap();
                                let hoff_10v1_recv = hoff_10v1_recv.drain(..);
                                let hoff_11v1_send = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                    v|
                                {
                                    hoff_11v1_send.give(Some(v));
                                });
                                let mut sg_3v1_node_3v1_vec = unsafe {
                                    context.state_ref_unchecked(singleton_op_3v1)
                                }
                                    .borrow_mut();
                                ::std::iter::Extend::extend(
                                    &mut *sg_3v1_node_3v1_vec,
                                    hoff_9v1_recv,
                                );
                                let op_3v1 = ::std::iter::IntoIterator::into_iter(
                                    ::std::clone::Clone::clone(&*sg_3v1_node_3v1_vec),
                                );
                                let op_3v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_3v1__prefix__loc_dfir_rs_tests_surface_loop_rs_115_22_115_30<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_3v1__prefix__loc_dfir_rs_tests_surface_loop_rs_115_22_115_30(
                                        op_3v1,
                                    )
                                };
                                let op_4v1 = {
                                    fn check_input<
                                        Iter: ::std::iter::Iterator<Item = Item>,
                                        Item,
                                    >(iter: Iter) -> impl ::std::iter::Iterator<Item = Item> {
                                        iter
                                    }
                                    check_input::<_, _>(hoff_10v1_recv)
                                };
                                let op_4v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_4v1__batch__loc_dfir_rs_tests_surface_loop_rs_116_25_116_32<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_4v1__batch__loc_dfir_rs_tests_surface_loop_rs_116_25_116_32(
                                        op_4v1,
                                    )
                                };
                                let op_3v1 = op_3v1.map(|a| ((), a));
                                let op_4v1 = op_4v1.map(|b| ((), b));
                                let mut sg_3v1_node_5v1_joindata_lhs_borrow = ::std::default::Default::default();
                                let mut sg_3v1_node_5v1_joindata_rhs_borrow = ::std::default::Default::default();
                                let op_5v1 = {
                                    #[inline(always)]
                                    fn check_inputs<'a, K, I1, V1, I2, V2>(
                                        lhs: I1,
                                        rhs: I2,
                                        lhs_state: &'a mut ::dfir_rs::compiled::pull::HalfSetJoinState<
                                            K,
                                            V1,
                                            V2,
                                        >,
                                        rhs_state: &'a mut ::dfir_rs::compiled::pull::HalfSetJoinState<
                                            K,
                                            V2,
                                            V1,
                                        >,
                                        is_new_tick: bool,
                                    ) -> impl 'a + Iterator<Item = (K, (V1, V2))>
                                    where
                                        K: Eq + std::hash::Hash + Clone,
                                        V1: Clone + ::std::cmp::Eq,
                                        V2: Clone + ::std::cmp::Eq,
                                        I1: 'a + Iterator<Item = (K, V1)>,
                                        I2: 'a + Iterator<Item = (K, V2)>,
                                    {
                                        op_5v1__cross_join__loc_dfir_rs_tests_surface_loop_rs_117_18_117_30(||
                                        ::dfir_rs::compiled::pull::symmetric_hash_join_into_iter(
                                            lhs,
                                            rhs,
                                            lhs_state,
                                            rhs_state,
                                            is_new_tick,
                                        ))
                                    }
                                    check_inputs(
                                        op_3v1,
                                        op_4v1,
                                        &mut sg_3v1_node_5v1_joindata_lhs_borrow,
                                        &mut sg_3v1_node_5v1_joindata_rhs_borrow,
                                        true,
                                    )
                                };
                                let op_5v1 = op_5v1.map(|((), (a, b))| (a, b));
                                let op_5v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_5v1__cross_join__loc_dfir_rs_tests_surface_loop_rs_117_18_117_30<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_5v1__cross_join__loc_dfir_rs_tests_surface_loop_rs_117_18_117_30(
                                        op_5v1,
                                    )
                                };
                                #[inline(always)]
                                fn pivot_run_sg_3v1<
                                    Pull: ::std::iter::Iterator<Item = Item>,
                                    Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    Item,
                                >(pull: Pull, push: Push) {
                                    ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                                }
                                pivot_run_sg_3v1(op_5v1, hoff_11v1_send);
                                context.allow_another_iteration();
                                context.allow_another_iteration();
                            },
                        );
                    let sgid_4v1 = df
                        .add_subgraph_full(
                            "Subgraph GraphSubgraphId(4v1)",
                            2,
                            (hoff_11v1_recv, ()),
                            (),
                            false,
                            Some(loop_2v1),
                            move |context, (hoff_11v1_recv, ()), ()| {
                                let mut hoff_11v1_recv = hoff_11v1_recv.borrow_mut_swap();
                                let hoff_11v1_recv = hoff_11v1_recv.drain(..);
                                let mut sg_4v1_node_6v1_vec = unsafe {
                                    context.state_ref_unchecked(singleton_op_6v1)
                                }
                                    .borrow_mut();
                                if 0 == context.loop_iter_count() {
                                    *sg_4v1_node_6v1_vec = hoff_11v1_recv
                                        .collect::<::std::vec::Vec<_>>();
                                }
                                let op_6v1 = std::iter::IntoIterator::into_iter(
                                    ::std::clone::Clone::clone(&*sg_4v1_node_6v1_vec),
                                );
                                let op_6v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_6v1__repeat_n__loc_dfir_rs_tests_surface_loop_rs_119_23_119_34<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_6v1__repeat_n__loc_dfir_rs_tests_surface_loop_rs_119_23_119_34(
                                        op_6v1,
                                    )
                                };
                                let op_7v1 = op_6v1
                                    .inspect(|x| {
                                        ::std::io::_print(
                                            format_args!("{0:?} {1}\n", x, context.loop_iter_count()),
                                        );
                                    });
                                let op_7v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_7v1__inspect__loc_dfir_rs_tests_surface_loop_rs_120_24_120_86<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_7v1__inspect__loc_dfir_rs_tests_surface_loop_rs_120_24_120_86(
                                        op_7v1,
                                    )
                                };
                                let op_8v1 = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                    x|
                                result_send.send(x).unwrap());
                                let op_8v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_8v1__for_each__loc_dfir_rs_tests_surface_loop_rs_121_24_121_66<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    >(
                                        input: Input,
                                    ) -> impl ::dfir_rs::pusherator::Pusherator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Push<
                                            Item,
                                            Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                        > ::dfir_rs::pusherator::Pusherator for Push<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn give(&mut self, item: Self::Item) {
                                                self.inner.give(item)
                                            }
                                        }
                                        Push { inner: input }
                                    }
                                    op_8v1__for_each__loc_dfir_rs_tests_surface_loop_rs_121_24_121_66(
                                        op_8v1,
                                    )
                                };
                                #[inline(always)]
                                fn pivot_run_sg_4v1<
                                    Pull: ::std::iter::Iterator<Item = Item>,
                                    Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    Item,
                                >(pull: Pull, push: Push) {
                                    ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                                }
                                pivot_run_sg_4v1(op_7v1, op_8v1);
                                {
                                    if context.loop_iter_count() + 1 < 2 {
                                        context.reschedule_loop_block();
                                    }
                                }
                            },
                        );
                    df
                }
            }
        };
        {
            #[cfg(not(target_arch = "wasm32"))]
            {
                {
                    let mut settings = ::insta::Settings::clone_current();
                    settings._private_inner_mut().snapshot_suffix("graphvis_mermaid");
                    settings
                        .bind(|| {
                            ::insta::_macro_support::assert_snapshot(
                                    ::insta::_macro_support::AutoName.into(),
                                    #[allow(clippy::redundant_closure_call)]
                                    &(|v| ::alloc::__export::must_use({
                                        let res = ::alloc::fmt::format(format_args!("{0}", v));
                                        res
                                    }))(
                                        &df.meta_graph().unwrap().to_mermaid(&Default::default()),
                                    ),
                                    "F:\\Projects\\hydroflow\\dfir_rs",
                                    {
                                        fn f() {}
                                        fn type_name_of_val<T>(_: T) -> &'static str {
                                            std::any::type_name::<T>()
                                        }
                                        let mut name = type_name_of_val(f)
                                            .strip_suffix("::f")
                                            .unwrap_or("");
                                        while let Some(rest) = name.strip_suffix("::{{closure}}") {
                                            name = rest;
                                        }
                                        name
                                    },
                                    "surface_loop",
                                    "dfir_rs\\tests\\surface_loop.rs",
                                    125u32,
                                    "df.meta_graph().unwrap().to_mermaid(& Default :: default())",
                                )
                                .unwrap();
                        })
                };
                {
                    let mut settings = ::insta::Settings::clone_current();
                    settings._private_inner_mut().snapshot_suffix("graphvis_dot");
                    settings
                        .bind(|| {
                            ::insta::_macro_support::assert_snapshot(
                                    ::insta::_macro_support::AutoName.into(),
                                    #[allow(clippy::redundant_closure_call)]
                                    &(|v| ::alloc::__export::must_use({
                                        let res = ::alloc::fmt::format(format_args!("{0}", v));
                                        res
                                    }))(&df.meta_graph().unwrap().to_dot(&Default::default())),
                                    "F:\\Projects\\hydroflow\\dfir_rs",
                                    {
                                        fn f() {}
                                        fn type_name_of_val<T>(_: T) -> &'static str {
                                            std::any::type_name::<T>()
                                        }
                                        let mut name = type_name_of_val(f)
                                            .strip_suffix("::f")
                                            .unwrap_or("");
                                        while let Some(rest) = name.strip_suffix("::{{closure}}") {
                                            name = rest;
                                        }
                                        name
                                    },
                                    "surface_loop",
                                    "dfir_rs\\tests\\surface_loop.rs",
                                    125u32,
                                    "df.meta_graph().unwrap().to_dot(& Default :: default())",
                                )
                                .unwrap();
                        })
                };
            }
        };
        df.run_available();
        match (
            &&[
                ("alice", 0),
                ("alice", 1),
                ("alice", 2),
                ("bob", 0),
                ("bob", 1),
                ("bob", 2),
                ("alice", 0),
                ("alice", 1),
                ("alice", 2),
                ("bob", 0),
                ("bob", 1),
                ("bob", 2),
                ("alice", 3),
                ("alice", 4),
                ("alice", 5),
                ("bob", 3),
                ("bob", 4),
                ("bob", 5),
                ("alice", 3),
                ("alice", 4),
                ("alice", 5),
                ("bob", 3),
                ("bob", 4),
                ("bob", 5),
                ("alice", 6),
                ("alice", 7),
                ("alice", 8),
                ("bob", 6),
                ("bob", 7),
                ("bob", 8),
                ("alice", 6),
                ("alice", 7),
                ("alice", 8),
                ("bob", 6),
                ("bob", 7),
                ("bob", 8),
            ],
            &&*collect_ready::<Vec<_>, _>(&mut result_recv),
        ) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
}
extern crate test;
#[cfg(test)]
#[rustc_test_marker = "test_flo_repeat_n_nested"]
#[doc(hidden)]
pub const test_flo_repeat_n_nested: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_flo_repeat_n_nested"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "dfir_rs\\tests\\surface_loop.rs",
        start_line: 172usize,
        start_col: 8usize,
        end_line: 172usize,
        end_col: 32usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_flo_repeat_n_nested()),
    ),
};
const _: () = {};
pub fn test_flo_repeat_n_nested() {
    {
        let subscriber = tracing_subscriber::FmtSubscriber::builder()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_test_writer()
            .finish();
        let _ = tracing::subscriber::set_global_default(subscriber);
    };
    {
        let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<_>();
        let mut df = {
            {
                #[allow(unused_qualifications)]
                {
                    use ::dfir_rs::{var_expr, var_args};
                    let mut df = ::dfir_rs::scheduled::graph::Dfir::new();
                    df.__assign_meta_graph(
                        "{\"nodes\":[{\"value\":null,\"version\":0},{\"value\":{\"Operator\":\"source_iter([\\\"alice\\\", \\\"bob\\\"])\"},\"version\":1},{\"value\":{\"Operator\":\"batch()\"},\"version\":1},{\"value\":{\"Operator\":\"repeat_n(3)\"},\"version\":1},{\"value\":{\"Operator\":\"inspect(| x | println! (\\\"A {:?} {}\\\", x, context.loop_iter_count()))\"},\"version\":1},{\"value\":{\"Operator\":\"repeat_n(3)\"},\"version\":1},{\"value\":{\"Operator\":\"inspect(| x | println! (\\\"B {:?} {}\\\", x, context.loop_iter_count()))\"},\"version\":1},{\"value\":{\"Operator\":\"for_each(| x | result_send.send(x).unwrap())\"},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1}],\"operator_tag\":[{\"value\":null,\"version\":0}],\"graph\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":1,\"version\":1},{\"idx\":8,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":3,\"version\":1},{\"idx\":9,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":2,\"version\":1},{\"idx\":10,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":6,\"version\":1},{\"idx\":7,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":5,\"version\":1},{\"idx\":6,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":4,\"version\":1},{\"idx\":11,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":8,\"version\":1},{\"idx\":2,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":9,\"version\":1},{\"idx\":4,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":10,\"version\":1},{\"idx\":3,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":11,\"version\":1},{\"idx\":5,\"version\":1}],\"version\":1}],\"ports\":[{\"value\":null,\"version\":0},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1}],\"node_loops\":[{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1}],\"loop_nodes\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":2,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":3,\"version\":1},{\"idx\":4,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":5,\"version\":1},{\"idx\":6,\"version\":1},{\"idx\":7,\"version\":1}],\"version\":1}],\"loop_parent\":[{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1}],\"root_loops\":[{\"idx\":1,\"version\":1}],\"loop_children\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":2,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":3,\"version\":1}],\"version\":1},{\"value\":[],\"version\":1}],\"node_subgraph\":[{\"value\":null,\"version\":0},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1},{\"value\":{\"idx\":4,\"version\":1},\"version\":1},{\"value\":{\"idx\":5,\"version\":1},\"version\":1},{\"value\":{\"idx\":5,\"version\":1},\"version\":1},{\"value\":{\"idx\":5,\"version\":1},\"version\":1}],\"subgraph_nodes\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":1,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":2,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":3,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":4,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":5,\"version\":1},{\"idx\":6,\"version\":1},{\"idx\":7,\"version\":1}],\"version\":1}],\"subgraph_stratum\":[{\"value\":null,\"version\":0},{\"value\":0,\"version\":1},{\"value\":1,\"version\":1},{\"value\":2,\"version\":1},{\"value\":2,\"version\":1},{\"value\":3,\"version\":1}],\"node_singleton_references\":[{\"value\":null,\"version\":0},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1}],\"node_varnames\":[{\"value\":null,\"version\":0},{\"value\":\"usrs1\",\"version\":1},{\"value\":\"usrs2\",\"version\":1},{\"value\":\"usrs3\",\"version\":1},{\"value\":\"usrs3\",\"version\":1}],\"subgraph_laziness\":[{\"value\":null,\"version\":0}]}",
                    );
                    df.__assign_diagnostics("[]");
                    let (hoff_8v1_send, hoff_8v1_recv) = df
                        .make_edge::<
                            _,
                            ::dfir_rs::scheduled::handoff::VecHandoff<_>,
                        >("handoff GraphNodeId(8v1)");
                    let (hoff_9v1_send, hoff_9v1_recv) = df
                        .make_edge::<
                            _,
                            ::dfir_rs::scheduled::handoff::VecHandoff<_>,
                        >("handoff GraphNodeId(9v1)");
                    let (hoff_10v1_send, hoff_10v1_recv) = df
                        .make_edge::<
                            _,
                            ::dfir_rs::scheduled::handoff::VecHandoff<_>,
                        >("handoff GraphNodeId(10v1)");
                    let (hoff_11v1_send, hoff_11v1_recv) = df
                        .make_edge::<
                            _,
                            ::dfir_rs::scheduled::handoff::VecHandoff<_>,
                        >("handoff GraphNodeId(11v1)");
                    let loop_1v1 = df.add_loop(None);
                    let loop_2v1 = df.add_loop(Some(loop_1v1));
                    let loop_3v1 = df.add_loop(Some(loop_2v1));
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_1v1__source_iter__loc_dfir_rs_tests_surface_loop_rs_176_17_176_46<
                        T,
                    >(thunk: impl FnOnce() -> T) -> T {
                        thunk()
                    }
                    let mut sg_1v1_node_1v1_iter = {
                        #[inline(always)]
                        fn check_iter<
                            IntoIter: ::std::iter::IntoIterator<Item = Item>,
                            Item,
                        >(
                            into_iter: IntoIter,
                        ) -> impl ::std::iter::Iterator<Item = Item> {
                            ::std::iter::IntoIterator::into_iter(into_iter)
                        }
                        check_iter(["alice", "bob"])
                    };
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_2v1__batch__loc_dfir_rs_tests_surface_loop_rs_178_30_178_37<T>(
                        thunk: impl FnOnce() -> T,
                    ) -> T {
                        thunk()
                    }
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_3v1__repeat_n__loc_dfir_rs_tests_surface_loop_rs_180_34_180_45<
                        T,
                    >(thunk: impl FnOnce() -> T) -> T {
                        thunk()
                    }
                    #[allow(clippy::redundant_closure_call)]
                    let singleton_op_3v1 = df
                        .add_state(::std::cell::RefCell::new(::std::vec::Vec::new()));
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_4v1__inspect__loc_dfir_rs_tests_surface_loop_rs_180_49_180_113<
                        T,
                    >(thunk: impl FnOnce() -> T) -> T {
                        thunk()
                    }
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_5v1__repeat_n__loc_dfir_rs_tests_surface_loop_rs_182_30_182_41<
                        T,
                    >(thunk: impl FnOnce() -> T) -> T {
                        thunk()
                    }
                    #[allow(clippy::redundant_closure_call)]
                    let singleton_op_5v1 = df
                        .add_state(::std::cell::RefCell::new(::std::vec::Vec::new()));
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_6v1__inspect__loc_dfir_rs_tests_surface_loop_rs_183_28_183_92<
                        T,
                    >(thunk: impl FnOnce() -> T) -> T {
                        thunk()
                    }
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_7v1__for_each__loc_dfir_rs_tests_surface_loop_rs_184_28_184_70<
                        T,
                    >(thunk: impl FnOnce() -> T) -> T {
                        thunk()
                    }
                    let sgid_1v1 = df
                        .add_subgraph_full(
                            "Subgraph GraphSubgraphId(1v1)",
                            0,
                            (),
                            (hoff_8v1_send, ()),
                            false,
                            None,
                            move |context, (), (hoff_8v1_send, ())| {
                                let hoff_8v1_send = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                    v|
                                {
                                    hoff_8v1_send.give(Some(v));
                                });
                                let op_1v1 = sg_1v1_node_1v1_iter.by_ref();
                                let op_1v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_1v1__source_iter__loc_dfir_rs_tests_surface_loop_rs_176_17_176_46<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_1v1__source_iter__loc_dfir_rs_tests_surface_loop_rs_176_17_176_46(
                                        op_1v1,
                                    )
                                };
                                #[inline(always)]
                                fn pivot_run_sg_1v1<
                                    Pull: ::std::iter::Iterator<Item = Item>,
                                    Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    Item,
                                >(pull: Pull, push: Push) {
                                    ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                                }
                                pivot_run_sg_1v1(op_1v1, hoff_8v1_send);
                            },
                        );
                    let sgid_2v1 = df
                        .add_subgraph_full(
                            "Subgraph GraphSubgraphId(2v1)",
                            1,
                            (hoff_8v1_recv, ()),
                            (hoff_10v1_send, ()),
                            false,
                            Some(loop_1v1),
                            move |context, (hoff_8v1_recv, ()), (hoff_10v1_send, ())| {
                                let mut hoff_8v1_recv = hoff_8v1_recv.borrow_mut_swap();
                                let hoff_8v1_recv = hoff_8v1_recv.drain(..);
                                let hoff_10v1_send = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                    v|
                                {
                                    hoff_10v1_send.give(Some(v));
                                });
                                let op_2v1 = {
                                    fn check_input<
                                        Iter: ::std::iter::Iterator<Item = Item>,
                                        Item,
                                    >(iter: Iter) -> impl ::std::iter::Iterator<Item = Item> {
                                        iter
                                    }
                                    check_input::<_, _>(hoff_8v1_recv)
                                };
                                let op_2v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_2v1__batch__loc_dfir_rs_tests_surface_loop_rs_178_30_178_37<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_2v1__batch__loc_dfir_rs_tests_surface_loop_rs_178_30_178_37(
                                        op_2v1,
                                    )
                                };
                                #[inline(always)]
                                fn pivot_run_sg_2v1<
                                    Pull: ::std::iter::Iterator<Item = Item>,
                                    Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    Item,
                                >(pull: Pull, push: Push) {
                                    ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                                }
                                pivot_run_sg_2v1(op_2v1, hoff_10v1_send);
                                context.allow_another_iteration();
                            },
                        );
                    let sgid_3v1 = df
                        .add_subgraph_full(
                            "Subgraph GraphSubgraphId(3v1)",
                            2,
                            (hoff_10v1_recv, ()),
                            (hoff_9v1_send, ()),
                            false,
                            Some(loop_2v1),
                            move |context, (hoff_10v1_recv, ()), (hoff_9v1_send, ())| {
                                let mut hoff_10v1_recv = hoff_10v1_recv.borrow_mut_swap();
                                let hoff_10v1_recv = hoff_10v1_recv.drain(..);
                                let hoff_9v1_send = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                    v|
                                {
                                    hoff_9v1_send.give(Some(v));
                                });
                                let mut sg_3v1_node_3v1_vec = unsafe {
                                    context.state_ref_unchecked(singleton_op_3v1)
                                }
                                    .borrow_mut();
                                if 0 == context.loop_iter_count() {
                                    *sg_3v1_node_3v1_vec = hoff_10v1_recv
                                        .collect::<::std::vec::Vec<_>>();
                                }
                                let op_3v1 = std::iter::IntoIterator::into_iter(
                                    ::std::clone::Clone::clone(&*sg_3v1_node_3v1_vec),
                                );
                                let op_3v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_3v1__repeat_n__loc_dfir_rs_tests_surface_loop_rs_180_34_180_45<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_3v1__repeat_n__loc_dfir_rs_tests_surface_loop_rs_180_34_180_45(
                                        op_3v1,
                                    )
                                };
                                #[inline(always)]
                                fn pivot_run_sg_3v1<
                                    Pull: ::std::iter::Iterator<Item = Item>,
                                    Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    Item,
                                >(pull: Pull, push: Push) {
                                    ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                                }
                                pivot_run_sg_3v1(op_3v1, hoff_9v1_send);
                                {
                                    if context.loop_iter_count() + 1 < 3 {
                                        context.reschedule_loop_block();
                                    }
                                }
                            },
                        );
                    let sgid_4v1 = df
                        .add_subgraph_full(
                            "Subgraph GraphSubgraphId(4v1)",
                            2,
                            (hoff_9v1_recv, ()),
                            (hoff_11v1_send, ()),
                            false,
                            Some(loop_2v1),
                            move |context, (hoff_9v1_recv, ()), (hoff_11v1_send, ())| {
                                let mut hoff_9v1_recv = hoff_9v1_recv.borrow_mut_swap();
                                let hoff_9v1_recv = hoff_9v1_recv.drain(..);
                                let hoff_11v1_send = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                    v|
                                {
                                    hoff_11v1_send.give(Some(v));
                                });
                                let op_4v1 = hoff_9v1_recv
                                    .inspect(|x| {
                                        ::std::io::_print(
                                            format_args!("A {0:?} {1}\n", x, context.loop_iter_count()),
                                        );
                                    });
                                let op_4v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_4v1__inspect__loc_dfir_rs_tests_surface_loop_rs_180_49_180_113<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_4v1__inspect__loc_dfir_rs_tests_surface_loop_rs_180_49_180_113(
                                        op_4v1,
                                    )
                                };
                                #[inline(always)]
                                fn pivot_run_sg_4v1<
                                    Pull: ::std::iter::Iterator<Item = Item>,
                                    Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    Item,
                                >(pull: Pull, push: Push) {
                                    ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                                }
                                pivot_run_sg_4v1(op_4v1, hoff_11v1_send);
                            },
                        );
                    let sgid_5v1 = df
                        .add_subgraph_full(
                            "Subgraph GraphSubgraphId(5v1)",
                            3,
                            (hoff_11v1_recv, ()),
                            (),
                            false,
                            Some(loop_3v1),
                            move |context, (hoff_11v1_recv, ()), ()| {
                                let mut hoff_11v1_recv = hoff_11v1_recv.borrow_mut_swap();
                                let hoff_11v1_recv = hoff_11v1_recv.drain(..);
                                let mut sg_5v1_node_5v1_vec = unsafe {
                                    context.state_ref_unchecked(singleton_op_5v1)
                                }
                                    .borrow_mut();
                                if 0 == context.loop_iter_count() {
                                    *sg_5v1_node_5v1_vec = hoff_11v1_recv
                                        .collect::<::std::vec::Vec<_>>();
                                }
                                let op_5v1 = std::iter::IntoIterator::into_iter(
                                    ::std::clone::Clone::clone(&*sg_5v1_node_5v1_vec),
                                );
                                let op_5v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_5v1__repeat_n__loc_dfir_rs_tests_surface_loop_rs_182_30_182_41<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_5v1__repeat_n__loc_dfir_rs_tests_surface_loop_rs_182_30_182_41(
                                        op_5v1,
                                    )
                                };
                                let op_6v1 = op_5v1
                                    .inspect(|x| {
                                        ::std::io::_print(
                                            format_args!("B {0:?} {1}\n", x, context.loop_iter_count()),
                                        );
                                    });
                                let op_6v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_6v1__inspect__loc_dfir_rs_tests_surface_loop_rs_183_28_183_92<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_6v1__inspect__loc_dfir_rs_tests_surface_loop_rs_183_28_183_92(
                                        op_6v1,
                                    )
                                };
                                let op_7v1 = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                    x|
                                result_send.send(x).unwrap());
                                let op_7v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_7v1__for_each__loc_dfir_rs_tests_surface_loop_rs_184_28_184_70<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    >(
                                        input: Input,
                                    ) -> impl ::dfir_rs::pusherator::Pusherator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Push<
                                            Item,
                                            Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                        > ::dfir_rs::pusherator::Pusherator for Push<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn give(&mut self, item: Self::Item) {
                                                self.inner.give(item)
                                            }
                                        }
                                        Push { inner: input }
                                    }
                                    op_7v1__for_each__loc_dfir_rs_tests_surface_loop_rs_184_28_184_70(
                                        op_7v1,
                                    )
                                };
                                #[inline(always)]
                                fn pivot_run_sg_5v1<
                                    Pull: ::std::iter::Iterator<Item = Item>,
                                    Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    Item,
                                >(pull: Pull, push: Push) {
                                    ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                                }
                                pivot_run_sg_5v1(op_6v1, op_7v1);
                                {
                                    if context.loop_iter_count() + 1 < 3 {
                                        context.reschedule_loop_block();
                                    }
                                }
                            },
                        );
                    df
                }
            }
        };
        {
            #[cfg(not(target_arch = "wasm32"))]
            {
                {
                    let mut settings = ::insta::Settings::clone_current();
                    settings._private_inner_mut().snapshot_suffix("graphvis_mermaid");
                    settings
                        .bind(|| {
                            ::insta::_macro_support::assert_snapshot(
                                    ::insta::_macro_support::AutoName.into(),
                                    #[allow(clippy::redundant_closure_call)]
                                    &(|v| ::alloc::__export::must_use({
                                        let res = ::alloc::fmt::format(format_args!("{0}", v));
                                        res
                                    }))(
                                        &df.meta_graph().unwrap().to_mermaid(&Default::default()),
                                    ),
                                    "F:\\Projects\\hydroflow\\dfir_rs",
                                    {
                                        fn f() {}
                                        fn type_name_of_val<T>(_: T) -> &'static str {
                                            std::any::type_name::<T>()
                                        }
                                        let mut name = type_name_of_val(f)
                                            .strip_suffix("::f")
                                            .unwrap_or("");
                                        while let Some(rest) = name.strip_suffix("::{{closure}}") {
                                            name = rest;
                                        }
                                        name
                                    },
                                    "surface_loop",
                                    "dfir_rs\\tests\\surface_loop.rs",
                                    189u32,
                                    "df.meta_graph().unwrap().to_mermaid(& Default :: default())",
                                )
                                .unwrap();
                        })
                };
                {
                    let mut settings = ::insta::Settings::clone_current();
                    settings._private_inner_mut().snapshot_suffix("graphvis_dot");
                    settings
                        .bind(|| {
                            ::insta::_macro_support::assert_snapshot(
                                    ::insta::_macro_support::AutoName.into(),
                                    #[allow(clippy::redundant_closure_call)]
                                    &(|v| ::alloc::__export::must_use({
                                        let res = ::alloc::fmt::format(format_args!("{0}", v));
                                        res
                                    }))(&df.meta_graph().unwrap().to_dot(&Default::default())),
                                    "F:\\Projects\\hydroflow\\dfir_rs",
                                    {
                                        fn f() {}
                                        fn type_name_of_val<T>(_: T) -> &'static str {
                                            std::any::type_name::<T>()
                                        }
                                        let mut name = type_name_of_val(f)
                                            .strip_suffix("::f")
                                            .unwrap_or("");
                                        while let Some(rest) = name.strip_suffix("::{{closure}}") {
                                            name = rest;
                                        }
                                        name
                                    },
                                    "surface_loop",
                                    "dfir_rs\\tests\\surface_loop.rs",
                                    189u32,
                                    "df.meta_graph().unwrap().to_dot(& Default :: default())",
                                )
                                .unwrap();
                        })
                };
            }
        };
        df.run_available();
        match (
            &&[
                "alice",
                "bob",
                "alice",
                "bob",
                "alice",
                "bob",
                "alice",
                "bob",
                "alice",
                "bob",
                "alice",
                "bob",
                "alice",
                "bob",
                "alice",
                "bob",
                "alice",
                "bob",
            ],
            &&*collect_ready::<Vec<_>, _>(&mut result_recv),
        ) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
}
extern crate test;
#[cfg(test)]
#[rustc_test_marker = "test_flo_repeat_n_multiple_nested"]
#[doc(hidden)]
pub const test_flo_repeat_n_multiple_nested: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_flo_repeat_n_multiple_nested"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "dfir_rs\\tests\\surface_loop.rs",
        start_line: 202usize,
        start_col: 8usize,
        end_line: 202usize,
        end_col: 41usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_flo_repeat_n_multiple_nested()),
    ),
};
const _: () = {};
pub fn test_flo_repeat_n_multiple_nested() {
    let (result1_send, mut result1_recv) = dfir_rs::util::unbounded_channel::<_>();
    let (result2_send, mut result2_recv) = dfir_rs::util::unbounded_channel::<_>();
    let mut df = {
        {
            #[allow(unused_qualifications)]
            {
                use ::dfir_rs::{var_expr, var_args};
                let mut df = ::dfir_rs::scheduled::graph::Dfir::new();
                df.__assign_meta_graph(
                    "{\"nodes\":[{\"value\":null,\"version\":0},{\"value\":{\"Operator\":\"source_iter([\\\"alice\\\", \\\"bob\\\"])\"},\"version\":1},{\"value\":{\"Operator\":\"batch()\"},\"version\":1},{\"value\":{\"Operator\":\"repeat_n(3)\"},\"version\":1},{\"value\":{\"Operator\":\"inspect(| x | println! (\\\"{:?} {}\\\", x, context.loop_iter_count()))\"},\"version\":1},{\"value\":{\"Operator\":\"tee()\"},\"version\":1},{\"value\":{\"Operator\":\"repeat_n(3)\"},\"version\":1},{\"value\":{\"Operator\":\"inspect(| x | println! (\\\"{} {:?} {}\\\", line! (), x, context.loop_iter_count()))\"},\"version\":1},{\"value\":{\"Operator\":\"for_each(| x | result1_send.send(x).unwrap())\"},\"version\":1},{\"value\":{\"Operator\":\"repeat_n(3)\"},\"version\":1},{\"value\":{\"Operator\":\"inspect(| x | println! (\\\"{} {:?} {}\\\", line! (), x, context.loop_iter_count()))\"},\"version\":1},{\"value\":{\"Operator\":\"for_each(| x | result2_send.send(x).unwrap())\"},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1}],\"operator_tag\":[{\"value\":null,\"version\":0}],\"graph\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":1,\"version\":1},{\"idx\":12,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":4,\"version\":1},{\"idx\":5,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":3,\"version\":1},{\"idx\":4,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":2,\"version\":1},{\"idx\":13,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":7,\"version\":1},{\"idx\":8,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":6,\"version\":1},{\"idx\":7,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":5,\"version\":1},{\"idx\":14,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":10,\"version\":1},{\"idx\":11,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":9,\"version\":1},{\"idx\":10,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":5,\"version\":1},{\"idx\":15,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":12,\"version\":1},{\"idx\":2,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":13,\"version\":1},{\"idx\":3,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":14,\"version\":1},{\"idx\":6,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":15,\"version\":1},{\"idx\":9,\"version\":1}],\"version\":1}],\"ports\":[{\"value\":null,\"version\":0},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1}],\"node_loops\":[{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1},{\"value\":{\"idx\":4,\"version\":1},\"version\":1},{\"value\":{\"idx\":4,\"version\":1},\"version\":1},{\"value\":{\"idx\":4,\"version\":1},\"version\":1}],\"loop_nodes\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":2,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":3,\"version\":1},{\"idx\":4,\"version\":1},{\"idx\":5,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":6,\"version\":1},{\"idx\":7,\"version\":1},{\"idx\":8,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":9,\"version\":1},{\"idx\":10,\"version\":1},{\"idx\":11,\"version\":1}],\"version\":1}],\"loop_parent\":[{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1}],\"root_loops\":[{\"idx\":1,\"version\":1}],\"loop_children\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":2,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":3,\"version\":1},{\"idx\":4,\"version\":1}],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1}],\"node_subgraph\":[{\"value\":null,\"version\":0},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1},{\"value\":{\"idx\":4,\"version\":1},\"version\":1},{\"value\":{\"idx\":4,\"version\":1},\"version\":1},{\"value\":{\"idx\":4,\"version\":1},\"version\":1},{\"value\":{\"idx\":5,\"version\":1},\"version\":1},{\"value\":{\"idx\":5,\"version\":1},\"version\":1},{\"value\":{\"idx\":5,\"version\":1},\"version\":1}],\"subgraph_nodes\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":1,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":2,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":3,\"version\":1},{\"idx\":4,\"version\":1},{\"idx\":5,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":6,\"version\":1},{\"idx\":7,\"version\":1},{\"idx\":8,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":9,\"version\":1},{\"idx\":10,\"version\":1},{\"idx\":11,\"version\":1}],\"version\":1}],\"subgraph_stratum\":[{\"value\":null,\"version\":0},{\"value\":0,\"version\":1},{\"value\":1,\"version\":1},{\"value\":2,\"version\":1},{\"value\":3,\"version\":1},{\"value\":3,\"version\":1}],\"node_singleton_references\":[{\"value\":null,\"version\":0},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1}],\"node_varnames\":[{\"value\":null,\"version\":0},{\"value\":\"usrs1\",\"version\":1},{\"value\":\"usrs2\",\"version\":1},{\"value\":\"usrs3\",\"version\":1},{\"value\":\"usrs3\",\"version\":1},{\"value\":\"usrs3\",\"version\":1}],\"subgraph_laziness\":[{\"value\":null,\"version\":0}]}",
                );
                df.__assign_diagnostics("[]");
                let (hoff_12v1_send, hoff_12v1_recv) = df
                    .make_edge::<
                        _,
                        ::dfir_rs::scheduled::handoff::VecHandoff<_>,
                    >("handoff GraphNodeId(12v1)");
                let (hoff_13v1_send, hoff_13v1_recv) = df
                    .make_edge::<
                        _,
                        ::dfir_rs::scheduled::handoff::VecHandoff<_>,
                    >("handoff GraphNodeId(13v1)");
                let (hoff_14v1_send, hoff_14v1_recv) = df
                    .make_edge::<
                        _,
                        ::dfir_rs::scheduled::handoff::VecHandoff<_>,
                    >("handoff GraphNodeId(14v1)");
                let (hoff_15v1_send, hoff_15v1_recv) = df
                    .make_edge::<
                        _,
                        ::dfir_rs::scheduled::handoff::VecHandoff<_>,
                    >("handoff GraphNodeId(15v1)");
                let loop_1v1 = df.add_loop(None);
                let loop_2v1 = df.add_loop(Some(loop_1v1));
                let loop_3v1 = df.add_loop(Some(loop_2v1));
                let loop_4v1 = df.add_loop(Some(loop_2v1));
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_1v1__source_iter__loc_dfir_rs_tests_surface_loop_rs_207_17_207_46<
                    T,
                >(thunk: impl FnOnce() -> T) -> T {
                    thunk()
                }
                let mut sg_1v1_node_1v1_iter = {
                    #[inline(always)]
                    fn check_iter<
                        IntoIter: ::std::iter::IntoIterator<Item = Item>,
                        Item,
                    >(into_iter: IntoIter) -> impl ::std::iter::Iterator<Item = Item> {
                        ::std::iter::IntoIterator::into_iter(into_iter)
                    }
                    check_iter(["alice", "bob"])
                };
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_2v1__batch__loc_dfir_rs_tests_surface_loop_rs_209_30_209_37<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_3v1__repeat_n__loc_dfir_rs_tests_surface_loop_rs_211_34_211_45<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(clippy::redundant_closure_call)]
                let singleton_op_3v1 = df
                    .add_state(::std::cell::RefCell::new(::std::vec::Vec::new()));
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_4v1__inspect__loc_dfir_rs_tests_surface_loop_rs_212_24_212_86<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_5v1__tee__loc_dfir_rs_tests_surface_loop_rs_213_24_213_29<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_6v1__repeat_n__loc_dfir_rs_tests_surface_loop_rs_215_30_215_41<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(clippy::redundant_closure_call)]
                let singleton_op_6v1 = df
                    .add_state(::std::cell::RefCell::new(::std::vec::Vec::new()));
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_7v1__inspect__loc_dfir_rs_tests_surface_loop_rs_216_24_216_98<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_8v1__for_each__loc_dfir_rs_tests_surface_loop_rs_217_24_217_67<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_9v1__repeat_n__loc_dfir_rs_tests_surface_loop_rs_220_30_220_41<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(clippy::redundant_closure_call)]
                let singleton_op_9v1 = df
                    .add_state(::std::cell::RefCell::new(::std::vec::Vec::new()));
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_10v1__inspect__loc_dfir_rs_tests_surface_loop_rs_221_28_221_102<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_11v1__for_each__loc_dfir_rs_tests_surface_loop_rs_222_28_222_71<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                let sgid_1v1 = df
                    .add_subgraph_full(
                        "Subgraph GraphSubgraphId(1v1)",
                        0,
                        (),
                        (hoff_12v1_send, ()),
                        false,
                        None,
                        move |context, (), (hoff_12v1_send, ())| {
                            let hoff_12v1_send = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                v|
                            {
                                hoff_12v1_send.give(Some(v));
                            });
                            let op_1v1 = sg_1v1_node_1v1_iter.by_ref();
                            let op_1v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_1v1__source_iter__loc_dfir_rs_tests_surface_loop_rs_207_17_207_46<
                                    Item,
                                    Input: ::std::iter::Iterator<Item = Item>,
                                >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Pull<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > Iterator for Pull<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item> {
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize, Option<usize>) {
                                            self.inner.size_hint()
                                        }
                                    }
                                    Pull { inner: input }
                                }
                                op_1v1__source_iter__loc_dfir_rs_tests_surface_loop_rs_207_17_207_46(
                                    op_1v1,
                                )
                            };
                            #[inline(always)]
                            fn pivot_run_sg_1v1<
                                Pull: ::std::iter::Iterator<Item = Item>,
                                Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                Item,
                            >(pull: Pull, push: Push) {
                                ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                            }
                            pivot_run_sg_1v1(op_1v1, hoff_12v1_send);
                        },
                    );
                let sgid_2v1 = df
                    .add_subgraph_full(
                        "Subgraph GraphSubgraphId(2v1)",
                        1,
                        (hoff_12v1_recv, ()),
                        (hoff_13v1_send, ()),
                        false,
                        Some(loop_1v1),
                        move |context, (hoff_12v1_recv, ()), (hoff_13v1_send, ())| {
                            let mut hoff_12v1_recv = hoff_12v1_recv.borrow_mut_swap();
                            let hoff_12v1_recv = hoff_12v1_recv.drain(..);
                            let hoff_13v1_send = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                v|
                            {
                                hoff_13v1_send.give(Some(v));
                            });
                            let op_2v1 = {
                                fn check_input<
                                    Iter: ::std::iter::Iterator<Item = Item>,
                                    Item,
                                >(iter: Iter) -> impl ::std::iter::Iterator<Item = Item> {
                                    iter
                                }
                                check_input::<_, _>(hoff_12v1_recv)
                            };
                            let op_2v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_2v1__batch__loc_dfir_rs_tests_surface_loop_rs_209_30_209_37<
                                    Item,
                                    Input: ::std::iter::Iterator<Item = Item>,
                                >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Pull<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > Iterator for Pull<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item> {
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize, Option<usize>) {
                                            self.inner.size_hint()
                                        }
                                    }
                                    Pull { inner: input }
                                }
                                op_2v1__batch__loc_dfir_rs_tests_surface_loop_rs_209_30_209_37(
                                    op_2v1,
                                )
                            };
                            #[inline(always)]
                            fn pivot_run_sg_2v1<
                                Pull: ::std::iter::Iterator<Item = Item>,
                                Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                Item,
                            >(pull: Pull, push: Push) {
                                ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                            }
                            pivot_run_sg_2v1(op_2v1, hoff_13v1_send);
                            context.allow_another_iteration();
                        },
                    );
                let sgid_3v1 = df
                    .add_subgraph_full(
                        "Subgraph GraphSubgraphId(3v1)",
                        2,
                        (hoff_13v1_recv, ()),
                        (hoff_14v1_send, (hoff_15v1_send, ())),
                        false,
                        Some(loop_2v1),
                        move |
                            context,
                            (hoff_13v1_recv, ()),
                            (hoff_14v1_send, (hoff_15v1_send, ()))|
                        {
                            let mut hoff_13v1_recv = hoff_13v1_recv.borrow_mut_swap();
                            let hoff_13v1_recv = hoff_13v1_recv.drain(..);
                            let hoff_14v1_send = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                v|
                            {
                                hoff_14v1_send.give(Some(v));
                            });
                            let hoff_15v1_send = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                v|
                            {
                                hoff_15v1_send.give(Some(v));
                            });
                            let mut sg_3v1_node_3v1_vec = unsafe {
                                context.state_ref_unchecked(singleton_op_3v1)
                            }
                                .borrow_mut();
                            if 0 == context.loop_iter_count() {
                                *sg_3v1_node_3v1_vec = hoff_13v1_recv
                                    .collect::<::std::vec::Vec<_>>();
                            }
                            let op_3v1 = std::iter::IntoIterator::into_iter(
                                ::std::clone::Clone::clone(&*sg_3v1_node_3v1_vec),
                            );
                            let op_3v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_3v1__repeat_n__loc_dfir_rs_tests_surface_loop_rs_211_34_211_45<
                                    Item,
                                    Input: ::std::iter::Iterator<Item = Item>,
                                >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Pull<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > Iterator for Pull<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item> {
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize, Option<usize>) {
                                            self.inner.size_hint()
                                        }
                                    }
                                    Pull { inner: input }
                                }
                                op_3v1__repeat_n__loc_dfir_rs_tests_surface_loop_rs_211_34_211_45(
                                    op_3v1,
                                )
                            };
                            let op_4v1 = op_3v1
                                .inspect(|x| {
                                    ::std::io::_print(
                                        format_args!("{0:?} {1}\n", x, context.loop_iter_count()),
                                    );
                                });
                            let op_4v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_4v1__inspect__loc_dfir_rs_tests_surface_loop_rs_212_24_212_86<
                                    Item,
                                    Input: ::std::iter::Iterator<Item = Item>,
                                >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Pull<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > Iterator for Pull<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item> {
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize, Option<usize>) {
                                            self.inner.size_hint()
                                        }
                                    }
                                    Pull { inner: input }
                                }
                                op_4v1__inspect__loc_dfir_rs_tests_surface_loop_rs_212_24_212_86(
                                    op_4v1,
                                )
                            };
                            let op_5v1 = ::dfir_rs::pusherator::tee::Tee::new(
                                hoff_14v1_send,
                                hoff_15v1_send,
                            );
                            let op_5v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_5v1__tee__loc_dfir_rs_tests_surface_loop_rs_213_24_213_29<
                                    Item,
                                    Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                >(
                                    input: Input,
                                ) -> impl ::dfir_rs::pusherator::Pusherator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Push<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    > ::dfir_rs::pusherator::Pusherator for Push<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn give(&mut self, item: Self::Item) {
                                            self.inner.give(item)
                                        }
                                    }
                                    Push { inner: input }
                                }
                                op_5v1__tee__loc_dfir_rs_tests_surface_loop_rs_213_24_213_29(
                                    op_5v1,
                                )
                            };
                            #[inline(always)]
                            fn pivot_run_sg_3v1<
                                Pull: ::std::iter::Iterator<Item = Item>,
                                Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                Item,
                            >(pull: Pull, push: Push) {
                                ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                            }
                            pivot_run_sg_3v1(op_4v1, op_5v1);
                            {
                                if context.loop_iter_count() + 1 < 3 {
                                    context.reschedule_loop_block();
                                }
                            }
                        },
                    );
                let sgid_4v1 = df
                    .add_subgraph_full(
                        "Subgraph GraphSubgraphId(4v1)",
                        3,
                        (hoff_14v1_recv, ()),
                        (),
                        false,
                        Some(loop_3v1),
                        move |context, (hoff_14v1_recv, ()), ()| {
                            let mut hoff_14v1_recv = hoff_14v1_recv.borrow_mut_swap();
                            let hoff_14v1_recv = hoff_14v1_recv.drain(..);
                            let mut sg_4v1_node_6v1_vec = unsafe {
                                context.state_ref_unchecked(singleton_op_6v1)
                            }
                                .borrow_mut();
                            if 0 == context.loop_iter_count() {
                                *sg_4v1_node_6v1_vec = hoff_14v1_recv
                                    .collect::<::std::vec::Vec<_>>();
                            }
                            let op_6v1 = std::iter::IntoIterator::into_iter(
                                ::std::clone::Clone::clone(&*sg_4v1_node_6v1_vec),
                            );
                            let op_6v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_6v1__repeat_n__loc_dfir_rs_tests_surface_loop_rs_215_30_215_41<
                                    Item,
                                    Input: ::std::iter::Iterator<Item = Item>,
                                >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Pull<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > Iterator for Pull<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item> {
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize, Option<usize>) {
                                            self.inner.size_hint()
                                        }
                                    }
                                    Pull { inner: input }
                                }
                                op_6v1__repeat_n__loc_dfir_rs_tests_surface_loop_rs_215_30_215_41(
                                    op_6v1,
                                )
                            };
                            let op_7v1 = op_6v1
                                .inspect(|x| {
                                    ::std::io::_print(
                                        format_args!(
                                            "{0} {1:?} {2}\n", 216u32, x, context.loop_iter_count()
                                        ),
                                    );
                                });
                            let op_7v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_7v1__inspect__loc_dfir_rs_tests_surface_loop_rs_216_24_216_98<
                                    Item,
                                    Input: ::std::iter::Iterator<Item = Item>,
                                >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Pull<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > Iterator for Pull<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item> {
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize, Option<usize>) {
                                            self.inner.size_hint()
                                        }
                                    }
                                    Pull { inner: input }
                                }
                                op_7v1__inspect__loc_dfir_rs_tests_surface_loop_rs_216_24_216_98(
                                    op_7v1,
                                )
                            };
                            let op_8v1 = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                x|
                            result1_send.send(x).unwrap());
                            let op_8v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_8v1__for_each__loc_dfir_rs_tests_surface_loop_rs_217_24_217_67<
                                    Item,
                                    Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                >(
                                    input: Input,
                                ) -> impl ::dfir_rs::pusherator::Pusherator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Push<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    > ::dfir_rs::pusherator::Pusherator for Push<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn give(&mut self, item: Self::Item) {
                                            self.inner.give(item)
                                        }
                                    }
                                    Push { inner: input }
                                }
                                op_8v1__for_each__loc_dfir_rs_tests_surface_loop_rs_217_24_217_67(
                                    op_8v1,
                                )
                            };
                            #[inline(always)]
                            fn pivot_run_sg_4v1<
                                Pull: ::std::iter::Iterator<Item = Item>,
                                Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                Item,
                            >(pull: Pull, push: Push) {
                                ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                            }
                            pivot_run_sg_4v1(op_7v1, op_8v1);
                            {
                                if context.loop_iter_count() + 1 < 3 {
                                    context.reschedule_loop_block();
                                }
                            }
                        },
                    );
                let sgid_5v1 = df
                    .add_subgraph_full(
                        "Subgraph GraphSubgraphId(5v1)",
                        3,
                        (hoff_15v1_recv, ()),
                        (),
                        false,
                        Some(loop_4v1),
                        move |context, (hoff_15v1_recv, ()), ()| {
                            let mut hoff_15v1_recv = hoff_15v1_recv.borrow_mut_swap();
                            let hoff_15v1_recv = hoff_15v1_recv.drain(..);
                            let mut sg_5v1_node_9v1_vec = unsafe {
                                context.state_ref_unchecked(singleton_op_9v1)
                            }
                                .borrow_mut();
                            if 0 == context.loop_iter_count() {
                                *sg_5v1_node_9v1_vec = hoff_15v1_recv
                                    .collect::<::std::vec::Vec<_>>();
                            }
                            let op_9v1 = std::iter::IntoIterator::into_iter(
                                ::std::clone::Clone::clone(&*sg_5v1_node_9v1_vec),
                            );
                            let op_9v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_9v1__repeat_n__loc_dfir_rs_tests_surface_loop_rs_220_30_220_41<
                                    Item,
                                    Input: ::std::iter::Iterator<Item = Item>,
                                >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Pull<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > Iterator for Pull<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item> {
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize, Option<usize>) {
                                            self.inner.size_hint()
                                        }
                                    }
                                    Pull { inner: input }
                                }
                                op_9v1__repeat_n__loc_dfir_rs_tests_surface_loop_rs_220_30_220_41(
                                    op_9v1,
                                )
                            };
                            let op_10v1 = op_9v1
                                .inspect(|x| {
                                    ::std::io::_print(
                                        format_args!(
                                            "{0} {1:?} {2}\n", 221u32, x, context.loop_iter_count()
                                        ),
                                    );
                                });
                            let op_10v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_10v1__inspect__loc_dfir_rs_tests_surface_loop_rs_221_28_221_102<
                                    Item,
                                    Input: ::std::iter::Iterator<Item = Item>,
                                >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Pull<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > Iterator for Pull<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item> {
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize, Option<usize>) {
                                            self.inner.size_hint()
                                        }
                                    }
                                    Pull { inner: input }
                                }
                                op_10v1__inspect__loc_dfir_rs_tests_surface_loop_rs_221_28_221_102(
                                    op_10v1,
                                )
                            };
                            let op_11v1 = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                x|
                            result2_send.send(x).unwrap());
                            let op_11v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_11v1__for_each__loc_dfir_rs_tests_surface_loop_rs_222_28_222_71<
                                    Item,
                                    Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                >(
                                    input: Input,
                                ) -> impl ::dfir_rs::pusherator::Pusherator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Push<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    > ::dfir_rs::pusherator::Pusherator for Push<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn give(&mut self, item: Self::Item) {
                                            self.inner.give(item)
                                        }
                                    }
                                    Push { inner: input }
                                }
                                op_11v1__for_each__loc_dfir_rs_tests_surface_loop_rs_222_28_222_71(
                                    op_11v1,
                                )
                            };
                            #[inline(always)]
                            fn pivot_run_sg_5v1<
                                Pull: ::std::iter::Iterator<Item = Item>,
                                Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                Item,
                            >(pull: Pull, push: Push) {
                                ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                            }
                            pivot_run_sg_5v1(op_10v1, op_11v1);
                            {
                                if context.loop_iter_count() + 1 < 3 {
                                    context.reschedule_loop_block();
                                }
                            }
                        },
                    );
                df
            }
        }
    };
    {
        #[cfg(not(target_arch = "wasm32"))]
        {
            {
                let mut settings = ::insta::Settings::clone_current();
                settings._private_inner_mut().snapshot_suffix("graphvis_mermaid");
                settings
                    .bind(|| {
                        ::insta::_macro_support::assert_snapshot(
                                ::insta::_macro_support::AutoName.into(),
                                #[allow(clippy::redundant_closure_call)]
                                &(|v| ::alloc::__export::must_use({
                                    let res = ::alloc::fmt::format(format_args!("{0}", v));
                                    res
                                }))(
                                    &df.meta_graph().unwrap().to_mermaid(&Default::default()),
                                ),
                                "F:\\Projects\\hydroflow\\dfir_rs",
                                {
                                    fn f() {}
                                    fn type_name_of_val<T>(_: T) -> &'static str {
                                        std::any::type_name::<T>()
                                    }
                                    let mut name = type_name_of_val(f)
                                        .strip_suffix("::f")
                                        .unwrap_or("");
                                    while let Some(rest) = name.strip_suffix("::{{closure}}") {
                                        name = rest;
                                    }
                                    name
                                },
                                "surface_loop",
                                "dfir_rs\\tests\\surface_loop.rs",
                                227u32,
                                "df.meta_graph().unwrap().to_mermaid(& Default :: default())",
                            )
                            .unwrap();
                    })
            };
            {
                let mut settings = ::insta::Settings::clone_current();
                settings._private_inner_mut().snapshot_suffix("graphvis_dot");
                settings
                    .bind(|| {
                        ::insta::_macro_support::assert_snapshot(
                                ::insta::_macro_support::AutoName.into(),
                                #[allow(clippy::redundant_closure_call)]
                                &(|v| ::alloc::__export::must_use({
                                    let res = ::alloc::fmt::format(format_args!("{0}", v));
                                    res
                                }))(&df.meta_graph().unwrap().to_dot(&Default::default())),
                                "F:\\Projects\\hydroflow\\dfir_rs",
                                {
                                    fn f() {}
                                    fn type_name_of_val<T>(_: T) -> &'static str {
                                        std::any::type_name::<T>()
                                    }
                                    let mut name = type_name_of_val(f)
                                        .strip_suffix("::f")
                                        .unwrap_or("");
                                    while let Some(rest) = name.strip_suffix("::{{closure}}") {
                                        name = rest;
                                    }
                                    name
                                },
                                "surface_loop",
                                "dfir_rs\\tests\\surface_loop.rs",
                                227u32,
                                "df.meta_graph().unwrap().to_dot(& Default :: default())",
                            )
                            .unwrap();
                    })
            };
        }
    };
    df.run_available();
    match (
        &&[
            "alice",
            "bob",
            "alice",
            "bob",
            "alice",
            "bob",
            "alice",
            "bob",
            "alice",
            "bob",
            "alice",
            "bob",
            "alice",
            "bob",
            "alice",
            "bob",
            "alice",
            "bob",
        ],
        &&*collect_ready::<Vec<_>, _>(&mut result1_recv),
    ) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    match (
        &&[
            "alice",
            "bob",
            "alice",
            "bob",
            "alice",
            "bob",
            "alice",
            "bob",
            "alice",
            "bob",
            "alice",
            "bob",
            "alice",
            "bob",
            "alice",
            "bob",
            "alice",
            "bob",
        ],
        &&*collect_ready::<Vec<_>, _>(&mut result2_recv),
    ) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
}
extern crate test;
#[cfg(test)]
#[rustc_test_marker = "test_flo_repeat_kmeans"]
#[doc(hidden)]
pub const test_flo_repeat_kmeans: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_flo_repeat_kmeans"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "dfir_rs\\tests\\surface_loop.rs",
        start_line: 248usize,
        start_col: 8usize,
        end_line: 248usize,
        end_col: 30usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_flo_repeat_kmeans()),
    ),
};
const _: () = {};
pub fn test_flo_repeat_kmeans() {
    {
        let subscriber = tracing_subscriber::FmtSubscriber::builder()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_test_writer()
            .finish();
        let _ = tracing::subscriber::set_global_default(subscriber);
    };
    {
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
        let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<_>();
        let mut df = {
            {
                #[allow(unused_qualifications)]
                {
                    use ::dfir_rs::{var_expr, var_args};
                    let mut df = ::dfir_rs::scheduled::graph::Dfir::new();
                    df.__assign_meta_graph(
                        "{\"nodes\":[{\"value\":null,\"version\":0},{\"value\":{\"Operator\":\"source_iter(POINTS)\"},\"version\":1},{\"value\":{\"Operator\":\"map(std :: clone :: Clone :: clone)\"},\"version\":1},{\"value\":{\"Operator\":\"source_iter(CENTROIDS)\"},\"version\":1},{\"value\":{\"Operator\":\"map(std :: clone :: Clone :: clone)\"},\"version\":1},{\"value\":{\"Operator\":\"batch()\"},\"version\":1},{\"value\":{\"Operator\":\"batch()\"},\"version\":1},{\"value\":{\"Operator\":\"repeat_n(10)\"},\"version\":1},{\"value\":{\"Operator\":\"all_once()\"},\"version\":1},{\"value\":{\"Operator\":\"union()\"},\"version\":1},{\"value\":{\"Operator\":\"tee()\"},\"version\":1},{\"value\":{\"Operator\":\"cross_join_multiset()\"},\"version\":1},{\"value\":{\"Operator\":\"map(| (point, centroid) : ([i32; 2], [i32; 2]) |\\n{\\n    let dist2 = (point [0] - centroid [0]).pow(2) +\\n    (point [1] - centroid [1]).pow(2); (point, (dist2, centroid))\\n})\"},\"version\":1},{\"value\":{\"Operator\":\"reduce_keyed(| (a_dist2, a_centroid), (b_dist2, b_centroid) |\\n{\\n    if b_dist2 < * a_dist2 { * a_dist2 = b_dist2; * a_centroid = b_centroid; }\\n})\"},\"version\":1},{\"value\":{\"Operator\":\"map(| (point, (_dist2, centroid)) | { (centroid, (point, 1)) })\"},\"version\":1},{\"value\":{\"Operator\":\"reduce_keyed(| (p1, n1), (p2, n2) : ([i32; 2], i32) |\\n{ p1 [0] += p2 [0]; p1 [1] += p2 [1]; * n1 += n2; })\"},\"version\":1},{\"value\":{\"Operator\":\"map(| (_centroid, (p, n)) : (_, ([i32; 2], i32)) | { [p [0] / n, p [1] / n] })\"},\"version\":1},{\"value\":{\"Operator\":\"next_iteration()\"},\"version\":1},{\"value\":{\"Operator\":\"inspect(| x | println! (\\\"centroid: {:?}\\\", x))\"},\"version\":1},{\"value\":{\"Operator\":\"all_iterations()\"},\"version\":1},{\"value\":{\"Operator\":\"for_each(| x | result_send.send(x).unwrap())\"},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1}],\"operator_tag\":[{\"value\":null,\"version\":0}],\"graph\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":1,\"version\":1},{\"idx\":2,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":3,\"version\":1},{\"idx\":4,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":2,\"version\":1},{\"idx\":21,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":4,\"version\":1},{\"idx\":22,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":7,\"version\":1},{\"idx\":11,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":5,\"version\":1},{\"idx\":23,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":8,\"version\":1},{\"idx\":9,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":6,\"version\":1},{\"idx\":24,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":9,\"version\":1},{\"idx\":10,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":10,\"version\":1},{\"idx\":25,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":18,\"version\":1},{\"idx\":9,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":17,\"version\":1},{\"idx\":18,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":16,\"version\":1},{\"idx\":26,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":15,\"version\":1},{\"idx\":16,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":14,\"version\":1},{\"idx\":15,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":13,\"version\":1},{\"idx\":14,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":12,\"version\":1},{\"idx\":13,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":11,\"version\":1},{\"idx\":12,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":19,\"version\":1},{\"idx\":20,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":10,\"version\":1},{\"idx\":27,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":21,\"version\":1},{\"idx\":5,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":22,\"version\":1},{\"idx\":6,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":23,\"version\":1},{\"idx\":7,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":24,\"version\":1},{\"idx\":8,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":25,\"version\":1},{\"idx\":11,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":26,\"version\":1},{\"idx\":17,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":27,\"version\":1},{\"idx\":19,\"version\":1}],\"version\":1}],\"ports\":[{\"value\":null,\"version\":0},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",{\"Int\":\"0\"}],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",{\"Int\":\"1\"}],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1}],\"node_loops\":[{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1}],\"loop_nodes\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":5,\"version\":1},{\"idx\":6,\"version\":1},{\"idx\":19,\"version\":1},{\"idx\":20,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":7,\"version\":1},{\"idx\":8,\"version\":1},{\"idx\":9,\"version\":1},{\"idx\":10,\"version\":1},{\"idx\":11,\"version\":1},{\"idx\":12,\"version\":1},{\"idx\":13,\"version\":1},{\"idx\":14,\"version\":1},{\"idx\":15,\"version\":1},{\"idx\":16,\"version\":1},{\"idx\":17,\"version\":1},{\"idx\":18,\"version\":1}],\"version\":1}],\"loop_parent\":[{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":{\"idx\":1,\"version\":1},\"version\":1}],\"root_loops\":[{\"idx\":1,\"version\":1}],\"loop_children\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":2,\"version\":1}],\"version\":1},{\"value\":[],\"version\":1}],\"node_subgraph\":[{\"value\":null,\"version\":0},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1},{\"value\":{\"idx\":4,\"version\":1},\"version\":1},{\"value\":{\"idx\":6,\"version\":1},\"version\":1},{\"value\":{\"idx\":5,\"version\":1},\"version\":1},{\"value\":{\"idx\":5,\"version\":1},\"version\":1},{\"value\":{\"idx\":5,\"version\":1},\"version\":1},{\"value\":{\"idx\":6,\"version\":1},\"version\":1},{\"value\":{\"idx\":6,\"version\":1},\"version\":1},{\"value\":{\"idx\":6,\"version\":1},\"version\":1},{\"value\":{\"idx\":6,\"version\":1},\"version\":1},{\"value\":{\"idx\":6,\"version\":1},\"version\":1},{\"value\":{\"idx\":6,\"version\":1},\"version\":1},{\"value\":{\"idx\":5,\"version\":1},\"version\":1},{\"value\":{\"idx\":5,\"version\":1},\"version\":1},{\"value\":{\"idx\":7,\"version\":1},\"version\":1},{\"value\":{\"idx\":7,\"version\":1},\"version\":1}],\"subgraph_nodes\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":1,\"version\":1},{\"idx\":2,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":3,\"version\":1},{\"idx\":4,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":5,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":6,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":8,\"version\":1},{\"idx\":17,\"version\":1},{\"idx\":18,\"version\":1},{\"idx\":9,\"version\":1},{\"idx\":10,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":7,\"version\":1},{\"idx\":11,\"version\":1},{\"idx\":12,\"version\":1},{\"idx\":13,\"version\":1},{\"idx\":14,\"version\":1},{\"idx\":15,\"version\":1},{\"idx\":16,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":19,\"version\":1},{\"idx\":20,\"version\":1}],\"version\":1}],\"subgraph_stratum\":[{\"value\":null,\"version\":0},{\"value\":0,\"version\":1},{\"value\":0,\"version\":1},{\"value\":1,\"version\":1},{\"value\":1,\"version\":1},{\"value\":2,\"version\":1},{\"value\":2,\"version\":1},{\"value\":3,\"version\":1}],\"node_singleton_references\":[{\"value\":null,\"version\":0},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1}],\"node_varnames\":[{\"value\":null,\"version\":0},{\"value\":\"init_points\",\"version\":1},{\"value\":\"init_points\",\"version\":1},{\"value\":\"init_centroids\",\"version\":1},{\"value\":\"init_centroids\",\"version\":1},{\"value\":\"batch_points\",\"version\":1},{\"value\":\"batch_centroids\",\"version\":1},{\"value\":\"points\",\"version\":1},{\"value\":null,\"version\":0},{\"value\":\"centroids\",\"version\":1},{\"value\":\"centroids\",\"version\":1},{\"value\":\"cj\",\"version\":1},{\"value\":\"cj\",\"version\":1},{\"value\":\"cj\",\"version\":1},{\"value\":\"cj\",\"version\":1},{\"value\":\"cj\",\"version\":1},{\"value\":\"cj\",\"version\":1},{\"value\":\"cj\",\"version\":1},{\"value\":\"cj\",\"version\":1}],\"subgraph_laziness\":[{\"value\":null,\"version\":0}]}",
                    );
                    df.__assign_diagnostics("[]");
                    let (hoff_21v1_send, hoff_21v1_recv) = df
                        .make_edge::<
                            _,
                            ::dfir_rs::scheduled::handoff::VecHandoff<_>,
                        >("handoff GraphNodeId(21v1)");
                    let (hoff_22v1_send, hoff_22v1_recv) = df
                        .make_edge::<
                            _,
                            ::dfir_rs::scheduled::handoff::VecHandoff<_>,
                        >("handoff GraphNodeId(22v1)");
                    let (hoff_23v1_send, hoff_23v1_recv) = df
                        .make_edge::<
                            _,
                            ::dfir_rs::scheduled::handoff::VecHandoff<_>,
                        >("handoff GraphNodeId(23v1)");
                    let (hoff_24v1_send, hoff_24v1_recv) = df
                        .make_edge::<
                            _,
                            ::dfir_rs::scheduled::handoff::VecHandoff<_>,
                        >("handoff GraphNodeId(24v1)");
                    let (hoff_25v1_send, hoff_25v1_recv) = df
                        .make_edge::<
                            _,
                            ::dfir_rs::scheduled::handoff::VecHandoff<_>,
                        >("handoff GraphNodeId(25v1)");
                    let (hoff_26v1_send, hoff_26v1_recv) = df
                        .make_edge::<
                            _,
                            ::dfir_rs::scheduled::handoff::VecHandoff<_>,
                        >("handoff GraphNodeId(26v1)");
                    let (hoff_27v1_send, hoff_27v1_recv) = df
                        .make_edge::<
                            _,
                            ::dfir_rs::scheduled::handoff::VecHandoff<_>,
                        >("handoff GraphNodeId(27v1)");
                    let loop_1v1 = df.add_loop(None);
                    let loop_2v1 = df.add_loop(Some(loop_1v1));
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_1v1__source_iter__loc_dfir_rs_tests_surface_loop_rs_286_23_286_42<
                        T,
                    >(thunk: impl FnOnce() -> T) -> T {
                        thunk()
                    }
                    let mut sg_1v1_node_1v1_iter = {
                        #[inline(always)]
                        fn check_iter<
                            IntoIter: ::std::iter::IntoIterator<Item = Item>,
                            Item,
                        >(
                            into_iter: IntoIter,
                        ) -> impl ::std::iter::Iterator<Item = Item> {
                            ::std::iter::IntoIterator::into_iter(into_iter)
                        }
                        check_iter(POINTS)
                    };
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_2v1__map__loc_dfir_rs_tests_surface_loop_rs_286_46_286_75<T>(
                        thunk: impl FnOnce() -> T,
                    ) -> T {
                        thunk()
                    }
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_3v1__source_iter__loc_dfir_rs_tests_surface_loop_rs_287_26_287_48<
                        T,
                    >(thunk: impl FnOnce() -> T) -> T {
                        thunk()
                    }
                    let mut sg_2v1_node_3v1_iter = {
                        #[inline(always)]
                        fn check_iter<
                            IntoIter: ::std::iter::IntoIterator<Item = Item>,
                            Item,
                        >(
                            into_iter: IntoIter,
                        ) -> impl ::std::iter::Iterator<Item = Item> {
                            ::std::iter::IntoIterator::into_iter(into_iter)
                        }
                        check_iter(CENTROIDS)
                    };
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_4v1__map__loc_dfir_rs_tests_surface_loop_rs_287_52_287_81<T>(
                        thunk: impl FnOnce() -> T,
                    ) -> T {
                        thunk()
                    }
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_5v1__batch__loc_dfir_rs_tests_surface_loop_rs_289_43_289_50<T>(
                        thunk: impl FnOnce() -> T,
                    ) -> T {
                        thunk()
                    }
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_6v1__batch__loc_dfir_rs_tests_surface_loop_rs_290_49_290_56<T>(
                        thunk: impl FnOnce() -> T,
                    ) -> T {
                        thunk()
                    }
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_8v1__all_once__loc_dfir_rs_tests_surface_loop_rs_296_36_296_46<
                        T,
                    >(thunk: impl FnOnce() -> T) -> T {
                        thunk()
                    }
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_17v1__next_iteration__loc_dfir_rs_tests_surface_loop_rs_323_24_323_40<
                        T,
                    >(thunk: impl FnOnce() -> T) -> T {
                        thunk()
                    }
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_18v1__inspect__loc_dfir_rs_tests_surface_loop_rs_324_24_324_66<
                        T,
                    >(thunk: impl FnOnce() -> T) -> T {
                        thunk()
                    }
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_9v1__union__loc_dfir_rs_tests_surface_loop_rs_298_29_298_36<T>(
                        thunk: impl FnOnce() -> T,
                    ) -> T {
                        thunk()
                    }
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_10v1__tee__loc_dfir_rs_tests_surface_loop_rs_298_40_298_45<T>(
                        thunk: impl FnOnce() -> T,
                    ) -> T {
                        thunk()
                    }
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_7v1__repeat_n__loc_dfir_rs_tests_surface_loop_rs_294_24_294_36<
                        T,
                    >(thunk: impl FnOnce() -> T) -> T {
                        thunk()
                    }
                    #[allow(clippy::redundant_closure_call)]
                    let singleton_op_7v1 = df
                        .add_state(::std::cell::RefCell::new(::std::vec::Vec::new()));
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_11v1__cross_join_multiset__loc_dfir_rs_tests_surface_loop_rs_301_22_301_43<
                        T,
                    >(thunk: impl FnOnce() -> T) -> T {
                        thunk()
                    }
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_12v1__map__loc_dfir_rs_tests_surface_loop_rs_302_24_305_23<T>(
                        thunk: impl FnOnce() -> T,
                    ) -> T {
                        thunk()
                    }
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_13v1__reduce_keyed__loc_dfir_rs_tests_surface_loop_rs_306_24_311_23<
                        T,
                    >(thunk: impl FnOnce() -> T) -> T {
                        thunk()
                    }
                    let sg_6v1_node_13v1_groupbydata = df
                        .add_state(
                            ::std::cell::RefCell::new(
                                ::dfir_rs::rustc_hash::FxHashMap::<_, _>::default(),
                            ),
                        );
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_14v1__map__loc_dfir_rs_tests_surface_loop_rs_312_24_314_23<T>(
                        thunk: impl FnOnce() -> T,
                    ) -> T {
                        thunk()
                    }
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_15v1__reduce_keyed__loc_dfir_rs_tests_surface_loop_rs_315_24_319_23<
                        T,
                    >(thunk: impl FnOnce() -> T) -> T {
                        thunk()
                    }
                    let sg_6v1_node_15v1_groupbydata = df
                        .add_state(
                            ::std::cell::RefCell::new(
                                ::dfir_rs::rustc_hash::FxHashMap::<_, _>::default(),
                            ),
                        );
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_16v1__map__loc_dfir_rs_tests_surface_loop_rs_320_24_322_23<T>(
                        thunk: impl FnOnce() -> T,
                    ) -> T {
                        thunk()
                    }
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_19v1__all_iterations__loc_dfir_rs_tests_surface_loop_rs_329_20_329_36<
                        T,
                    >(thunk: impl FnOnce() -> T) -> T {
                        thunk()
                    }
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    fn op_20v1__for_each__loc_dfir_rs_tests_surface_loop_rs_330_20_330_62<
                        T,
                    >(thunk: impl FnOnce() -> T) -> T {
                        thunk()
                    }
                    let sgid_1v1 = df
                        .add_subgraph_full(
                            "Subgraph GraphSubgraphId(1v1)",
                            0,
                            (),
                            (hoff_21v1_send, ()),
                            false,
                            None,
                            move |context, (), (hoff_21v1_send, ())| {
                                let hoff_21v1_send = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                    v|
                                {
                                    hoff_21v1_send.give(Some(v));
                                });
                                let op_1v1 = sg_1v1_node_1v1_iter.by_ref();
                                let op_1v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_1v1__source_iter__loc_dfir_rs_tests_surface_loop_rs_286_23_286_42<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_1v1__source_iter__loc_dfir_rs_tests_surface_loop_rs_286_23_286_42(
                                        op_1v1,
                                    )
                                };
                                #[allow(
                                    clippy::map_clone,
                                    reason = "dfir has no explicit `cloned`/`copied` operator"
                                )]
                                let op_2v1 = op_1v1.map(std::clone::Clone::clone);
                                let op_2v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_2v1__map__loc_dfir_rs_tests_surface_loop_rs_286_46_286_75<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_2v1__map__loc_dfir_rs_tests_surface_loop_rs_286_46_286_75(
                                        op_2v1,
                                    )
                                };
                                #[inline(always)]
                                fn pivot_run_sg_1v1<
                                    Pull: ::std::iter::Iterator<Item = Item>,
                                    Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    Item,
                                >(pull: Pull, push: Push) {
                                    ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                                }
                                pivot_run_sg_1v1(op_2v1, hoff_21v1_send);
                            },
                        );
                    let sgid_2v1 = df
                        .add_subgraph_full(
                            "Subgraph GraphSubgraphId(2v1)",
                            0,
                            (),
                            (hoff_22v1_send, ()),
                            false,
                            None,
                            move |context, (), (hoff_22v1_send, ())| {
                                let hoff_22v1_send = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                    v|
                                {
                                    hoff_22v1_send.give(Some(v));
                                });
                                let op_3v1 = sg_2v1_node_3v1_iter.by_ref();
                                let op_3v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_3v1__source_iter__loc_dfir_rs_tests_surface_loop_rs_287_26_287_48<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_3v1__source_iter__loc_dfir_rs_tests_surface_loop_rs_287_26_287_48(
                                        op_3v1,
                                    )
                                };
                                #[allow(
                                    clippy::map_clone,
                                    reason = "dfir has no explicit `cloned`/`copied` operator"
                                )]
                                let op_4v1 = op_3v1.map(std::clone::Clone::clone);
                                let op_4v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_4v1__map__loc_dfir_rs_tests_surface_loop_rs_287_52_287_81<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_4v1__map__loc_dfir_rs_tests_surface_loop_rs_287_52_287_81(
                                        op_4v1,
                                    )
                                };
                                #[inline(always)]
                                fn pivot_run_sg_2v1<
                                    Pull: ::std::iter::Iterator<Item = Item>,
                                    Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    Item,
                                >(pull: Pull, push: Push) {
                                    ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                                }
                                pivot_run_sg_2v1(op_4v1, hoff_22v1_send);
                            },
                        );
                    let sgid_3v1 = df
                        .add_subgraph_full(
                            "Subgraph GraphSubgraphId(3v1)",
                            1,
                            (hoff_21v1_recv, ()),
                            (hoff_23v1_send, ()),
                            false,
                            Some(loop_1v1),
                            move |context, (hoff_21v1_recv, ()), (hoff_23v1_send, ())| {
                                let mut hoff_21v1_recv = hoff_21v1_recv.borrow_mut_swap();
                                let hoff_21v1_recv = hoff_21v1_recv.drain(..);
                                let hoff_23v1_send = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                    v|
                                {
                                    hoff_23v1_send.give(Some(v));
                                });
                                let op_5v1 = {
                                    fn check_input<
                                        Iter: ::std::iter::Iterator<Item = Item>,
                                        Item,
                                    >(iter: Iter) -> impl ::std::iter::Iterator<Item = Item> {
                                        iter
                                    }
                                    check_input::<_, _>(hoff_21v1_recv)
                                };
                                let op_5v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_5v1__batch__loc_dfir_rs_tests_surface_loop_rs_289_43_289_50<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_5v1__batch__loc_dfir_rs_tests_surface_loop_rs_289_43_289_50(
                                        op_5v1,
                                    )
                                };
                                #[inline(always)]
                                fn pivot_run_sg_3v1<
                                    Pull: ::std::iter::Iterator<Item = Item>,
                                    Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    Item,
                                >(pull: Pull, push: Push) {
                                    ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                                }
                                pivot_run_sg_3v1(op_5v1, hoff_23v1_send);
                                context.allow_another_iteration();
                            },
                        );
                    let sgid_4v1 = df
                        .add_subgraph_full(
                            "Subgraph GraphSubgraphId(4v1)",
                            1,
                            (hoff_22v1_recv, ()),
                            (hoff_24v1_send, ()),
                            false,
                            Some(loop_1v1),
                            move |context, (hoff_22v1_recv, ()), (hoff_24v1_send, ())| {
                                let mut hoff_22v1_recv = hoff_22v1_recv.borrow_mut_swap();
                                let hoff_22v1_recv = hoff_22v1_recv.drain(..);
                                let hoff_24v1_send = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                    v|
                                {
                                    hoff_24v1_send.give(Some(v));
                                });
                                let op_6v1 = {
                                    fn check_input<
                                        Iter: ::std::iter::Iterator<Item = Item>,
                                        Item,
                                    >(iter: Iter) -> impl ::std::iter::Iterator<Item = Item> {
                                        iter
                                    }
                                    check_input::<_, _>(hoff_22v1_recv)
                                };
                                let op_6v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_6v1__batch__loc_dfir_rs_tests_surface_loop_rs_290_49_290_56<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_6v1__batch__loc_dfir_rs_tests_surface_loop_rs_290_49_290_56(
                                        op_6v1,
                                    )
                                };
                                #[inline(always)]
                                fn pivot_run_sg_4v1<
                                    Pull: ::std::iter::Iterator<Item = Item>,
                                    Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    Item,
                                >(pull: Pull, push: Push) {
                                    ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                                }
                                pivot_run_sg_4v1(op_6v1, hoff_24v1_send);
                                context.allow_another_iteration();
                            },
                        );
                    let sgid_5v1 = df
                        .add_subgraph_full(
                            "Subgraph GraphSubgraphId(5v1)",
                            2,
                            (hoff_24v1_recv, (hoff_26v1_recv, ())),
                            (hoff_25v1_send, (hoff_27v1_send, ())),
                            false,
                            Some(loop_2v1),
                            move |
                                context,
                                (hoff_24v1_recv, (hoff_26v1_recv, ())),
                                (hoff_25v1_send, (hoff_27v1_send, ()))|
                            {
                                let mut hoff_24v1_recv = hoff_24v1_recv.borrow_mut_swap();
                                let hoff_24v1_recv = hoff_24v1_recv.drain(..);
                                let mut hoff_26v1_recv = hoff_26v1_recv.borrow_mut_swap();
                                let hoff_26v1_recv = hoff_26v1_recv.drain(..);
                                let hoff_25v1_send = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                    v|
                                {
                                    hoff_25v1_send.give(Some(v));
                                });
                                let hoff_27v1_send = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                    v|
                                {
                                    hoff_27v1_send.give(Some(v));
                                });
                                let op_8v1 = {
                                    fn check_input<
                                        Iter: ::std::iter::Iterator<Item = Item>,
                                        Item,
                                    >(iter: Iter) -> impl ::std::iter::Iterator<Item = Item> {
                                        iter
                                    }
                                    check_input::<_, _>(hoff_24v1_recv)
                                };
                                let op_8v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_8v1__all_once__loc_dfir_rs_tests_surface_loop_rs_296_36_296_46<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_8v1__all_once__loc_dfir_rs_tests_surface_loop_rs_296_36_296_46(
                                        op_8v1,
                                    )
                                };
                                let op_17v1 = ::std::iter::Iterator::filter(
                                    hoff_26v1_recv,
                                    |_| 0 != context.loop_iter_count(),
                                );
                                let op_17v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_17v1__next_iteration__loc_dfir_rs_tests_surface_loop_rs_323_24_323_40<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_17v1__next_iteration__loc_dfir_rs_tests_surface_loop_rs_323_24_323_40(
                                        op_17v1,
                                    )
                                };
                                let op_18v1 = op_17v1
                                    .inspect(|x| {
                                        ::std::io::_print(format_args!("centroid: {0:?}\n", x));
                                    });
                                let op_18v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_18v1__inspect__loc_dfir_rs_tests_surface_loop_rs_324_24_324_66<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_18v1__inspect__loc_dfir_rs_tests_surface_loop_rs_324_24_324_66(
                                        op_18v1,
                                    )
                                };
                                let op_9v1 = {
                                    #[allow(unused)]
                                    #[inline(always)]
                                    fn check_inputs<
                                        A: ::std::iter::Iterator<Item = Item>,
                                        B: ::std::iter::Iterator<Item = Item>,
                                        Item,
                                    >(a: A, b: B) -> impl ::std::iter::Iterator<Item = Item> {
                                        a.chain(b)
                                    }
                                    check_inputs(op_8v1, op_18v1)
                                };
                                let op_9v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_9v1__union__loc_dfir_rs_tests_surface_loop_rs_298_29_298_36<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_9v1__union__loc_dfir_rs_tests_surface_loop_rs_298_29_298_36(
                                        op_9v1,
                                    )
                                };
                                let op_10v1 = ::dfir_rs::pusherator::tee::Tee::new(
                                    hoff_25v1_send,
                                    hoff_27v1_send,
                                );
                                let op_10v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_10v1__tee__loc_dfir_rs_tests_surface_loop_rs_298_40_298_45<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    >(
                                        input: Input,
                                    ) -> impl ::dfir_rs::pusherator::Pusherator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Push<
                                            Item,
                                            Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                        > ::dfir_rs::pusherator::Pusherator for Push<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn give(&mut self, item: Self::Item) {
                                                self.inner.give(item)
                                            }
                                        }
                                        Push { inner: input }
                                    }
                                    op_10v1__tee__loc_dfir_rs_tests_surface_loop_rs_298_40_298_45(
                                        op_10v1,
                                    )
                                };
                                #[inline(always)]
                                fn pivot_run_sg_5v1<
                                    Pull: ::std::iter::Iterator<Item = Item>,
                                    Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    Item,
                                >(pull: Pull, push: Push) {
                                    ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                                }
                                pivot_run_sg_5v1(op_9v1, op_10v1);
                            },
                        );
                    let sgid_6v1 = df
                        .add_subgraph_full(
                            "Subgraph GraphSubgraphId(6v1)",
                            2,
                            (hoff_23v1_recv, (hoff_25v1_recv, ())),
                            (hoff_26v1_send, ()),
                            false,
                            Some(loop_2v1),
                            move |
                                context,
                                (hoff_23v1_recv, (hoff_25v1_recv, ())),
                                (hoff_26v1_send, ())|
                            {
                                let mut hoff_23v1_recv = hoff_23v1_recv.borrow_mut_swap();
                                let hoff_23v1_recv = hoff_23v1_recv.drain(..);
                                let mut hoff_25v1_recv = hoff_25v1_recv.borrow_mut_swap();
                                let hoff_25v1_recv = hoff_25v1_recv.drain(..);
                                let hoff_26v1_send = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                    v|
                                {
                                    hoff_26v1_send.give(Some(v));
                                });
                                let mut sg_6v1_node_7v1_vec = unsafe {
                                    context.state_ref_unchecked(singleton_op_7v1)
                                }
                                    .borrow_mut();
                                if 0 == context.loop_iter_count() {
                                    *sg_6v1_node_7v1_vec = hoff_23v1_recv
                                        .collect::<::std::vec::Vec<_>>();
                                }
                                let op_7v1 = std::iter::IntoIterator::into_iter(
                                    ::std::clone::Clone::clone(&*sg_6v1_node_7v1_vec),
                                );
                                let op_7v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_7v1__repeat_n__loc_dfir_rs_tests_surface_loop_rs_294_24_294_36<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_7v1__repeat_n__loc_dfir_rs_tests_surface_loop_rs_294_24_294_36(
                                        op_7v1,
                                    )
                                };
                                let op_7v1 = op_7v1.map(|a| ((), a));
                                let hoff_25v1_recv = hoff_25v1_recv.map(|b| ((), b));
                                let mut sg_6v1_node_11v1_joindata_lhs_borrow = ::std::default::Default::default();
                                let mut sg_6v1_node_11v1_joindata_rhs_borrow = ::std::default::Default::default();
                                let op_11v1 = {
                                    #[inline(always)]
                                    fn check_inputs<'a, K, I1, V1, I2, V2>(
                                        lhs: I1,
                                        rhs: I2,
                                        lhs_state: &'a mut ::dfir_rs::compiled::pull::HalfMultisetJoinState<
                                            K,
                                            V1,
                                            V2,
                                        >,
                                        rhs_state: &'a mut ::dfir_rs::compiled::pull::HalfMultisetJoinState<
                                            K,
                                            V2,
                                            V1,
                                        >,
                                        is_new_tick: bool,
                                    ) -> impl 'a + Iterator<Item = (K, (V1, V2))>
                                    where
                                        K: Eq + std::hash::Hash + Clone,
                                        V1: Clone,
                                        V2: Clone,
                                        I1: 'a + Iterator<Item = (K, V1)>,
                                        I2: 'a + Iterator<Item = (K, V2)>,
                                    {
                                        op_11v1__cross_join_multiset__loc_dfir_rs_tests_surface_loop_rs_301_22_301_43(||
                                        ::dfir_rs::compiled::pull::symmetric_hash_join_into_iter(
                                            lhs,
                                            rhs,
                                            lhs_state,
                                            rhs_state,
                                            is_new_tick,
                                        ))
                                    }
                                    check_inputs(
                                        op_7v1,
                                        hoff_25v1_recv,
                                        &mut sg_6v1_node_11v1_joindata_lhs_borrow,
                                        &mut sg_6v1_node_11v1_joindata_rhs_borrow,
                                        true,
                                    )
                                };
                                let op_11v1 = op_11v1.map(|((), (a, b))| (a, b));
                                let op_11v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_11v1__cross_join_multiset__loc_dfir_rs_tests_surface_loop_rs_301_22_301_43<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_11v1__cross_join_multiset__loc_dfir_rs_tests_surface_loop_rs_301_22_301_43(
                                        op_11v1,
                                    )
                                };
                                #[allow(
                                    clippy::map_clone,
                                    reason = "dfir has no explicit `cloned`/`copied` operator"
                                )]
                                let op_12v1 = op_11v1
                                    .map(|(point, centroid): ([i32; 2], [i32; 2])| {
                                        let dist2 = (point[0] - centroid[0]).pow(2)
                                            + (point[1] - centroid[1]).pow(2);
                                        (point, (dist2, centroid))
                                    });
                                let op_12v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_12v1__map__loc_dfir_rs_tests_surface_loop_rs_302_24_305_23<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_12v1__map__loc_dfir_rs_tests_surface_loop_rs_302_24_305_23(
                                        op_12v1,
                                    )
                                };
                                let mut sg_6v1_node_13v1_hashtable = unsafe {
                                    context.state_ref_unchecked(sg_6v1_node_13v1_groupbydata)
                                }
                                    .borrow_mut();
                                op_13v1__reduce_keyed__loc_dfir_rs_tests_surface_loop_rs_306_24_311_23(||
                                {
                                    #[inline(always)]
                                    fn check_input<Iter, K, V>(
                                        iter: Iter,
                                    ) -> impl ::std::iter::Iterator<Item = (K, V)>
                                    where
                                        Iter: std::iter::Iterator<Item = (K, V)>,
                                        K: ::std::clone::Clone,
                                        V: ::std::clone::Clone,
                                    {
                                        iter
                                    }
                                    /// A: accumulator/item type
                                    /// O: output type
                                    #[inline(always)]
                                    fn call_comb_type<A, O>(
                                        acc: &mut A,
                                        item: A,
                                        f: impl Fn(&mut A, A) -> O,
                                    ) -> O {
                                        (f)(acc, item)
                                    }
                                    for kv in check_input(op_12v1) {
                                        match sg_6v1_node_13v1_hashtable.entry(kv.0) {
                                            ::std::collections::hash_map::Entry::Vacant(vacant) => {
                                                vacant.insert(kv.1);
                                            }
                                            ::std::collections::hash_map::Entry::Occupied(
                                                mut occupied,
                                            ) => {
                                                call_comb_type(
                                                    occupied.get_mut(),
                                                    kv.1,
                                                    |(a_dist2, a_centroid), (b_dist2, b_centroid)| {
                                                        if b_dist2 < *a_dist2 {
                                                            *a_dist2 = b_dist2;
                                                            *a_centroid = b_centroid;
                                                        }
                                                    },
                                                );
                                            }
                                        }
                                    }
                                });
                                let op_13v1 = sg_6v1_node_13v1_hashtable.drain();
                                let op_13v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_13v1__reduce_keyed__loc_dfir_rs_tests_surface_loop_rs_306_24_311_23<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_13v1__reduce_keyed__loc_dfir_rs_tests_surface_loop_rs_306_24_311_23(
                                        op_13v1,
                                    )
                                };
                                #[allow(
                                    clippy::map_clone,
                                    reason = "dfir has no explicit `cloned`/`copied` operator"
                                )]
                                let op_14v1 = op_13v1
                                    .map(|(point, (_dist2, centroid))| {
                                        (centroid, (point, 1))
                                    });
                                let op_14v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_14v1__map__loc_dfir_rs_tests_surface_loop_rs_312_24_314_23<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_14v1__map__loc_dfir_rs_tests_surface_loop_rs_312_24_314_23(
                                        op_14v1,
                                    )
                                };
                                let mut sg_6v1_node_15v1_hashtable = unsafe {
                                    context.state_ref_unchecked(sg_6v1_node_15v1_groupbydata)
                                }
                                    .borrow_mut();
                                op_15v1__reduce_keyed__loc_dfir_rs_tests_surface_loop_rs_315_24_319_23(||
                                {
                                    #[inline(always)]
                                    fn check_input<Iter, K, V>(
                                        iter: Iter,
                                    ) -> impl ::std::iter::Iterator<Item = (K, V)>
                                    where
                                        Iter: std::iter::Iterator<Item = (K, V)>,
                                        K: ::std::clone::Clone,
                                        V: ::std::clone::Clone,
                                    {
                                        iter
                                    }
                                    /// A: accumulator/item type
                                    /// O: output type
                                    #[inline(always)]
                                    fn call_comb_type<A, O>(
                                        acc: &mut A,
                                        item: A,
                                        f: impl Fn(&mut A, A) -> O,
                                    ) -> O {
                                        (f)(acc, item)
                                    }
                                    for kv in check_input(op_14v1) {
                                        match sg_6v1_node_15v1_hashtable.entry(kv.0) {
                                            ::std::collections::hash_map::Entry::Vacant(vacant) => {
                                                vacant.insert(kv.1);
                                            }
                                            ::std::collections::hash_map::Entry::Occupied(
                                                mut occupied,
                                            ) => {
                                                call_comb_type(
                                                    occupied.get_mut(),
                                                    kv.1,
                                                    |(p1, n1), (p2, n2): ([i32; 2], i32)| {
                                                        p1[0] += p2[0];
                                                        p1[1] += p2[1];
                                                        *n1 += n2;
                                                    },
                                                );
                                            }
                                        }
                                    }
                                });
                                let op_15v1 = sg_6v1_node_15v1_hashtable.drain();
                                let op_15v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_15v1__reduce_keyed__loc_dfir_rs_tests_surface_loop_rs_315_24_319_23<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_15v1__reduce_keyed__loc_dfir_rs_tests_surface_loop_rs_315_24_319_23(
                                        op_15v1,
                                    )
                                };
                                #[allow(
                                    clippy::map_clone,
                                    reason = "dfir has no explicit `cloned`/`copied` operator"
                                )]
                                let op_16v1 = op_15v1
                                    .map(|(_centroid, (p, n)): (_, ([i32; 2], i32))| {
                                        [p[0] / n, p[1] / n]
                                    });
                                let op_16v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_16v1__map__loc_dfir_rs_tests_surface_loop_rs_320_24_322_23<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_16v1__map__loc_dfir_rs_tests_surface_loop_rs_320_24_322_23(
                                        op_16v1,
                                    )
                                };
                                #[inline(always)]
                                fn pivot_run_sg_6v1<
                                    Pull: ::std::iter::Iterator<Item = Item>,
                                    Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    Item,
                                >(pull: Pull, push: Push) {
                                    ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                                }
                                pivot_run_sg_6v1(op_16v1, hoff_26v1_send);
                                {
                                    if context.loop_iter_count() + 1 < 10 {
                                        context.reschedule_loop_block();
                                    }
                                }
                            },
                        );
                    let sgid_7v1 = df
                        .add_subgraph_full(
                            "Subgraph GraphSubgraphId(7v1)",
                            3,
                            (hoff_27v1_recv, ()),
                            (),
                            false,
                            Some(loop_1v1),
                            move |context, (hoff_27v1_recv, ()), ()| {
                                let mut hoff_27v1_recv = hoff_27v1_recv.borrow_mut_swap();
                                let hoff_27v1_recv = hoff_27v1_recv.drain(..);
                                let op_19v1 = {
                                    fn check_input<
                                        Iter: ::std::iter::Iterator<Item = Item>,
                                        Item,
                                    >(iter: Iter) -> impl ::std::iter::Iterator<Item = Item> {
                                        iter
                                    }
                                    check_input::<_, _>(hoff_27v1_recv)
                                };
                                let op_19v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_19v1__all_iterations__loc_dfir_rs_tests_surface_loop_rs_329_20_329_36<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Pull<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::std::iter::Iterator<Item = Item>,
                                        > Iterator for Pull<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn next(&mut self) -> Option<Self::Item> {
                                                self.inner.next()
                                            }
                                            #[inline(always)]
                                            fn size_hint(&self) -> (usize, Option<usize>) {
                                                self.inner.size_hint()
                                            }
                                        }
                                        Pull { inner: input }
                                    }
                                    op_19v1__all_iterations__loc_dfir_rs_tests_surface_loop_rs_329_20_329_36(
                                        op_19v1,
                                    )
                                };
                                let op_20v1 = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                    x|
                                result_send.send(x).unwrap());
                                let op_20v1 = {
                                    #[allow(non_snake_case)]
                                    #[inline(always)]
                                    pub fn op_20v1__for_each__loc_dfir_rs_tests_surface_loop_rs_330_20_330_62<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    >(
                                        input: Input,
                                    ) -> impl ::dfir_rs::pusherator::Pusherator<Item = Item> {
                                        #[repr(transparent)]
                                        struct Push<
                                            Item,
                                            Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                        > {
                                            inner: Input,
                                        }
                                        impl<
                                            Item,
                                            Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                        > ::dfir_rs::pusherator::Pusherator for Push<Item, Input> {
                                            type Item = Item;
                                            #[inline(always)]
                                            fn give(&mut self, item: Self::Item) {
                                                self.inner.give(item)
                                            }
                                        }
                                        Push { inner: input }
                                    }
                                    op_20v1__for_each__loc_dfir_rs_tests_surface_loop_rs_330_20_330_62(
                                        op_20v1,
                                    )
                                };
                                #[inline(always)]
                                fn pivot_run_sg_7v1<
                                    Pull: ::std::iter::Iterator<Item = Item>,
                                    Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    Item,
                                >(pull: Pull, push: Push) {
                                    ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                                }
                                pivot_run_sg_7v1(op_19v1, op_20v1);
                            },
                        );
                    df
                }
            }
        };
        {
            #[cfg(not(target_arch = "wasm32"))]
            {
                {
                    let mut settings = ::insta::Settings::clone_current();
                    settings._private_inner_mut().snapshot_suffix("graphvis_mermaid");
                    settings
                        .bind(|| {
                            ::insta::_macro_support::assert_snapshot(
                                    ::insta::_macro_support::AutoName.into(),
                                    #[allow(clippy::redundant_closure_call)]
                                    &(|v| ::alloc::__export::must_use({
                                        let res = ::alloc::fmt::format(format_args!("{0}", v));
                                        res
                                    }))(
                                        &df.meta_graph().unwrap().to_mermaid(&Default::default()),
                                    ),
                                    "F:\\Projects\\hydroflow\\dfir_rs",
                                    {
                                        fn f() {}
                                        fn type_name_of_val<T>(_: T) -> &'static str {
                                            std::any::type_name::<T>()
                                        }
                                        let mut name = type_name_of_val(f)
                                            .strip_suffix("::f")
                                            .unwrap_or("");
                                        while let Some(rest) = name.strip_suffix("::{{closure}}") {
                                            name = rest;
                                        }
                                        name
                                    },
                                    "surface_loop",
                                    "dfir_rs\\tests\\surface_loop.rs",
                                    333u32,
                                    "df.meta_graph().unwrap().to_mermaid(& Default :: default())",
                                )
                                .unwrap();
                        })
                };
                {
                    let mut settings = ::insta::Settings::clone_current();
                    settings._private_inner_mut().snapshot_suffix("graphvis_dot");
                    settings
                        .bind(|| {
                            ::insta::_macro_support::assert_snapshot(
                                    ::insta::_macro_support::AutoName.into(),
                                    #[allow(clippy::redundant_closure_call)]
                                    &(|v| ::alloc::__export::must_use({
                                        let res = ::alloc::fmt::format(format_args!("{0}", v));
                                        res
                                    }))(&df.meta_graph().unwrap().to_dot(&Default::default())),
                                    "F:\\Projects\\hydroflow\\dfir_rs",
                                    {
                                        fn f() {}
                                        fn type_name_of_val<T>(_: T) -> &'static str {
                                            std::any::type_name::<T>()
                                        }
                                        let mut name = type_name_of_val(f)
                                            .strip_suffix("::f")
                                            .unwrap_or("");
                                        while let Some(rest) = name.strip_suffix("::{{closure}}") {
                                            name = rest;
                                        }
                                        name
                                    },
                                    "surface_loop",
                                    "dfir_rs\\tests\\surface_loop.rs",
                                    333u32,
                                    "df.meta_graph().unwrap().to_dot(& Default :: default())",
                                )
                                .unwrap();
                        })
                };
            }
        };
        df.run_available();
        let mut result = collect_ready::<Vec<_>, _>(&mut result_recv);
        let n = result.len();
        let last = &mut result[n - 3..];
        last.sort_unstable();
        match (&&[[-231, -118], [-103, 97], [168, -6]], &last) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
}
extern crate test;
#[cfg(test)]
#[rustc_test_marker = "test_loop_lifetime_reduce"]
#[doc(hidden)]
pub const test_loop_lifetime_reduce: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_loop_lifetime_reduce"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "dfir_rs\\tests\\surface_loop.rs",
        start_line: 344usize,
        start_col: 4usize,
        end_line: 344usize,
        end_col: 29usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_loop_lifetime_reduce()),
    ),
};
fn test_loop_lifetime_reduce() {
    let (result1_send, mut result1_recv) = dfir_rs::util::unbounded_channel::<_>();
    let (result2_send, mut result2_recv) = dfir_rs::util::unbounded_channel::<_>();
    let mut df = {
        {
            #[allow(unused_qualifications)]
            {
                use ::dfir_rs::{var_expr, var_args};
                let mut df = ::dfir_rs::scheduled::graph::Dfir::new();
                df.__assign_meta_graph(
                    "{\"nodes\":[{\"value\":null,\"version\":0},{\"value\":{\"Operator\":\"source_iter(0 .. 10)\"},\"version\":1},{\"value\":{\"Operator\":\"batch()\"},\"version\":1},{\"value\":{\"Operator\":\"tee()\"},\"version\":1},{\"value\":{\"Operator\":\"repeat_n(5)\"},\"version\":1},{\"value\":{\"Operator\":\"reduce :: < 'none > (| old : & mut _, val | { * old += val; })\"},\"version\":1},{\"value\":{\"Operator\":\"for_each(| v | result1_send.send(v).unwrap())\"},\"version\":1},{\"value\":{\"Operator\":\"repeat_n(5)\"},\"version\":1},{\"value\":{\"Operator\":\"reduce :: < 'loop > (| old : & mut _, val | { * old += val; })\"},\"version\":1},{\"value\":{\"Operator\":\"for_each(| v | result2_send.send(v).unwrap())\"},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1}],\"operator_tag\":[{\"value\":null,\"version\":0}],\"graph\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":2,\"version\":1},{\"idx\":3,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":1,\"version\":1},{\"idx\":10,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":5,\"version\":1},{\"idx\":6,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":4,\"version\":1},{\"idx\":5,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":3,\"version\":1},{\"idx\":11,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":8,\"version\":1},{\"idx\":9,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":7,\"version\":1},{\"idx\":8,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":3,\"version\":1},{\"idx\":12,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":10,\"version\":1},{\"idx\":2,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":11,\"version\":1},{\"idx\":4,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":12,\"version\":1},{\"idx\":7,\"version\":1}],\"version\":1}],\"ports\":[{\"value\":null,\"version\":0},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1}],\"node_loops\":[{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1}],\"loop_nodes\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":2,\"version\":1},{\"idx\":3,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":4,\"version\":1},{\"idx\":5,\"version\":1},{\"idx\":6,\"version\":1},{\"idx\":7,\"version\":1},{\"idx\":8,\"version\":1},{\"idx\":9,\"version\":1}],\"version\":1}],\"loop_parent\":[{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":{\"idx\":1,\"version\":1},\"version\":1}],\"root_loops\":[{\"idx\":1,\"version\":1}],\"loop_children\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":2,\"version\":1}],\"version\":1},{\"value\":[],\"version\":1}],\"node_subgraph\":[{\"value\":null,\"version\":0},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1},{\"value\":{\"idx\":4,\"version\":1},\"version\":1},{\"value\":{\"idx\":4,\"version\":1},\"version\":1},{\"value\":{\"idx\":4,\"version\":1},\"version\":1}],\"subgraph_nodes\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":1,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":2,\"version\":1},{\"idx\":3,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":4,\"version\":1},{\"idx\":5,\"version\":1},{\"idx\":6,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":7,\"version\":1},{\"idx\":8,\"version\":1},{\"idx\":9,\"version\":1}],\"version\":1}],\"subgraph_stratum\":[{\"value\":null,\"version\":0},{\"value\":0,\"version\":1},{\"value\":1,\"version\":1},{\"value\":2,\"version\":1},{\"value\":2,\"version\":1}],\"node_singleton_references\":[{\"value\":null,\"version\":0},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1}],\"node_varnames\":[{\"value\":null,\"version\":0},{\"value\":\"a\",\"version\":1},{\"value\":\"b\",\"version\":1},{\"value\":\"b\",\"version\":1}],\"subgraph_laziness\":[{\"value\":null,\"version\":0}]}",
                );
                df.__assign_diagnostics("[]");
                let (hoff_10v1_send, hoff_10v1_recv) = df
                    .make_edge::<
                        _,
                        ::dfir_rs::scheduled::handoff::VecHandoff<_>,
                    >("handoff GraphNodeId(10v1)");
                let (hoff_11v1_send, hoff_11v1_recv) = df
                    .make_edge::<
                        _,
                        ::dfir_rs::scheduled::handoff::VecHandoff<_>,
                    >("handoff GraphNodeId(11v1)");
                let (hoff_12v1_send, hoff_12v1_recv) = df
                    .make_edge::<
                        _,
                        ::dfir_rs::scheduled::handoff::VecHandoff<_>,
                    >("handoff GraphNodeId(12v1)");
                let loop_1v1 = df.add_loop(None);
                let loop_2v1 = df.add_loop(Some(loop_1v1));
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_1v1__source_iter__loc_dfir_rs_tests_surface_loop_rs_349_13_349_31<
                    T,
                >(thunk: impl FnOnce() -> T) -> T {
                    thunk()
                }
                let mut sg_1v1_node_1v1_iter = {
                    #[inline(always)]
                    fn check_iter<
                        IntoIter: ::std::iter::IntoIterator<Item = Item>,
                        Item,
                    >(into_iter: IntoIter) -> impl ::std::iter::Iterator<Item = Item> {
                        ::std::iter::IntoIterator::into_iter(into_iter)
                    }
                    check_iter(0..10)
                };
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_2v1__batch__loc_dfir_rs_tests_surface_loop_rs_351_22_351_29<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_3v1__tee__loc_dfir_rs_tests_surface_loop_rs_351_33_351_38<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_4v1__repeat_n__loc_dfir_rs_tests_surface_loop_rs_353_22_353_33<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(clippy::redundant_closure_call)]
                let singleton_op_4v1 = df
                    .add_state(::std::cell::RefCell::new(::std::vec::Vec::new()));
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_5v1__reduce__loc_dfir_rs_tests_surface_loop_rs_354_24_356_23<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                let singleton_op_5v1 = df
                    .add_state(::std::cell::RefCell::new(::std::option::Option::None));
                df.set_state_lifespan_hook(
                    singleton_op_5v1,
                    ::dfir_rs::scheduled::graph::StateLifespan::Subgraph(sgid_3v1),
                    move |rcell| {
                        rcell.replace(::std::option::Option::None);
                    },
                );
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_6v1__for_each__loc_dfir_rs_tests_surface_loop_rs_357_24_357_67<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_7v1__repeat_n__loc_dfir_rs_tests_surface_loop_rs_359_22_359_33<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(clippy::redundant_closure_call)]
                let singleton_op_7v1 = df
                    .add_state(::std::cell::RefCell::new(::std::vec::Vec::new()));
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_8v1__reduce__loc_dfir_rs_tests_surface_loop_rs_360_24_362_23<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                let singleton_op_8v1 = df
                    .add_state(::std::cell::RefCell::new(::std::option::Option::None));
                df.set_state_lifespan_hook(
                    singleton_op_8v1,
                    ::dfir_rs::scheduled::graph::StateLifespan::Loop(loop_2v1),
                    move |rcell| {
                        rcell.replace(::std::option::Option::None);
                    },
                );
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_9v1__for_each__loc_dfir_rs_tests_surface_loop_rs_363_24_363_67<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                let sgid_1v1 = df
                    .add_subgraph_full(
                        "Subgraph GraphSubgraphId(1v1)",
                        0,
                        (),
                        (hoff_10v1_send, ()),
                        false,
                        None,
                        move |context, (), (hoff_10v1_send, ())| {
                            let hoff_10v1_send = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                v|
                            {
                                hoff_10v1_send.give(Some(v));
                            });
                            let op_1v1 = sg_1v1_node_1v1_iter.by_ref();
                            let op_1v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_1v1__source_iter__loc_dfir_rs_tests_surface_loop_rs_349_13_349_31<
                                    Item,
                                    Input: ::std::iter::Iterator<Item = Item>,
                                >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Pull<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > Iterator for Pull<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item> {
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize, Option<usize>) {
                                            self.inner.size_hint()
                                        }
                                    }
                                    Pull { inner: input }
                                }
                                op_1v1__source_iter__loc_dfir_rs_tests_surface_loop_rs_349_13_349_31(
                                    op_1v1,
                                )
                            };
                            #[inline(always)]
                            fn pivot_run_sg_1v1<
                                Pull: ::std::iter::Iterator<Item = Item>,
                                Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                Item,
                            >(pull: Pull, push: Push) {
                                ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                            }
                            pivot_run_sg_1v1(op_1v1, hoff_10v1_send);
                        },
                    );
                let sgid_2v1 = df
                    .add_subgraph_full(
                        "Subgraph GraphSubgraphId(2v1)",
                        1,
                        (hoff_10v1_recv, ()),
                        (hoff_11v1_send, (hoff_12v1_send, ())),
                        false,
                        Some(loop_1v1),
                        move |
                            context,
                            (hoff_10v1_recv, ()),
                            (hoff_11v1_send, (hoff_12v1_send, ()))|
                        {
                            let mut hoff_10v1_recv = hoff_10v1_recv.borrow_mut_swap();
                            let hoff_10v1_recv = hoff_10v1_recv.drain(..);
                            let hoff_11v1_send = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                v|
                            {
                                hoff_11v1_send.give(Some(v));
                            });
                            let hoff_12v1_send = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                v|
                            {
                                hoff_12v1_send.give(Some(v));
                            });
                            let op_2v1 = {
                                fn check_input<
                                    Iter: ::std::iter::Iterator<Item = Item>,
                                    Item,
                                >(iter: Iter) -> impl ::std::iter::Iterator<Item = Item> {
                                    iter
                                }
                                check_input::<_, _>(hoff_10v1_recv)
                            };
                            let op_2v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_2v1__batch__loc_dfir_rs_tests_surface_loop_rs_351_22_351_29<
                                    Item,
                                    Input: ::std::iter::Iterator<Item = Item>,
                                >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Pull<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > Iterator for Pull<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item> {
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize, Option<usize>) {
                                            self.inner.size_hint()
                                        }
                                    }
                                    Pull { inner: input }
                                }
                                op_2v1__batch__loc_dfir_rs_tests_surface_loop_rs_351_22_351_29(
                                    op_2v1,
                                )
                            };
                            let op_3v1 = ::dfir_rs::pusherator::tee::Tee::new(
                                hoff_11v1_send,
                                hoff_12v1_send,
                            );
                            let op_3v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_3v1__tee__loc_dfir_rs_tests_surface_loop_rs_351_33_351_38<
                                    Item,
                                    Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                >(
                                    input: Input,
                                ) -> impl ::dfir_rs::pusherator::Pusherator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Push<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    > ::dfir_rs::pusherator::Pusherator for Push<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn give(&mut self, item: Self::Item) {
                                            self.inner.give(item)
                                        }
                                    }
                                    Push { inner: input }
                                }
                                op_3v1__tee__loc_dfir_rs_tests_surface_loop_rs_351_33_351_38(
                                    op_3v1,
                                )
                            };
                            #[inline(always)]
                            fn pivot_run_sg_2v1<
                                Pull: ::std::iter::Iterator<Item = Item>,
                                Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                Item,
                            >(pull: Pull, push: Push) {
                                ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                            }
                            pivot_run_sg_2v1(op_2v1, op_3v1);
                            context.allow_another_iteration();
                        },
                    );
                let sgid_3v1 = df
                    .add_subgraph_full(
                        "Subgraph GraphSubgraphId(3v1)",
                        2,
                        (hoff_11v1_recv, ()),
                        (),
                        false,
                        Some(loop_2v1),
                        move |context, (hoff_11v1_recv, ()), ()| {
                            let mut hoff_11v1_recv = hoff_11v1_recv.borrow_mut_swap();
                            let hoff_11v1_recv = hoff_11v1_recv.drain(..);
                            let mut sg_3v1_node_4v1_vec = unsafe {
                                context.state_ref_unchecked(singleton_op_4v1)
                            }
                                .borrow_mut();
                            if 0 == context.loop_iter_count() {
                                *sg_3v1_node_4v1_vec = hoff_11v1_recv
                                    .collect::<::std::vec::Vec<_>>();
                            }
                            let op_4v1 = std::iter::IntoIterator::into_iter(
                                ::std::clone::Clone::clone(&*sg_3v1_node_4v1_vec),
                            );
                            let op_4v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_4v1__repeat_n__loc_dfir_rs_tests_surface_loop_rs_353_22_353_33<
                                    Item,
                                    Input: ::std::iter::Iterator<Item = Item>,
                                >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Pull<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > Iterator for Pull<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item> {
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize, Option<usize>) {
                                            self.inner.size_hint()
                                        }
                                    }
                                    Pull { inner: input }
                                }
                                op_4v1__repeat_n__loc_dfir_rs_tests_surface_loop_rs_353_22_353_33(
                                    op_4v1,
                                )
                            };
                            let op_5v1 = {
                                #[allow(unused_mut)]
                                let mut sg_3v1_node_5v1_accumulator = unsafe {
                                    context.state_ref_unchecked(singleton_op_5v1)
                                }
                                    .borrow_mut();
                                op_5v1__reduce__loc_dfir_rs_tests_surface_loop_rs_354_24_356_23(||
                                {
                                    op_4v1
                                        .for_each(|sg_3v1_node_5v1_iterator_item| {
                                            #[inline(always)]
                                            fn call_comb_type<Item>(
                                                accum: &mut Option<Item>,
                                                item: Item,
                                                func: impl Fn(&mut Item, Item),
                                            ) {
                                                match accum {
                                                    accum @ None => *accum = Some(item),
                                                    Some(accum) => (func)(accum, item),
                                                }
                                            }
                                            #[allow(clippy::redundant_closure_call)]
                                            call_comb_type(
                                                &mut *sg_3v1_node_5v1_accumulator,
                                                sg_3v1_node_5v1_iterator_item,
                                                |old: &mut _, val| {
                                                    *old += val;
                                                },
                                            );
                                        })
                                });
                                #[allow(clippy::clone_on_copy)]
                                {
                                    ::std::iter::IntoIterator::into_iter(
                                        op_5v1__reduce__loc_dfir_rs_tests_surface_loop_rs_354_24_356_23(||
                                        ::std::clone::Clone::clone(&*sg_3v1_node_5v1_accumulator)),
                                    )
                                }
                            };
                            let op_5v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_5v1__reduce__loc_dfir_rs_tests_surface_loop_rs_354_24_356_23<
                                    Item,
                                    Input: ::std::iter::Iterator<Item = Item>,
                                >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Pull<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > Iterator for Pull<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item> {
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize, Option<usize>) {
                                            self.inner.size_hint()
                                        }
                                    }
                                    Pull { inner: input }
                                }
                                op_5v1__reduce__loc_dfir_rs_tests_surface_loop_rs_354_24_356_23(
                                    op_5v1,
                                )
                            };
                            let op_6v1 = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                v|
                            result1_send.send(v).unwrap());
                            let op_6v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_6v1__for_each__loc_dfir_rs_tests_surface_loop_rs_357_24_357_67<
                                    Item,
                                    Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                >(
                                    input: Input,
                                ) -> impl ::dfir_rs::pusherator::Pusherator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Push<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    > ::dfir_rs::pusherator::Pusherator for Push<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn give(&mut self, item: Self::Item) {
                                            self.inner.give(item)
                                        }
                                    }
                                    Push { inner: input }
                                }
                                op_6v1__for_each__loc_dfir_rs_tests_surface_loop_rs_357_24_357_67(
                                    op_6v1,
                                )
                            };
                            #[inline(always)]
                            fn pivot_run_sg_3v1<
                                Pull: ::std::iter::Iterator<Item = Item>,
                                Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                Item,
                            >(pull: Pull, push: Push) {
                                ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                            }
                            pivot_run_sg_3v1(op_5v1, op_6v1);
                            {
                                if context.loop_iter_count() + 1 < 5 {
                                    context.reschedule_loop_block();
                                }
                            }
                        },
                    );
                let sgid_4v1 = df
                    .add_subgraph_full(
                        "Subgraph GraphSubgraphId(4v1)",
                        2,
                        (hoff_12v1_recv, ()),
                        (),
                        false,
                        Some(loop_2v1),
                        move |context, (hoff_12v1_recv, ()), ()| {
                            let mut hoff_12v1_recv = hoff_12v1_recv.borrow_mut_swap();
                            let hoff_12v1_recv = hoff_12v1_recv.drain(..);
                            let mut sg_4v1_node_7v1_vec = unsafe {
                                context.state_ref_unchecked(singleton_op_7v1)
                            }
                                .borrow_mut();
                            if 0 == context.loop_iter_count() {
                                *sg_4v1_node_7v1_vec = hoff_12v1_recv
                                    .collect::<::std::vec::Vec<_>>();
                            }
                            let op_7v1 = std::iter::IntoIterator::into_iter(
                                ::std::clone::Clone::clone(&*sg_4v1_node_7v1_vec),
                            );
                            let op_7v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_7v1__repeat_n__loc_dfir_rs_tests_surface_loop_rs_359_22_359_33<
                                    Item,
                                    Input: ::std::iter::Iterator<Item = Item>,
                                >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Pull<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > Iterator for Pull<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item> {
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize, Option<usize>) {
                                            self.inner.size_hint()
                                        }
                                    }
                                    Pull { inner: input }
                                }
                                op_7v1__repeat_n__loc_dfir_rs_tests_surface_loop_rs_359_22_359_33(
                                    op_7v1,
                                )
                            };
                            let op_8v1 = {
                                #[allow(unused_mut)]
                                let mut sg_4v1_node_8v1_accumulator = unsafe {
                                    context.state_ref_unchecked(singleton_op_8v1)
                                }
                                    .borrow_mut();
                                op_8v1__reduce__loc_dfir_rs_tests_surface_loop_rs_360_24_362_23(||
                                {
                                    op_7v1
                                        .for_each(|sg_4v1_node_8v1_iterator_item| {
                                            #[inline(always)]
                                            fn call_comb_type<Item>(
                                                accum: &mut Option<Item>,
                                                item: Item,
                                                func: impl Fn(&mut Item, Item),
                                            ) {
                                                match accum {
                                                    accum @ None => *accum = Some(item),
                                                    Some(accum) => (func)(accum, item),
                                                }
                                            }
                                            #[allow(clippy::redundant_closure_call)]
                                            call_comb_type(
                                                &mut *sg_4v1_node_8v1_accumulator,
                                                sg_4v1_node_8v1_iterator_item,
                                                |old: &mut _, val| {
                                                    *old += val;
                                                },
                                            );
                                        })
                                });
                                #[allow(clippy::clone_on_copy)]
                                {
                                    ::std::iter::IntoIterator::into_iter(
                                        op_8v1__reduce__loc_dfir_rs_tests_surface_loop_rs_360_24_362_23(||
                                        ::std::clone::Clone::clone(&*sg_4v1_node_8v1_accumulator)),
                                    )
                                }
                            };
                            let op_8v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_8v1__reduce__loc_dfir_rs_tests_surface_loop_rs_360_24_362_23<
                                    Item,
                                    Input: ::std::iter::Iterator<Item = Item>,
                                >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Pull<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > Iterator for Pull<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item> {
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize, Option<usize>) {
                                            self.inner.size_hint()
                                        }
                                    }
                                    Pull { inner: input }
                                }
                                op_8v1__reduce__loc_dfir_rs_tests_surface_loop_rs_360_24_362_23(
                                    op_8v1,
                                )
                            };
                            let op_9v1 = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                v|
                            result2_send.send(v).unwrap());
                            let op_9v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_9v1__for_each__loc_dfir_rs_tests_surface_loop_rs_363_24_363_67<
                                    Item,
                                    Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                >(
                                    input: Input,
                                ) -> impl ::dfir_rs::pusherator::Pusherator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Push<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    > ::dfir_rs::pusherator::Pusherator for Push<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn give(&mut self, item: Self::Item) {
                                            self.inner.give(item)
                                        }
                                    }
                                    Push { inner: input }
                                }
                                op_9v1__for_each__loc_dfir_rs_tests_surface_loop_rs_363_24_363_67(
                                    op_9v1,
                                )
                            };
                            #[inline(always)]
                            fn pivot_run_sg_4v1<
                                Pull: ::std::iter::Iterator<Item = Item>,
                                Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                Item,
                            >(pull: Pull, push: Push) {
                                ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                            }
                            pivot_run_sg_4v1(op_8v1, op_9v1);
                            {
                                if context.loop_iter_count() + 1 < 5 {
                                    context.reschedule_loop_block();
                                }
                            }
                        },
                    );
                df
            }
        }
    };
    df.run_available();
    match (&&[45, 45, 45, 45, 45], &&*collect_ready::<Vec<_>, _>(&mut result1_recv)) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    match (&&[45, 90, 135, 180, 225], &&*collect_ready::<Vec<_>, _>(&mut result2_recv)) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
}
extern crate test;
#[cfg(test)]
#[rustc_test_marker = "test_loop_lifetime_fold"]
#[doc(hidden)]
pub const test_loop_lifetime_fold: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_loop_lifetime_fold"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "dfir_rs\\tests\\surface_loop.rs",
        start_line: 382usize,
        start_col: 4usize,
        end_line: 382usize,
        end_col: 27usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_loop_lifetime_fold()),
    ),
};
fn test_loop_lifetime_fold() {
    let (result1_send, mut result1_recv) = dfir_rs::util::unbounded_channel::<_>();
    let (result2_send, mut result2_recv) = dfir_rs::util::unbounded_channel::<_>();
    let mut df = {
        {
            #[allow(unused_qualifications)]
            {
                use ::dfir_rs::{var_expr, var_args};
                let mut df = ::dfir_rs::scheduled::graph::Dfir::new();
                df.__assign_meta_graph(
                    "{\"nodes\":[{\"value\":null,\"version\":0},{\"value\":{\"Operator\":\"source_iter(0 .. 10)\"},\"version\":1},{\"value\":{\"Operator\":\"batch()\"},\"version\":1},{\"value\":{\"Operator\":\"tee()\"},\"version\":1},{\"value\":{\"Operator\":\"repeat_n(5)\"},\"version\":1},{\"value\":{\"Operator\":\"fold :: < 'none > (| | 10000, | old : & mut _, val | { * old += val; })\"},\"version\":1},{\"value\":{\"Operator\":\"for_each(| v | result1_send.send(v).unwrap())\"},\"version\":1},{\"value\":{\"Operator\":\"repeat_n(5)\"},\"version\":1},{\"value\":{\"Operator\":\"fold :: < 'loop > (| | 10000, | old : & mut _, val | { * old += val; })\"},\"version\":1},{\"value\":{\"Operator\":\"for_each(| v | result2_send.send(v).unwrap())\"},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1}],\"operator_tag\":[{\"value\":null,\"version\":0}],\"graph\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":2,\"version\":1},{\"idx\":3,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":1,\"version\":1},{\"idx\":10,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":5,\"version\":1},{\"idx\":6,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":4,\"version\":1},{\"idx\":5,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":3,\"version\":1},{\"idx\":11,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":8,\"version\":1},{\"idx\":9,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":7,\"version\":1},{\"idx\":8,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":3,\"version\":1},{\"idx\":12,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":10,\"version\":1},{\"idx\":2,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":11,\"version\":1},{\"idx\":4,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":12,\"version\":1},{\"idx\":7,\"version\":1}],\"version\":1}],\"ports\":[{\"value\":null,\"version\":0},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1}],\"node_loops\":[{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1}],\"loop_nodes\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":2,\"version\":1},{\"idx\":3,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":4,\"version\":1},{\"idx\":5,\"version\":1},{\"idx\":6,\"version\":1},{\"idx\":7,\"version\":1},{\"idx\":8,\"version\":1},{\"idx\":9,\"version\":1}],\"version\":1}],\"loop_parent\":[{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":{\"idx\":1,\"version\":1},\"version\":1}],\"root_loops\":[{\"idx\":1,\"version\":1}],\"loop_children\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":2,\"version\":1}],\"version\":1},{\"value\":[],\"version\":1}],\"node_subgraph\":[{\"value\":null,\"version\":0},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1},{\"value\":{\"idx\":4,\"version\":1},\"version\":1},{\"value\":{\"idx\":4,\"version\":1},\"version\":1},{\"value\":{\"idx\":4,\"version\":1},\"version\":1}],\"subgraph_nodes\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":1,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":2,\"version\":1},{\"idx\":3,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":4,\"version\":1},{\"idx\":5,\"version\":1},{\"idx\":6,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":7,\"version\":1},{\"idx\":8,\"version\":1},{\"idx\":9,\"version\":1}],\"version\":1}],\"subgraph_stratum\":[{\"value\":null,\"version\":0},{\"value\":0,\"version\":1},{\"value\":1,\"version\":1},{\"value\":2,\"version\":1},{\"value\":2,\"version\":1}],\"node_singleton_references\":[{\"value\":null,\"version\":0},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1}],\"node_varnames\":[{\"value\":null,\"version\":0},{\"value\":\"a\",\"version\":1},{\"value\":\"b\",\"version\":1},{\"value\":\"b\",\"version\":1}],\"subgraph_laziness\":[{\"value\":null,\"version\":0}]}",
                );
                df.__assign_diagnostics("[]");
                let (hoff_10v1_send, hoff_10v1_recv) = df
                    .make_edge::<
                        _,
                        ::dfir_rs::scheduled::handoff::VecHandoff<_>,
                    >("handoff GraphNodeId(10v1)");
                let (hoff_11v1_send, hoff_11v1_recv) = df
                    .make_edge::<
                        _,
                        ::dfir_rs::scheduled::handoff::VecHandoff<_>,
                    >("handoff GraphNodeId(11v1)");
                let (hoff_12v1_send, hoff_12v1_recv) = df
                    .make_edge::<
                        _,
                        ::dfir_rs::scheduled::handoff::VecHandoff<_>,
                    >("handoff GraphNodeId(12v1)");
                let loop_1v1 = df.add_loop(None);
                let loop_2v1 = df.add_loop(Some(loop_1v1));
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_1v1__source_iter__loc_dfir_rs_tests_surface_loop_rs_387_13_387_31<
                    T,
                >(thunk: impl FnOnce() -> T) -> T {
                    thunk()
                }
                let mut sg_1v1_node_1v1_iter = {
                    #[inline(always)]
                    fn check_iter<
                        IntoIter: ::std::iter::IntoIterator<Item = Item>,
                        Item,
                    >(into_iter: IntoIter) -> impl ::std::iter::Iterator<Item = Item> {
                        ::std::iter::IntoIterator::into_iter(into_iter)
                    }
                    check_iter(0..10)
                };
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_2v1__batch__loc_dfir_rs_tests_surface_loop_rs_389_22_389_29<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_3v1__tee__loc_dfir_rs_tests_surface_loop_rs_389_33_389_38<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_4v1__repeat_n__loc_dfir_rs_tests_surface_loop_rs_391_22_391_33<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(clippy::redundant_closure_call)]
                let singleton_op_4v1 = df
                    .add_state(::std::cell::RefCell::new(::std::vec::Vec::new()));
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_5v1__fold__loc_dfir_rs_tests_surface_loop_rs_392_24_394_23<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(unused_mut, reason = "for if `Fn` instead of `FnMut`.")]
                let mut sg_3v1_node_5v1_initializer_func = || 10000;
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_6v1__for_each__loc_dfir_rs_tests_surface_loop_rs_395_24_395_67<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_7v1__repeat_n__loc_dfir_rs_tests_surface_loop_rs_397_22_397_33<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(clippy::redundant_closure_call)]
                let singleton_op_7v1 = df
                    .add_state(::std::cell::RefCell::new(::std::vec::Vec::new()));
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_8v1__fold__loc_dfir_rs_tests_surface_loop_rs_398_24_400_23<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(unused_mut, reason = "for if `Fn` instead of `FnMut`.")]
                let mut sg_4v1_node_8v1_initializer_func = || 10000;
                #[allow(clippy::redundant_closure_call)]
                let singleton_op_8v1 = df
                    .add_state(
                        ::std::cell::RefCell::new((sg_4v1_node_8v1_initializer_func)()),
                    );
                #[allow(clippy::redundant_closure_call)]
                df.set_state_lifespan_hook(
                    singleton_op_8v1,
                    ::dfir_rs::scheduled::graph::StateLifespan::Loop(loop_2v1),
                    move |rcell| {
                        rcell.replace((sg_4v1_node_8v1_initializer_func)());
                    },
                );
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_9v1__for_each__loc_dfir_rs_tests_surface_loop_rs_401_24_401_67<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                let sgid_1v1 = df
                    .add_subgraph_full(
                        "Subgraph GraphSubgraphId(1v1)",
                        0,
                        (),
                        (hoff_10v1_send, ()),
                        false,
                        None,
                        move |context, (), (hoff_10v1_send, ())| {
                            let hoff_10v1_send = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                v|
                            {
                                hoff_10v1_send.give(Some(v));
                            });
                            let op_1v1 = sg_1v1_node_1v1_iter.by_ref();
                            let op_1v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_1v1__source_iter__loc_dfir_rs_tests_surface_loop_rs_387_13_387_31<
                                    Item,
                                    Input: ::std::iter::Iterator<Item = Item>,
                                >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Pull<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > Iterator for Pull<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item> {
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize, Option<usize>) {
                                            self.inner.size_hint()
                                        }
                                    }
                                    Pull { inner: input }
                                }
                                op_1v1__source_iter__loc_dfir_rs_tests_surface_loop_rs_387_13_387_31(
                                    op_1v1,
                                )
                            };
                            #[inline(always)]
                            fn pivot_run_sg_1v1<
                                Pull: ::std::iter::Iterator<Item = Item>,
                                Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                Item,
                            >(pull: Pull, push: Push) {
                                ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                            }
                            pivot_run_sg_1v1(op_1v1, hoff_10v1_send);
                        },
                    );
                let sgid_2v1 = df
                    .add_subgraph_full(
                        "Subgraph GraphSubgraphId(2v1)",
                        1,
                        (hoff_10v1_recv, ()),
                        (hoff_11v1_send, (hoff_12v1_send, ())),
                        false,
                        Some(loop_1v1),
                        move |
                            context,
                            (hoff_10v1_recv, ()),
                            (hoff_11v1_send, (hoff_12v1_send, ()))|
                        {
                            let mut hoff_10v1_recv = hoff_10v1_recv.borrow_mut_swap();
                            let hoff_10v1_recv = hoff_10v1_recv.drain(..);
                            let hoff_11v1_send = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                v|
                            {
                                hoff_11v1_send.give(Some(v));
                            });
                            let hoff_12v1_send = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                v|
                            {
                                hoff_12v1_send.give(Some(v));
                            });
                            let op_2v1 = {
                                fn check_input<
                                    Iter: ::std::iter::Iterator<Item = Item>,
                                    Item,
                                >(iter: Iter) -> impl ::std::iter::Iterator<Item = Item> {
                                    iter
                                }
                                check_input::<_, _>(hoff_10v1_recv)
                            };
                            let op_2v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_2v1__batch__loc_dfir_rs_tests_surface_loop_rs_389_22_389_29<
                                    Item,
                                    Input: ::std::iter::Iterator<Item = Item>,
                                >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Pull<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > Iterator for Pull<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item> {
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize, Option<usize>) {
                                            self.inner.size_hint()
                                        }
                                    }
                                    Pull { inner: input }
                                }
                                op_2v1__batch__loc_dfir_rs_tests_surface_loop_rs_389_22_389_29(
                                    op_2v1,
                                )
                            };
                            let op_3v1 = ::dfir_rs::pusherator::tee::Tee::new(
                                hoff_11v1_send,
                                hoff_12v1_send,
                            );
                            let op_3v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_3v1__tee__loc_dfir_rs_tests_surface_loop_rs_389_33_389_38<
                                    Item,
                                    Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                >(
                                    input: Input,
                                ) -> impl ::dfir_rs::pusherator::Pusherator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Push<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    > ::dfir_rs::pusherator::Pusherator for Push<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn give(&mut self, item: Self::Item) {
                                            self.inner.give(item)
                                        }
                                    }
                                    Push { inner: input }
                                }
                                op_3v1__tee__loc_dfir_rs_tests_surface_loop_rs_389_33_389_38(
                                    op_3v1,
                                )
                            };
                            #[inline(always)]
                            fn pivot_run_sg_2v1<
                                Pull: ::std::iter::Iterator<Item = Item>,
                                Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                Item,
                            >(pull: Pull, push: Push) {
                                ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                            }
                            pivot_run_sg_2v1(op_2v1, op_3v1);
                            context.allow_another_iteration();
                        },
                    );
                let sgid_3v1 = df
                    .add_subgraph_full(
                        "Subgraph GraphSubgraphId(3v1)",
                        2,
                        (hoff_11v1_recv, ()),
                        (),
                        false,
                        Some(loop_2v1),
                        move |context, (hoff_11v1_recv, ()), ()| {
                            let mut hoff_11v1_recv = hoff_11v1_recv.borrow_mut_swap();
                            let hoff_11v1_recv = hoff_11v1_recv.drain(..);
                            let mut sg_3v1_node_4v1_vec = unsafe {
                                context.state_ref_unchecked(singleton_op_4v1)
                            }
                                .borrow_mut();
                            if 0 == context.loop_iter_count() {
                                *sg_3v1_node_4v1_vec = hoff_11v1_recv
                                    .collect::<::std::vec::Vec<_>>();
                            }
                            let op_4v1 = std::iter::IntoIterator::into_iter(
                                ::std::clone::Clone::clone(&*sg_3v1_node_4v1_vec),
                            );
                            let op_4v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_4v1__repeat_n__loc_dfir_rs_tests_surface_loop_rs_391_22_391_33<
                                    Item,
                                    Input: ::std::iter::Iterator<Item = Item>,
                                >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Pull<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > Iterator for Pull<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item> {
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize, Option<usize>) {
                                            self.inner.size_hint()
                                        }
                                    }
                                    Pull { inner: input }
                                }
                                op_4v1__repeat_n__loc_dfir_rs_tests_surface_loop_rs_391_22_391_33(
                                    op_4v1,
                                )
                            };
                            let op_5v1 = {
                                #[allow(unused_mut)]
                                let mut sg_3v1_node_5v1_accumulator = &mut (sg_3v1_node_5v1_initializer_func)();
                                op_5v1__fold__loc_dfir_rs_tests_surface_loop_rs_392_24_394_23(||
                                {
                                    op_4v1
                                        .for_each(|sg_3v1_node_5v1_iterator_item| {
                                            #[inline(always)]
                                            fn call_comb_type<Accum, Item>(
                                                accum: &mut Accum,
                                                item: Item,
                                                func: impl Fn(&mut Accum, Item),
                                            ) {
                                                (func)(accum, item);
                                            }
                                            #[allow(clippy::redundant_closure_call)]
                                            call_comb_type(
                                                &mut *sg_3v1_node_5v1_accumulator,
                                                sg_3v1_node_5v1_iterator_item,
                                                |old: &mut _, val| {
                                                    *old += val;
                                                },
                                            );
                                        })
                                });
                                #[allow(clippy::clone_on_copy)]
                                {
                                    ::std::iter::once(
                                        op_5v1__fold__loc_dfir_rs_tests_surface_loop_rs_392_24_394_23(||
                                        ::std::clone::Clone::clone(&*sg_3v1_node_5v1_accumulator)),
                                    )
                                }
                            };
                            let op_5v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_5v1__fold__loc_dfir_rs_tests_surface_loop_rs_392_24_394_23<
                                    Item,
                                    Input: ::std::iter::Iterator<Item = Item>,
                                >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Pull<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > Iterator for Pull<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item> {
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize, Option<usize>) {
                                            self.inner.size_hint()
                                        }
                                    }
                                    Pull { inner: input }
                                }
                                op_5v1__fold__loc_dfir_rs_tests_surface_loop_rs_392_24_394_23(
                                    op_5v1,
                                )
                            };
                            let op_6v1 = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                v|
                            result1_send.send(v).unwrap());
                            let op_6v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_6v1__for_each__loc_dfir_rs_tests_surface_loop_rs_395_24_395_67<
                                    Item,
                                    Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                >(
                                    input: Input,
                                ) -> impl ::dfir_rs::pusherator::Pusherator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Push<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    > ::dfir_rs::pusherator::Pusherator for Push<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn give(&mut self, item: Self::Item) {
                                            self.inner.give(item)
                                        }
                                    }
                                    Push { inner: input }
                                }
                                op_6v1__for_each__loc_dfir_rs_tests_surface_loop_rs_395_24_395_67(
                                    op_6v1,
                                )
                            };
                            #[inline(always)]
                            fn pivot_run_sg_3v1<
                                Pull: ::std::iter::Iterator<Item = Item>,
                                Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                Item,
                            >(pull: Pull, push: Push) {
                                ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                            }
                            pivot_run_sg_3v1(op_5v1, op_6v1);
                            {
                                if context.loop_iter_count() + 1 < 5 {
                                    context.reschedule_loop_block();
                                }
                            }
                        },
                    );
                let sgid_4v1 = df
                    .add_subgraph_full(
                        "Subgraph GraphSubgraphId(4v1)",
                        2,
                        (hoff_12v1_recv, ()),
                        (),
                        false,
                        Some(loop_2v1),
                        move |context, (hoff_12v1_recv, ()), ()| {
                            let mut hoff_12v1_recv = hoff_12v1_recv.borrow_mut_swap();
                            let hoff_12v1_recv = hoff_12v1_recv.drain(..);
                            let mut sg_4v1_node_7v1_vec = unsafe {
                                context.state_ref_unchecked(singleton_op_7v1)
                            }
                                .borrow_mut();
                            if 0 == context.loop_iter_count() {
                                *sg_4v1_node_7v1_vec = hoff_12v1_recv
                                    .collect::<::std::vec::Vec<_>>();
                            }
                            let op_7v1 = std::iter::IntoIterator::into_iter(
                                ::std::clone::Clone::clone(&*sg_4v1_node_7v1_vec),
                            );
                            let op_7v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_7v1__repeat_n__loc_dfir_rs_tests_surface_loop_rs_397_22_397_33<
                                    Item,
                                    Input: ::std::iter::Iterator<Item = Item>,
                                >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Pull<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > Iterator for Pull<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item> {
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize, Option<usize>) {
                                            self.inner.size_hint()
                                        }
                                    }
                                    Pull { inner: input }
                                }
                                op_7v1__repeat_n__loc_dfir_rs_tests_surface_loop_rs_397_22_397_33(
                                    op_7v1,
                                )
                            };
                            let op_8v1 = {
                                #[allow(unused_mut)]
                                let mut sg_4v1_node_8v1_accumulator = unsafe {
                                    context.state_ref_unchecked(singleton_op_8v1)
                                }
                                    .borrow_mut();
                                op_8v1__fold__loc_dfir_rs_tests_surface_loop_rs_398_24_400_23(||
                                {
                                    op_7v1
                                        .for_each(|sg_4v1_node_8v1_iterator_item| {
                                            #[inline(always)]
                                            fn call_comb_type<Accum, Item>(
                                                accum: &mut Accum,
                                                item: Item,
                                                func: impl Fn(&mut Accum, Item),
                                            ) {
                                                (func)(accum, item);
                                            }
                                            #[allow(clippy::redundant_closure_call)]
                                            call_comb_type(
                                                &mut *sg_4v1_node_8v1_accumulator,
                                                sg_4v1_node_8v1_iterator_item,
                                                |old: &mut _, val| {
                                                    *old += val;
                                                },
                                            );
                                        })
                                });
                                #[allow(clippy::clone_on_copy)]
                                {
                                    ::std::iter::once(
                                        op_8v1__fold__loc_dfir_rs_tests_surface_loop_rs_398_24_400_23(||
                                        ::std::clone::Clone::clone(&*sg_4v1_node_8v1_accumulator)),
                                    )
                                }
                            };
                            let op_8v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_8v1__fold__loc_dfir_rs_tests_surface_loop_rs_398_24_400_23<
                                    Item,
                                    Input: ::std::iter::Iterator<Item = Item>,
                                >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Pull<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > Iterator for Pull<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item> {
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize, Option<usize>) {
                                            self.inner.size_hint()
                                        }
                                    }
                                    Pull { inner: input }
                                }
                                op_8v1__fold__loc_dfir_rs_tests_surface_loop_rs_398_24_400_23(
                                    op_8v1,
                                )
                            };
                            let op_9v1 = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                v|
                            result2_send.send(v).unwrap());
                            let op_9v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_9v1__for_each__loc_dfir_rs_tests_surface_loop_rs_401_24_401_67<
                                    Item,
                                    Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                >(
                                    input: Input,
                                ) -> impl ::dfir_rs::pusherator::Pusherator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Push<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    > ::dfir_rs::pusherator::Pusherator for Push<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn give(&mut self, item: Self::Item) {
                                            self.inner.give(item)
                                        }
                                    }
                                    Push { inner: input }
                                }
                                op_9v1__for_each__loc_dfir_rs_tests_surface_loop_rs_401_24_401_67(
                                    op_9v1,
                                )
                            };
                            #[inline(always)]
                            fn pivot_run_sg_4v1<
                                Pull: ::std::iter::Iterator<Item = Item>,
                                Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                Item,
                            >(pull: Pull, push: Push) {
                                ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                            }
                            pivot_run_sg_4v1(op_8v1, op_9v1);
                            {
                                if context.loop_iter_count() + 1 < 5 {
                                    context.reschedule_loop_block();
                                }
                            }
                        },
                    );
                df
            }
        }
    };
    df.run_available();
    match (
        &&[10045, 10045, 10045, 10045, 10045],
        &&*collect_ready::<Vec<_>, _>(&mut result1_recv),
    ) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    match (
        &&[10045, 10090, 10135, 10180, 10225],
        &&*collect_ready::<Vec<_>, _>(&mut result2_recv),
    ) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
}
extern crate test;
#[cfg(test)]
#[rustc_test_marker = "test_state_codegen"]
#[doc(hidden)]
pub const test_state_codegen: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_state_codegen"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "dfir_rs\\tests\\surface_loop.rs",
        start_line: 420usize,
        start_col: 4usize,
        end_line: 420usize,
        end_col: 22usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_state_codegen()),
    ),
};
fn test_state_codegen() {
    let mut df = {
        {
            #[allow(unused_qualifications)]
            {
                use ::dfir_rs::{var_expr, var_args};
                let mut df = ::dfir_rs::scheduled::graph::Dfir::new();
                df.__assign_meta_graph(
                    "{\"nodes\":[{\"value\":null,\"version\":0},{\"value\":{\"Operator\":\"source_iter((0 .. 10).chain(5 .. 15))\"},\"version\":1},{\"value\":{\"Operator\":\"tee()\"},\"version\":1},{\"value\":{\"Operator\":\"map(| n | (n / 3, n))\"},\"version\":1},{\"value\":{\"Operator\":\"tee()\"},\"version\":1},{\"value\":{\"Operator\":\"batch()\"},\"version\":1},{\"value\":{\"Operator\":\"fold(| | 0, | old : & mut _, val | { * old += val; })\"},\"version\":1},{\"value\":{\"Operator\":\"for_each(| v | println! (\\\"fold1 {:?}\\\", v))\"},\"version\":1},{\"value\":{\"Operator\":\"batch()\"},\"version\":1},{\"value\":{\"Operator\":\"fold :: < 'none > (| | 0, | old : & mut _, val | { * old += val; })\"},\"version\":1},{\"value\":{\"Operator\":\"for_each(| v | println! (\\\"fold2 {:?}\\\", v))\"},\"version\":1},{\"value\":{\"Operator\":\"batch()\"},\"version\":1},{\"value\":{\"Operator\":\"reduce :: < 'none > (| old : & mut _, val | { * old += val; })\"},\"version\":1},{\"value\":{\"Operator\":\"for_each(| v | println! (\\\"reduce {:?}\\\", v))\"},\"version\":1},{\"value\":{\"Operator\":\"batch()\"},\"version\":1},{\"value\":{\"Operator\":\"fold_keyed(| | 0, | old : & mut _, val | { * old += val; })\"},\"version\":1},{\"value\":{\"Operator\":\"for_each(| v | println! (\\\"fold_keyed {:?}\\\", v))\"},\"version\":1},{\"value\":{\"Operator\":\"batch()\"},\"version\":1},{\"value\":{\"Operator\":\"unique()\"},\"version\":1},{\"value\":{\"Operator\":\"for_each(| v | println! (\\\"unique {:?}\\\", v))\"},\"version\":1},{\"value\":{\"Operator\":\"join()\"},\"version\":1},{\"value\":{\"Operator\":\"for_each(| v | println! (\\\"join {:?}\\\", v))\"},\"version\":1},{\"value\":{\"Operator\":\"batch()\"},\"version\":1},{\"value\":{\"Operator\":\"batch()\"},\"version\":1},{\"value\":{\"Operator\":\"difference()\"},\"version\":1},{\"value\":{\"Operator\":\"for_each(| v | println! (\\\"difference {:?}\\\", v))\"},\"version\":1},{\"value\":{\"Operator\":\"batch()\"},\"version\":1},{\"value\":{\"Operator\":\"filter(| n | 0 == n % 2)\"},\"version\":1},{\"value\":{\"Operator\":\"batch()\"},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1}],\"operator_tag\":[{\"value\":null,\"version\":0}],\"graph\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":1,\"version\":1},{\"idx\":2,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":3,\"version\":1},{\"idx\":4,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":2,\"version\":1},{\"idx\":3,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":6,\"version\":1},{\"idx\":7,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":5,\"version\":1},{\"idx\":6,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":2,\"version\":1},{\"idx\":29,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":9,\"version\":1},{\"idx\":10,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":8,\"version\":1},{\"idx\":9,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":2,\"version\":1},{\"idx\":30,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":12,\"version\":1},{\"idx\":13,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":11,\"version\":1},{\"idx\":12,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":2,\"version\":1},{\"idx\":31,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":15,\"version\":1},{\"idx\":16,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":14,\"version\":1},{\"idx\":15,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":4,\"version\":1},{\"idx\":32,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":18,\"version\":1},{\"idx\":19,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":17,\"version\":1},{\"idx\":18,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":2,\"version\":1},{\"idx\":33,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":20,\"version\":1},{\"idx\":21,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":22,\"version\":1},{\"idx\":20,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":4,\"version\":1},{\"idx\":34,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":23,\"version\":1},{\"idx\":20,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":4,\"version\":1},{\"idx\":35,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":24,\"version\":1},{\"idx\":25,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":27,\"version\":1},{\"idx\":24,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":26,\"version\":1},{\"idx\":27,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":2,\"version\":1},{\"idx\":36,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":28,\"version\":1},{\"idx\":24,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":2,\"version\":1},{\"idx\":37,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":29,\"version\":1},{\"idx\":5,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":30,\"version\":1},{\"idx\":8,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":31,\"version\":1},{\"idx\":11,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":32,\"version\":1},{\"idx\":14,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":33,\"version\":1},{\"idx\":17,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":34,\"version\":1},{\"idx\":22,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":35,\"version\":1},{\"idx\":23,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":36,\"version\":1},{\"idx\":26,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":37,\"version\":1},{\"idx\":28,\"version\":1}],\"version\":1}],\"ports\":[{\"value\":null,\"version\":0},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",{\"Int\":\"0\"}],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",{\"Int\":\"1\"}],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",{\"Path\":\"neg\"}],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",{\"Path\":\"pos\"}],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1}],\"node_loops\":[{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1}],\"loop_nodes\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":5,\"version\":1},{\"idx\":6,\"version\":1},{\"idx\":7,\"version\":1},{\"idx\":8,\"version\":1},{\"idx\":9,\"version\":1},{\"idx\":10,\"version\":1},{\"idx\":11,\"version\":1},{\"idx\":12,\"version\":1},{\"idx\":13,\"version\":1},{\"idx\":14,\"version\":1},{\"idx\":15,\"version\":1},{\"idx\":16,\"version\":1},{\"idx\":17,\"version\":1},{\"idx\":18,\"version\":1},{\"idx\":19,\"version\":1},{\"idx\":20,\"version\":1},{\"idx\":21,\"version\":1},{\"idx\":22,\"version\":1},{\"idx\":23,\"version\":1},{\"idx\":24,\"version\":1},{\"idx\":25,\"version\":1},{\"idx\":26,\"version\":1},{\"idx\":27,\"version\":1},{\"idx\":28,\"version\":1}],\"version\":1}],\"loop_parent\":[{\"value\":null,\"version\":0}],\"root_loops\":[{\"idx\":1,\"version\":1}],\"loop_children\":[{\"value\":null,\"version\":0},{\"value\":[],\"version\":1}],\"node_subgraph\":[{\"value\":null,\"version\":0},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1},{\"value\":{\"idx\":4,\"version\":1},\"version\":1},{\"value\":{\"idx\":4,\"version\":1},\"version\":1},{\"value\":{\"idx\":4,\"version\":1},\"version\":1},{\"value\":{\"idx\":5,\"version\":1},\"version\":1},{\"value\":{\"idx\":5,\"version\":1},\"version\":1},{\"value\":{\"idx\":5,\"version\":1},\"version\":1},{\"value\":{\"idx\":6,\"version\":1},\"version\":1},{\"value\":{\"idx\":6,\"version\":1},\"version\":1},{\"value\":{\"idx\":6,\"version\":1},\"version\":1},{\"value\":{\"idx\":7,\"version\":1},\"version\":1},{\"value\":{\"idx\":7,\"version\":1},\"version\":1},{\"value\":{\"idx\":7,\"version\":1},\"version\":1},{\"value\":{\"idx\":7,\"version\":1},\"version\":1},{\"value\":{\"idx\":8,\"version\":1},\"version\":1},{\"value\":{\"idx\":8,\"version\":1},\"version\":1},{\"value\":{\"idx\":8,\"version\":1},\"version\":1},{\"value\":{\"idx\":8,\"version\":1},\"version\":1},{\"value\":{\"idx\":8,\"version\":1},\"version\":1}],\"subgraph_nodes\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":1,\"version\":1},{\"idx\":2,\"version\":1},{\"idx\":3,\"version\":1},{\"idx\":4,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":5,\"version\":1},{\"idx\":6,\"version\":1},{\"idx\":7,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":8,\"version\":1},{\"idx\":9,\"version\":1},{\"idx\":10,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":11,\"version\":1},{\"idx\":12,\"version\":1},{\"idx\":13,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":14,\"version\":1},{\"idx\":15,\"version\":1},{\"idx\":16,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":17,\"version\":1},{\"idx\":18,\"version\":1},{\"idx\":19,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":22,\"version\":1},{\"idx\":23,\"version\":1},{\"idx\":20,\"version\":1},{\"idx\":21,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":26,\"version\":1},{\"idx\":27,\"version\":1},{\"idx\":28,\"version\":1},{\"idx\":24,\"version\":1},{\"idx\":25,\"version\":1}],\"version\":1}],\"subgraph_stratum\":[{\"value\":null,\"version\":0},{\"value\":0,\"version\":1},{\"value\":1,\"version\":1},{\"value\":1,\"version\":1},{\"value\":1,\"version\":1},{\"value\":1,\"version\":1},{\"value\":1,\"version\":1},{\"value\":1,\"version\":1},{\"value\":1,\"version\":1}],\"node_singleton_references\":[{\"value\":null,\"version\":0},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1}],\"node_varnames\":[{\"value\":null,\"version\":0},{\"value\":\"a\",\"version\":1},{\"value\":\"a\",\"version\":1},{\"value\":\"pairs\",\"version\":1},{\"value\":\"pairs\",\"version\":1},{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":\"j\",\"version\":1},{\"value\":\"j\",\"version\":1},{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":\"aj\",\"version\":1},{\"value\":\"aj\",\"version\":1}],\"subgraph_laziness\":[{\"value\":null,\"version\":0}]}",
                );
                df.__assign_diagnostics("[]");
                let (hoff_29v1_send, hoff_29v1_recv) = df
                    .make_edge::<
                        _,
                        ::dfir_rs::scheduled::handoff::VecHandoff<_>,
                    >("handoff GraphNodeId(29v1)");
                let (hoff_30v1_send, hoff_30v1_recv) = df
                    .make_edge::<
                        _,
                        ::dfir_rs::scheduled::handoff::VecHandoff<_>,
                    >("handoff GraphNodeId(30v1)");
                let (hoff_31v1_send, hoff_31v1_recv) = df
                    .make_edge::<
                        _,
                        ::dfir_rs::scheduled::handoff::VecHandoff<_>,
                    >("handoff GraphNodeId(31v1)");
                let (hoff_32v1_send, hoff_32v1_recv) = df
                    .make_edge::<
                        _,
                        ::dfir_rs::scheduled::handoff::VecHandoff<_>,
                    >("handoff GraphNodeId(32v1)");
                let (hoff_33v1_send, hoff_33v1_recv) = df
                    .make_edge::<
                        _,
                        ::dfir_rs::scheduled::handoff::VecHandoff<_>,
                    >("handoff GraphNodeId(33v1)");
                let (hoff_34v1_send, hoff_34v1_recv) = df
                    .make_edge::<
                        _,
                        ::dfir_rs::scheduled::handoff::VecHandoff<_>,
                    >("handoff GraphNodeId(34v1)");
                let (hoff_35v1_send, hoff_35v1_recv) = df
                    .make_edge::<
                        _,
                        ::dfir_rs::scheduled::handoff::VecHandoff<_>,
                    >("handoff GraphNodeId(35v1)");
                let (hoff_36v1_send, hoff_36v1_recv) = df
                    .make_edge::<
                        _,
                        ::dfir_rs::scheduled::handoff::VecHandoff<_>,
                    >("handoff GraphNodeId(36v1)");
                let (hoff_37v1_send, hoff_37v1_recv) = df
                    .make_edge::<
                        _,
                        ::dfir_rs::scheduled::handoff::VecHandoff<_>,
                    >("handoff GraphNodeId(37v1)");
                let loop_1v1 = df.add_loop(None);
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_1v1__source_iter__loc_dfir_rs_tests_surface_loop_rs_422_13_422_46<
                    T,
                >(thunk: impl FnOnce() -> T) -> T {
                    thunk()
                }
                let mut sg_1v1_node_1v1_iter = {
                    #[inline(always)]
                    fn check_iter<
                        IntoIter: ::std::iter::IntoIterator<Item = Item>,
                        Item,
                    >(into_iter: IntoIter) -> impl ::std::iter::Iterator<Item = Item> {
                        ::std::iter::IntoIterator::into_iter(into_iter)
                    }
                    check_iter((0..10).chain(5..15))
                };
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_4v1__tee__loc_dfir_rs_tests_surface_loop_rs_423_45_423_50<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_3v1__map__loc_dfir_rs_tests_surface_loop_rs_423_22_423_41<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_2v1__tee__loc_dfir_rs_tests_surface_loop_rs_422_50_422_55<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_5v1__batch__loc_dfir_rs_tests_surface_loop_rs_425_18_425_25<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_6v1__fold__loc_dfir_rs_tests_surface_loop_rs_425_29_427_15<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(unused_mut, reason = "for if `Fn` instead of `FnMut`.")]
                let mut sg_2v1_node_6v1_initializer_func = || 0;
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_7v1__for_each__loc_dfir_rs_tests_surface_loop_rs_427_19_427_58<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_8v1__batch__loc_dfir_rs_tests_surface_loop_rs_429_18_429_25<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_9v1__fold__loc_dfir_rs_tests_surface_loop_rs_429_29_431_15<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(unused_mut, reason = "for if `Fn` instead of `FnMut`.")]
                let mut sg_3v1_node_9v1_initializer_func = || 0;
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_10v1__for_each__loc_dfir_rs_tests_surface_loop_rs_431_19_431_58<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_11v1__batch__loc_dfir_rs_tests_surface_loop_rs_433_18_433_25<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_12v1__reduce__loc_dfir_rs_tests_surface_loop_rs_433_29_435_15<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                let singleton_op_12v1 = df
                    .add_state(::std::cell::RefCell::new(::std::option::Option::None));
                df.set_state_lifespan_hook(
                    singleton_op_12v1,
                    ::dfir_rs::scheduled::graph::StateLifespan::Subgraph(sgid_4v1),
                    move |rcell| {
                        rcell.replace(::std::option::Option::None);
                    },
                );
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_13v1__for_each__loc_dfir_rs_tests_surface_loop_rs_435_19_435_59<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_14v1__batch__loc_dfir_rs_tests_surface_loop_rs_437_22_437_29<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_15v1__fold_keyed__loc_dfir_rs_tests_surface_loop_rs_437_33_439_15<
                    T,
                >(thunk: impl FnOnce() -> T) -> T {
                    thunk()
                }
                let sg_5v1_node_15v1_groupbydata = df
                    .add_state(
                        ::std::cell::RefCell::new(
                            ::dfir_rs::rustc_hash::FxHashMap::<_, _>::default(),
                        ),
                    );
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_16v1__for_each__loc_dfir_rs_tests_surface_loop_rs_439_19_439_63<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_17v1__batch__loc_dfir_rs_tests_surface_loop_rs_441_18_441_25<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_18v1__unique__loc_dfir_rs_tests_surface_loop_rs_441_29_441_37<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_19v1__for_each__loc_dfir_rs_tests_surface_loop_rs_441_41_441_81<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_22v1__batch__loc_dfir_rs_tests_surface_loop_rs_444_22_444_29<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_23v1__batch__loc_dfir_rs_tests_surface_loop_rs_445_22_445_29<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_20v1__join__loc_dfir_rs_tests_surface_loop_rs_443_17_443_23<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_21v1__for_each__loc_dfir_rs_tests_surface_loop_rs_443_27_443_65<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_26v1__batch__loc_dfir_rs_tests_surface_loop_rs_448_18_448_25<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_27v1__filter__loc_dfir_rs_tests_surface_loop_rs_448_29_448_51<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_28v1__batch__loc_dfir_rs_tests_surface_loop_rs_449_18_449_25<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_24v1__difference__loc_dfir_rs_tests_surface_loop_rs_447_18_447_30<
                    T,
                >(thunk: impl FnOnce() -> T) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_25v1__for_each__loc_dfir_rs_tests_surface_loop_rs_447_34_447_78<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                let sgid_1v1 = df
                    .add_subgraph_full(
                        "Subgraph GraphSubgraphId(1v1)",
                        0,
                        (),
                        (
                            hoff_29v1_send,
                            (
                                hoff_30v1_send,
                                (
                                    hoff_31v1_send,
                                    (
                                        hoff_32v1_send,
                                        (
                                            hoff_33v1_send,
                                            (
                                                hoff_34v1_send,
                                                (hoff_35v1_send, (hoff_36v1_send, (hoff_37v1_send, ()))),
                                            ),
                                        ),
                                    ),
                                ),
                            ),
                        ),
                        false,
                        None,
                        move |
                            context,
                            (),
                            (
                                hoff_29v1_send,
                                (
                                    hoff_30v1_send,
                                    (
                                        hoff_31v1_send,
                                        (
                                            hoff_32v1_send,
                                            (
                                                hoff_33v1_send,
                                                (
                                                    hoff_34v1_send,
                                                    (hoff_35v1_send, (hoff_36v1_send, (hoff_37v1_send, ()))),
                                                ),
                                            ),
                                        ),
                                    ),
                                ),
                            )|
                        {
                            let hoff_29v1_send = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                v|
                            {
                                hoff_29v1_send.give(Some(v));
                            });
                            let hoff_30v1_send = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                v|
                            {
                                hoff_30v1_send.give(Some(v));
                            });
                            let hoff_31v1_send = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                v|
                            {
                                hoff_31v1_send.give(Some(v));
                            });
                            let hoff_32v1_send = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                v|
                            {
                                hoff_32v1_send.give(Some(v));
                            });
                            let hoff_33v1_send = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                v|
                            {
                                hoff_33v1_send.give(Some(v));
                            });
                            let hoff_34v1_send = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                v|
                            {
                                hoff_34v1_send.give(Some(v));
                            });
                            let hoff_35v1_send = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                v|
                            {
                                hoff_35v1_send.give(Some(v));
                            });
                            let hoff_36v1_send = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                v|
                            {
                                hoff_36v1_send.give(Some(v));
                            });
                            let hoff_37v1_send = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                v|
                            {
                                hoff_37v1_send.give(Some(v));
                            });
                            let op_1v1 = sg_1v1_node_1v1_iter.by_ref();
                            let op_1v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_1v1__source_iter__loc_dfir_rs_tests_surface_loop_rs_422_13_422_46<
                                    Item,
                                    Input: ::std::iter::Iterator<Item = Item>,
                                >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Pull<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > Iterator for Pull<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item> {
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize, Option<usize>) {
                                            self.inner.size_hint()
                                        }
                                    }
                                    Pull { inner: input }
                                }
                                op_1v1__source_iter__loc_dfir_rs_tests_surface_loop_rs_422_13_422_46(
                                    op_1v1,
                                )
                            };
                            let op_4v1 = ::dfir_rs::pusherator::tee::Tee::new(
                                hoff_32v1_send,
                                ::dfir_rs::pusherator::tee::Tee::new(
                                    hoff_34v1_send,
                                    hoff_35v1_send,
                                ),
                            );
                            let op_4v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_4v1__tee__loc_dfir_rs_tests_surface_loop_rs_423_45_423_50<
                                    Item,
                                    Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                >(
                                    input: Input,
                                ) -> impl ::dfir_rs::pusherator::Pusherator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Push<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    > ::dfir_rs::pusherator::Pusherator for Push<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn give(&mut self, item: Self::Item) {
                                            self.inner.give(item)
                                        }
                                    }
                                    Push { inner: input }
                                }
                                op_4v1__tee__loc_dfir_rs_tests_surface_loop_rs_423_45_423_50(
                                    op_4v1,
                                )
                            };
                            let op_3v1 = ::dfir_rs::pusherator::map::Map::new(
                                |n| (n / 3, n),
                                op_4v1,
                            );
                            let op_3v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_3v1__map__loc_dfir_rs_tests_surface_loop_rs_423_22_423_41<
                                    Item,
                                    Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                >(
                                    input: Input,
                                ) -> impl ::dfir_rs::pusherator::Pusherator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Push<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    > ::dfir_rs::pusherator::Pusherator for Push<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn give(&mut self, item: Self::Item) {
                                            self.inner.give(item)
                                        }
                                    }
                                    Push { inner: input }
                                }
                                op_3v1__map__loc_dfir_rs_tests_surface_loop_rs_423_22_423_41(
                                    op_3v1,
                                )
                            };
                            let op_2v1 = ::dfir_rs::pusherator::tee::Tee::new(
                                op_3v1,
                                ::dfir_rs::pusherator::tee::Tee::new(
                                    hoff_29v1_send,
                                    ::dfir_rs::pusherator::tee::Tee::new(
                                        hoff_30v1_send,
                                        ::dfir_rs::pusherator::tee::Tee::new(
                                            hoff_31v1_send,
                                            ::dfir_rs::pusherator::tee::Tee::new(
                                                hoff_33v1_send,
                                                ::dfir_rs::pusherator::tee::Tee::new(
                                                    hoff_36v1_send,
                                                    hoff_37v1_send,
                                                ),
                                            ),
                                        ),
                                    ),
                                ),
                            );
                            let op_2v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_2v1__tee__loc_dfir_rs_tests_surface_loop_rs_422_50_422_55<
                                    Item,
                                    Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                >(
                                    input: Input,
                                ) -> impl ::dfir_rs::pusherator::Pusherator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Push<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    > ::dfir_rs::pusherator::Pusherator for Push<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn give(&mut self, item: Self::Item) {
                                            self.inner.give(item)
                                        }
                                    }
                                    Push { inner: input }
                                }
                                op_2v1__tee__loc_dfir_rs_tests_surface_loop_rs_422_50_422_55(
                                    op_2v1,
                                )
                            };
                            #[inline(always)]
                            fn pivot_run_sg_1v1<
                                Pull: ::std::iter::Iterator<Item = Item>,
                                Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                Item,
                            >(pull: Pull, push: Push) {
                                ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                            }
                            pivot_run_sg_1v1(op_1v1, op_2v1);
                        },
                    );
                let sgid_2v1 = df
                    .add_subgraph_full(
                        "Subgraph GraphSubgraphId(2v1)",
                        1,
                        (hoff_29v1_recv, ()),
                        (),
                        false,
                        Some(loop_1v1),
                        move |context, (hoff_29v1_recv, ()), ()| {
                            let mut hoff_29v1_recv = hoff_29v1_recv.borrow_mut_swap();
                            let hoff_29v1_recv = hoff_29v1_recv.drain(..);
                            let op_5v1 = {
                                fn check_input<
                                    Iter: ::std::iter::Iterator<Item = Item>,
                                    Item,
                                >(iter: Iter) -> impl ::std::iter::Iterator<Item = Item> {
                                    iter
                                }
                                check_input::<_, _>(hoff_29v1_recv)
                            };
                            let op_5v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_5v1__batch__loc_dfir_rs_tests_surface_loop_rs_425_18_425_25<
                                    Item,
                                    Input: ::std::iter::Iterator<Item = Item>,
                                >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Pull<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > Iterator for Pull<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item> {
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize, Option<usize>) {
                                            self.inner.size_hint()
                                        }
                                    }
                                    Pull { inner: input }
                                }
                                op_5v1__batch__loc_dfir_rs_tests_surface_loop_rs_425_18_425_25(
                                    op_5v1,
                                )
                            };
                            let op_6v1 = {
                                #[allow(unused_mut)]
                                let mut sg_2v1_node_6v1_accumulator = &mut (sg_2v1_node_6v1_initializer_func)();
                                op_6v1__fold__loc_dfir_rs_tests_surface_loop_rs_425_29_427_15(||
                                {
                                    op_5v1
                                        .for_each(|sg_2v1_node_6v1_iterator_item| {
                                            #[inline(always)]
                                            fn call_comb_type<Accum, Item>(
                                                accum: &mut Accum,
                                                item: Item,
                                                func: impl Fn(&mut Accum, Item),
                                            ) {
                                                (func)(accum, item);
                                            }
                                            #[allow(clippy::redundant_closure_call)]
                                            call_comb_type(
                                                &mut *sg_2v1_node_6v1_accumulator,
                                                sg_2v1_node_6v1_iterator_item,
                                                |old: &mut _, val| {
                                                    *old += val;
                                                },
                                            );
                                        })
                                });
                                #[allow(clippy::clone_on_copy)]
                                {
                                    ::std::iter::once(
                                        op_6v1__fold__loc_dfir_rs_tests_surface_loop_rs_425_29_427_15(||
                                        ::std::clone::Clone::clone(&*sg_2v1_node_6v1_accumulator)),
                                    )
                                }
                            };
                            let op_6v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_6v1__fold__loc_dfir_rs_tests_surface_loop_rs_425_29_427_15<
                                    Item,
                                    Input: ::std::iter::Iterator<Item = Item>,
                                >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Pull<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > Iterator for Pull<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item> {
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize, Option<usize>) {
                                            self.inner.size_hint()
                                        }
                                    }
                                    Pull { inner: input }
                                }
                                op_6v1__fold__loc_dfir_rs_tests_surface_loop_rs_425_29_427_15(
                                    op_6v1,
                                )
                            };
                            let op_7v1 = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                v|
                            {
                                ::std::io::_print(format_args!("fold1 {0:?}\n", v));
                            });
                            let op_7v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_7v1__for_each__loc_dfir_rs_tests_surface_loop_rs_427_19_427_58<
                                    Item,
                                    Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                >(
                                    input: Input,
                                ) -> impl ::dfir_rs::pusherator::Pusherator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Push<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    > ::dfir_rs::pusherator::Pusherator for Push<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn give(&mut self, item: Self::Item) {
                                            self.inner.give(item)
                                        }
                                    }
                                    Push { inner: input }
                                }
                                op_7v1__for_each__loc_dfir_rs_tests_surface_loop_rs_427_19_427_58(
                                    op_7v1,
                                )
                            };
                            #[inline(always)]
                            fn pivot_run_sg_2v1<
                                Pull: ::std::iter::Iterator<Item = Item>,
                                Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                Item,
                            >(pull: Pull, push: Push) {
                                ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                            }
                            pivot_run_sg_2v1(op_6v1, op_7v1);
                            context.allow_another_iteration();
                        },
                    );
                let sgid_3v1 = df
                    .add_subgraph_full(
                        "Subgraph GraphSubgraphId(3v1)",
                        1,
                        (hoff_30v1_recv, ()),
                        (),
                        false,
                        Some(loop_1v1),
                        move |context, (hoff_30v1_recv, ()), ()| {
                            let mut hoff_30v1_recv = hoff_30v1_recv.borrow_mut_swap();
                            let hoff_30v1_recv = hoff_30v1_recv.drain(..);
                            let op_8v1 = {
                                fn check_input<
                                    Iter: ::std::iter::Iterator<Item = Item>,
                                    Item,
                                >(iter: Iter) -> impl ::std::iter::Iterator<Item = Item> {
                                    iter
                                }
                                check_input::<_, _>(hoff_30v1_recv)
                            };
                            let op_8v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_8v1__batch__loc_dfir_rs_tests_surface_loop_rs_429_18_429_25<
                                    Item,
                                    Input: ::std::iter::Iterator<Item = Item>,
                                >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Pull<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > Iterator for Pull<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item> {
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize, Option<usize>) {
                                            self.inner.size_hint()
                                        }
                                    }
                                    Pull { inner: input }
                                }
                                op_8v1__batch__loc_dfir_rs_tests_surface_loop_rs_429_18_429_25(
                                    op_8v1,
                                )
                            };
                            let op_9v1 = {
                                #[allow(unused_mut)]
                                let mut sg_3v1_node_9v1_accumulator = &mut (sg_3v1_node_9v1_initializer_func)();
                                op_9v1__fold__loc_dfir_rs_tests_surface_loop_rs_429_29_431_15(||
                                {
                                    op_8v1
                                        .for_each(|sg_3v1_node_9v1_iterator_item| {
                                            #[inline(always)]
                                            fn call_comb_type<Accum, Item>(
                                                accum: &mut Accum,
                                                item: Item,
                                                func: impl Fn(&mut Accum, Item),
                                            ) {
                                                (func)(accum, item);
                                            }
                                            #[allow(clippy::redundant_closure_call)]
                                            call_comb_type(
                                                &mut *sg_3v1_node_9v1_accumulator,
                                                sg_3v1_node_9v1_iterator_item,
                                                |old: &mut _, val| {
                                                    *old += val;
                                                },
                                            );
                                        })
                                });
                                #[allow(clippy::clone_on_copy)]
                                {
                                    ::std::iter::once(
                                        op_9v1__fold__loc_dfir_rs_tests_surface_loop_rs_429_29_431_15(||
                                        ::std::clone::Clone::clone(&*sg_3v1_node_9v1_accumulator)),
                                    )
                                }
                            };
                            let op_9v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_9v1__fold__loc_dfir_rs_tests_surface_loop_rs_429_29_431_15<
                                    Item,
                                    Input: ::std::iter::Iterator<Item = Item>,
                                >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Pull<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > Iterator for Pull<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item> {
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize, Option<usize>) {
                                            self.inner.size_hint()
                                        }
                                    }
                                    Pull { inner: input }
                                }
                                op_9v1__fold__loc_dfir_rs_tests_surface_loop_rs_429_29_431_15(
                                    op_9v1,
                                )
                            };
                            let op_10v1 = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                v|
                            {
                                ::std::io::_print(format_args!("fold2 {0:?}\n", v));
                            });
                            let op_10v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_10v1__for_each__loc_dfir_rs_tests_surface_loop_rs_431_19_431_58<
                                    Item,
                                    Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                >(
                                    input: Input,
                                ) -> impl ::dfir_rs::pusherator::Pusherator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Push<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    > ::dfir_rs::pusherator::Pusherator for Push<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn give(&mut self, item: Self::Item) {
                                            self.inner.give(item)
                                        }
                                    }
                                    Push { inner: input }
                                }
                                op_10v1__for_each__loc_dfir_rs_tests_surface_loop_rs_431_19_431_58(
                                    op_10v1,
                                )
                            };
                            #[inline(always)]
                            fn pivot_run_sg_3v1<
                                Pull: ::std::iter::Iterator<Item = Item>,
                                Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                Item,
                            >(pull: Pull, push: Push) {
                                ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                            }
                            pivot_run_sg_3v1(op_9v1, op_10v1);
                            context.allow_another_iteration();
                        },
                    );
                let sgid_4v1 = df
                    .add_subgraph_full(
                        "Subgraph GraphSubgraphId(4v1)",
                        1,
                        (hoff_31v1_recv, ()),
                        (),
                        false,
                        Some(loop_1v1),
                        move |context, (hoff_31v1_recv, ()), ()| {
                            let mut hoff_31v1_recv = hoff_31v1_recv.borrow_mut_swap();
                            let hoff_31v1_recv = hoff_31v1_recv.drain(..);
                            let op_11v1 = {
                                fn check_input<
                                    Iter: ::std::iter::Iterator<Item = Item>,
                                    Item,
                                >(iter: Iter) -> impl ::std::iter::Iterator<Item = Item> {
                                    iter
                                }
                                check_input::<_, _>(hoff_31v1_recv)
                            };
                            let op_11v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_11v1__batch__loc_dfir_rs_tests_surface_loop_rs_433_18_433_25<
                                    Item,
                                    Input: ::std::iter::Iterator<Item = Item>,
                                >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Pull<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > Iterator for Pull<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item> {
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize, Option<usize>) {
                                            self.inner.size_hint()
                                        }
                                    }
                                    Pull { inner: input }
                                }
                                op_11v1__batch__loc_dfir_rs_tests_surface_loop_rs_433_18_433_25(
                                    op_11v1,
                                )
                            };
                            let op_12v1 = {
                                #[allow(unused_mut)]
                                let mut sg_4v1_node_12v1_accumulator = unsafe {
                                    context.state_ref_unchecked(singleton_op_12v1)
                                }
                                    .borrow_mut();
                                op_12v1__reduce__loc_dfir_rs_tests_surface_loop_rs_433_29_435_15(||
                                {
                                    op_11v1
                                        .for_each(|sg_4v1_node_12v1_iterator_item| {
                                            #[inline(always)]
                                            fn call_comb_type<Item>(
                                                accum: &mut Option<Item>,
                                                item: Item,
                                                func: impl Fn(&mut Item, Item),
                                            ) {
                                                match accum {
                                                    accum @ None => *accum = Some(item),
                                                    Some(accum) => (func)(accum, item),
                                                }
                                            }
                                            #[allow(clippy::redundant_closure_call)]
                                            call_comb_type(
                                                &mut *sg_4v1_node_12v1_accumulator,
                                                sg_4v1_node_12v1_iterator_item,
                                                |old: &mut _, val| {
                                                    *old += val;
                                                },
                                            );
                                        })
                                });
                                #[allow(clippy::clone_on_copy)]
                                {
                                    ::std::iter::IntoIterator::into_iter(
                                        op_12v1__reduce__loc_dfir_rs_tests_surface_loop_rs_433_29_435_15(||
                                        ::std::clone::Clone::clone(&*sg_4v1_node_12v1_accumulator)),
                                    )
                                }
                            };
                            let op_12v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_12v1__reduce__loc_dfir_rs_tests_surface_loop_rs_433_29_435_15<
                                    Item,
                                    Input: ::std::iter::Iterator<Item = Item>,
                                >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Pull<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > Iterator for Pull<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item> {
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize, Option<usize>) {
                                            self.inner.size_hint()
                                        }
                                    }
                                    Pull { inner: input }
                                }
                                op_12v1__reduce__loc_dfir_rs_tests_surface_loop_rs_433_29_435_15(
                                    op_12v1,
                                )
                            };
                            let op_13v1 = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                v|
                            {
                                ::std::io::_print(format_args!("reduce {0:?}\n", v));
                            });
                            let op_13v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_13v1__for_each__loc_dfir_rs_tests_surface_loop_rs_435_19_435_59<
                                    Item,
                                    Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                >(
                                    input: Input,
                                ) -> impl ::dfir_rs::pusherator::Pusherator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Push<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    > ::dfir_rs::pusherator::Pusherator for Push<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn give(&mut self, item: Self::Item) {
                                            self.inner.give(item)
                                        }
                                    }
                                    Push { inner: input }
                                }
                                op_13v1__for_each__loc_dfir_rs_tests_surface_loop_rs_435_19_435_59(
                                    op_13v1,
                                )
                            };
                            #[inline(always)]
                            fn pivot_run_sg_4v1<
                                Pull: ::std::iter::Iterator<Item = Item>,
                                Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                Item,
                            >(pull: Pull, push: Push) {
                                ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                            }
                            pivot_run_sg_4v1(op_12v1, op_13v1);
                            context.allow_another_iteration();
                        },
                    );
                let sgid_5v1 = df
                    .add_subgraph_full(
                        "Subgraph GraphSubgraphId(5v1)",
                        1,
                        (hoff_32v1_recv, ()),
                        (),
                        false,
                        Some(loop_1v1),
                        move |context, (hoff_32v1_recv, ()), ()| {
                            let mut hoff_32v1_recv = hoff_32v1_recv.borrow_mut_swap();
                            let hoff_32v1_recv = hoff_32v1_recv.drain(..);
                            let op_14v1 = {
                                fn check_input<
                                    Iter: ::std::iter::Iterator<Item = Item>,
                                    Item,
                                >(iter: Iter) -> impl ::std::iter::Iterator<Item = Item> {
                                    iter
                                }
                                check_input::<_, _>(hoff_32v1_recv)
                            };
                            let op_14v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_14v1__batch__loc_dfir_rs_tests_surface_loop_rs_437_22_437_29<
                                    Item,
                                    Input: ::std::iter::Iterator<Item = Item>,
                                >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Pull<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > Iterator for Pull<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item> {
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize, Option<usize>) {
                                            self.inner.size_hint()
                                        }
                                    }
                                    Pull { inner: input }
                                }
                                op_14v1__batch__loc_dfir_rs_tests_surface_loop_rs_437_22_437_29(
                                    op_14v1,
                                )
                            };
                            let mut sg_5v1_node_15v1_hashtable = unsafe {
                                context.state_ref_unchecked(sg_5v1_node_15v1_groupbydata)
                            }
                                .borrow_mut();
                            op_15v1__fold_keyed__loc_dfir_rs_tests_surface_loop_rs_437_33_439_15(||
                            {
                                #[inline(always)]
                                fn check_input<Iter, K, V>(
                                    iter: Iter,
                                ) -> impl ::std::iter::Iterator<Item = (K, V)>
                                where
                                    Iter: std::iter::Iterator<Item = (K, V)>,
                                    K: ::std::clone::Clone,
                                    V: ::std::clone::Clone,
                                {
                                    iter
                                }
                                /// A: accumulator type
                                /// T: iterator item type
                                /// O: output type
                                #[inline(always)]
                                fn call_comb_type<A, T, O>(
                                    a: &mut A,
                                    t: T,
                                    f: impl Fn(&mut A, T) -> O,
                                ) -> O {
                                    (f)(a, t)
                                }
                                for kv in check_input(op_14v1) {
                                    #[allow(unknown_lints, clippy::unwrap_or_default)]
                                    let entry = sg_5v1_node_15v1_hashtable
                                        .entry(kv.0)
                                        .or_insert_with(|| 0);
                                    call_comb_type(
                                        entry,
                                        kv.1,
                                        |old: &mut _, val| {
                                            *old += val;
                                        },
                                    );
                                }
                            });
                            let op_15v1 = sg_5v1_node_15v1_hashtable.drain();
                            let op_15v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_15v1__fold_keyed__loc_dfir_rs_tests_surface_loop_rs_437_33_439_15<
                                    Item,
                                    Input: ::std::iter::Iterator<Item = Item>,
                                >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Pull<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > Iterator for Pull<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item> {
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize, Option<usize>) {
                                            self.inner.size_hint()
                                        }
                                    }
                                    Pull { inner: input }
                                }
                                op_15v1__fold_keyed__loc_dfir_rs_tests_surface_loop_rs_437_33_439_15(
                                    op_15v1,
                                )
                            };
                            let op_16v1 = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                v|
                            {
                                ::std::io::_print(format_args!("fold_keyed {0:?}\n", v));
                            });
                            let op_16v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_16v1__for_each__loc_dfir_rs_tests_surface_loop_rs_439_19_439_63<
                                    Item,
                                    Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                >(
                                    input: Input,
                                ) -> impl ::dfir_rs::pusherator::Pusherator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Push<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    > ::dfir_rs::pusherator::Pusherator for Push<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn give(&mut self, item: Self::Item) {
                                            self.inner.give(item)
                                        }
                                    }
                                    Push { inner: input }
                                }
                                op_16v1__for_each__loc_dfir_rs_tests_surface_loop_rs_439_19_439_63(
                                    op_16v1,
                                )
                            };
                            #[inline(always)]
                            fn pivot_run_sg_5v1<
                                Pull: ::std::iter::Iterator<Item = Item>,
                                Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                Item,
                            >(pull: Pull, push: Push) {
                                ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                            }
                            pivot_run_sg_5v1(op_15v1, op_16v1);
                            context.allow_another_iteration();
                        },
                    );
                let sgid_6v1 = df
                    .add_subgraph_full(
                        "Subgraph GraphSubgraphId(6v1)",
                        1,
                        (hoff_33v1_recv, ()),
                        (),
                        false,
                        Some(loop_1v1),
                        move |context, (hoff_33v1_recv, ()), ()| {
                            let mut hoff_33v1_recv = hoff_33v1_recv.borrow_mut_swap();
                            let hoff_33v1_recv = hoff_33v1_recv.drain(..);
                            let op_17v1 = {
                                fn check_input<
                                    Iter: ::std::iter::Iterator<Item = Item>,
                                    Item,
                                >(iter: Iter) -> impl ::std::iter::Iterator<Item = Item> {
                                    iter
                                }
                                check_input::<_, _>(hoff_33v1_recv)
                            };
                            let op_17v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_17v1__batch__loc_dfir_rs_tests_surface_loop_rs_441_18_441_25<
                                    Item,
                                    Input: ::std::iter::Iterator<Item = Item>,
                                >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Pull<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > Iterator for Pull<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item> {
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize, Option<usize>) {
                                            self.inner.size_hint()
                                        }
                                    }
                                    Pull { inner: input }
                                }
                                op_17v1__batch__loc_dfir_rs_tests_surface_loop_rs_441_18_441_25(
                                    op_17v1,
                                )
                            };
                            let op_18v1 = op_17v1
                                .filter(|item| {
                                    let mut set = ::dfir_rs::rustc_hash::FxHashSet::default();
                                    if !set.contains(item) {
                                        set.insert(::std::clone::Clone::clone(item));
                                        true
                                    } else {
                                        false
                                    }
                                });
                            let op_18v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_18v1__unique__loc_dfir_rs_tests_surface_loop_rs_441_29_441_37<
                                    Item,
                                    Input: ::std::iter::Iterator<Item = Item>,
                                >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Pull<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > Iterator for Pull<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item> {
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize, Option<usize>) {
                                            self.inner.size_hint()
                                        }
                                    }
                                    Pull { inner: input }
                                }
                                op_18v1__unique__loc_dfir_rs_tests_surface_loop_rs_441_29_441_37(
                                    op_18v1,
                                )
                            };
                            let op_19v1 = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                v|
                            {
                                ::std::io::_print(format_args!("unique {0:?}\n", v));
                            });
                            let op_19v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_19v1__for_each__loc_dfir_rs_tests_surface_loop_rs_441_41_441_81<
                                    Item,
                                    Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                >(
                                    input: Input,
                                ) -> impl ::dfir_rs::pusherator::Pusherator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Push<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    > ::dfir_rs::pusherator::Pusherator for Push<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn give(&mut self, item: Self::Item) {
                                            self.inner.give(item)
                                        }
                                    }
                                    Push { inner: input }
                                }
                                op_19v1__for_each__loc_dfir_rs_tests_surface_loop_rs_441_41_441_81(
                                    op_19v1,
                                )
                            };
                            #[inline(always)]
                            fn pivot_run_sg_6v1<
                                Pull: ::std::iter::Iterator<Item = Item>,
                                Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                Item,
                            >(pull: Pull, push: Push) {
                                ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                            }
                            pivot_run_sg_6v1(op_18v1, op_19v1);
                            context.allow_another_iteration();
                        },
                    );
                let sgid_7v1 = df
                    .add_subgraph_full(
                        "Subgraph GraphSubgraphId(7v1)",
                        1,
                        (hoff_34v1_recv, (hoff_35v1_recv, ())),
                        (),
                        false,
                        Some(loop_1v1),
                        move |context, (hoff_34v1_recv, (hoff_35v1_recv, ())), ()| {
                            let mut hoff_34v1_recv = hoff_34v1_recv.borrow_mut_swap();
                            let hoff_34v1_recv = hoff_34v1_recv.drain(..);
                            let mut hoff_35v1_recv = hoff_35v1_recv.borrow_mut_swap();
                            let hoff_35v1_recv = hoff_35v1_recv.drain(..);
                            let op_22v1 = {
                                fn check_input<
                                    Iter: ::std::iter::Iterator<Item = Item>,
                                    Item,
                                >(iter: Iter) -> impl ::std::iter::Iterator<Item = Item> {
                                    iter
                                }
                                check_input::<_, _>(hoff_34v1_recv)
                            };
                            let op_22v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_22v1__batch__loc_dfir_rs_tests_surface_loop_rs_444_22_444_29<
                                    Item,
                                    Input: ::std::iter::Iterator<Item = Item>,
                                >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Pull<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > Iterator for Pull<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item> {
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize, Option<usize>) {
                                            self.inner.size_hint()
                                        }
                                    }
                                    Pull { inner: input }
                                }
                                op_22v1__batch__loc_dfir_rs_tests_surface_loop_rs_444_22_444_29(
                                    op_22v1,
                                )
                            };
                            let op_23v1 = {
                                fn check_input<
                                    Iter: ::std::iter::Iterator<Item = Item>,
                                    Item,
                                >(iter: Iter) -> impl ::std::iter::Iterator<Item = Item> {
                                    iter
                                }
                                check_input::<_, _>(hoff_35v1_recv)
                            };
                            let op_23v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_23v1__batch__loc_dfir_rs_tests_surface_loop_rs_445_22_445_29<
                                    Item,
                                    Input: ::std::iter::Iterator<Item = Item>,
                                >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Pull<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > Iterator for Pull<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item> {
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize, Option<usize>) {
                                            self.inner.size_hint()
                                        }
                                    }
                                    Pull { inner: input }
                                }
                                op_23v1__batch__loc_dfir_rs_tests_surface_loop_rs_445_22_445_29(
                                    op_23v1,
                                )
                            };
                            let mut sg_7v1_node_20v1_joindata_lhs_borrow = ::std::default::Default::default();
                            let mut sg_7v1_node_20v1_joindata_rhs_borrow = ::std::default::Default::default();
                            let op_20v1 = {
                                #[inline(always)]
                                fn check_inputs<'a, K, I1, V1, I2, V2>(
                                    lhs: I1,
                                    rhs: I2,
                                    lhs_state: &'a mut ::dfir_rs::compiled::pull::HalfSetJoinState<
                                        K,
                                        V1,
                                        V2,
                                    >,
                                    rhs_state: &'a mut ::dfir_rs::compiled::pull::HalfSetJoinState<
                                        K,
                                        V2,
                                        V1,
                                    >,
                                    is_new_tick: bool,
                                ) -> impl 'a + Iterator<Item = (K, (V1, V2))>
                                where
                                    K: Eq + std::hash::Hash + Clone,
                                    V1: Clone + ::std::cmp::Eq,
                                    V2: Clone + ::std::cmp::Eq,
                                    I1: 'a + Iterator<Item = (K, V1)>,
                                    I2: 'a + Iterator<Item = (K, V2)>,
                                {
                                    op_20v1__join__loc_dfir_rs_tests_surface_loop_rs_443_17_443_23(||
                                    ::dfir_rs::compiled::pull::symmetric_hash_join_into_iter(
                                        lhs,
                                        rhs,
                                        lhs_state,
                                        rhs_state,
                                        is_new_tick,
                                    ))
                                }
                                check_inputs(
                                    op_22v1,
                                    op_23v1,
                                    &mut sg_7v1_node_20v1_joindata_lhs_borrow,
                                    &mut sg_7v1_node_20v1_joindata_rhs_borrow,
                                    true,
                                )
                            };
                            let op_20v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_20v1__join__loc_dfir_rs_tests_surface_loop_rs_443_17_443_23<
                                    Item,
                                    Input: ::std::iter::Iterator<Item = Item>,
                                >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Pull<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > Iterator for Pull<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item> {
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize, Option<usize>) {
                                            self.inner.size_hint()
                                        }
                                    }
                                    Pull { inner: input }
                                }
                                op_20v1__join__loc_dfir_rs_tests_surface_loop_rs_443_17_443_23(
                                    op_20v1,
                                )
                            };
                            let op_21v1 = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                v|
                            {
                                ::std::io::_print(format_args!("join {0:?}\n", v));
                            });
                            let op_21v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_21v1__for_each__loc_dfir_rs_tests_surface_loop_rs_443_27_443_65<
                                    Item,
                                    Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                >(
                                    input: Input,
                                ) -> impl ::dfir_rs::pusherator::Pusherator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Push<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    > ::dfir_rs::pusherator::Pusherator for Push<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn give(&mut self, item: Self::Item) {
                                            self.inner.give(item)
                                        }
                                    }
                                    Push { inner: input }
                                }
                                op_21v1__for_each__loc_dfir_rs_tests_surface_loop_rs_443_27_443_65(
                                    op_21v1,
                                )
                            };
                            #[inline(always)]
                            fn pivot_run_sg_7v1<
                                Pull: ::std::iter::Iterator<Item = Item>,
                                Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                Item,
                            >(pull: Pull, push: Push) {
                                ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                            }
                            pivot_run_sg_7v1(op_20v1, op_21v1);
                            context.allow_another_iteration();
                            context.allow_another_iteration();
                        },
                    );
                let sgid_8v1 = df
                    .add_subgraph_full(
                        "Subgraph GraphSubgraphId(8v1)",
                        1,
                        (hoff_36v1_recv, (hoff_37v1_recv, ())),
                        (),
                        false,
                        Some(loop_1v1),
                        move |context, (hoff_36v1_recv, (hoff_37v1_recv, ())), ()| {
                            let mut hoff_36v1_recv = hoff_36v1_recv.borrow_mut_swap();
                            let hoff_36v1_recv = hoff_36v1_recv.drain(..);
                            let mut hoff_37v1_recv = hoff_37v1_recv.borrow_mut_swap();
                            let hoff_37v1_recv = hoff_37v1_recv.drain(..);
                            let op_26v1 = {
                                fn check_input<
                                    Iter: ::std::iter::Iterator<Item = Item>,
                                    Item,
                                >(iter: Iter) -> impl ::std::iter::Iterator<Item = Item> {
                                    iter
                                }
                                check_input::<_, _>(hoff_36v1_recv)
                            };
                            let op_26v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_26v1__batch__loc_dfir_rs_tests_surface_loop_rs_448_18_448_25<
                                    Item,
                                    Input: ::std::iter::Iterator<Item = Item>,
                                >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Pull<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > Iterator for Pull<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item> {
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize, Option<usize>) {
                                            self.inner.size_hint()
                                        }
                                    }
                                    Pull { inner: input }
                                }
                                op_26v1__batch__loc_dfir_rs_tests_surface_loop_rs_448_18_448_25(
                                    op_26v1,
                                )
                            };
                            let op_27v1 = op_26v1.filter(|n| 0 == n % 2);
                            let op_27v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_27v1__filter__loc_dfir_rs_tests_surface_loop_rs_448_29_448_51<
                                    Item,
                                    Input: ::std::iter::Iterator<Item = Item>,
                                >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Pull<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > Iterator for Pull<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item> {
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize, Option<usize>) {
                                            self.inner.size_hint()
                                        }
                                    }
                                    Pull { inner: input }
                                }
                                op_27v1__filter__loc_dfir_rs_tests_surface_loop_rs_448_29_448_51(
                                    op_27v1,
                                )
                            };
                            let op_28v1 = {
                                fn check_input<
                                    Iter: ::std::iter::Iterator<Item = Item>,
                                    Item,
                                >(iter: Iter) -> impl ::std::iter::Iterator<Item = Item> {
                                    iter
                                }
                                check_input::<_, _>(hoff_37v1_recv)
                            };
                            let op_28v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_28v1__batch__loc_dfir_rs_tests_surface_loop_rs_449_18_449_25<
                                    Item,
                                    Input: ::std::iter::Iterator<Item = Item>,
                                >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Pull<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > Iterator for Pull<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item> {
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize, Option<usize>) {
                                            self.inner.size_hint()
                                        }
                                    }
                                    Pull { inner: input }
                                }
                                op_28v1__batch__loc_dfir_rs_tests_surface_loop_rs_449_18_449_25(
                                    op_28v1,
                                )
                            };
                            let op_28v1 = op_28v1.map(|k| (k, ()));
                            let sg_8v1_node_24v1_antijoindata_pos_borrow = &mut ::dfir_rs::rustc_hash::FxHashSet::default();
                            let sg_8v1_node_24v1_antijoindata_neg_borrow = &mut ::dfir_rs::rustc_hash::FxHashSet::default();
                            let op_24v1 = {
                                /// Limit error propagation by bounding locally, erasing output iterator type.
                                #[inline(always)]
                                fn check_inputs<'a, K, I1, V, I2>(
                                    input_neg: I1,
                                    input_pos: I2,
                                    neg_state: &'a mut ::dfir_rs::rustc_hash::FxHashSet<K>,
                                    pos_state: &'a mut ::dfir_rs::rustc_hash::FxHashSet<(K, V)>,
                                    is_new_tick: bool,
                                ) -> impl 'a + Iterator<Item = (K, V)>
                                where
                                    K: Eq + ::std::hash::Hash + Clone,
                                    V: Eq + ::std::hash::Hash + Clone,
                                    I1: 'a + Iterator<Item = K>,
                                    I2: 'a + Iterator<Item = (K, V)>,
                                {
                                    op_24v1__difference__loc_dfir_rs_tests_surface_loop_rs_447_18_447_30(||
                                    neg_state.extend(input_neg));
                                    ::dfir_rs::compiled::pull::anti_join_into_iter(
                                        input_pos,
                                        neg_state,
                                        pos_state,
                                        is_new_tick,
                                    )
                                }
                                check_inputs(
                                    op_27v1,
                                    op_28v1,
                                    &mut *sg_8v1_node_24v1_antijoindata_neg_borrow,
                                    &mut *sg_8v1_node_24v1_antijoindata_pos_borrow,
                                    context.is_first_run_this_tick(),
                                )
                            };
                            let op_24v1 = op_24v1.map(|(k, ())| k);
                            let op_24v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_24v1__difference__loc_dfir_rs_tests_surface_loop_rs_447_18_447_30<
                                    Item,
                                    Input: ::std::iter::Iterator<Item = Item>,
                                >(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Pull<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::std::iter::Iterator<Item = Item>,
                                    > Iterator for Pull<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn next(&mut self) -> Option<Self::Item> {
                                            self.inner.next()
                                        }
                                        #[inline(always)]
                                        fn size_hint(&self) -> (usize, Option<usize>) {
                                            self.inner.size_hint()
                                        }
                                    }
                                    Pull { inner: input }
                                }
                                op_24v1__difference__loc_dfir_rs_tests_surface_loop_rs_447_18_447_30(
                                    op_24v1,
                                )
                            };
                            let op_25v1 = ::dfir_rs::pusherator::for_each::ForEach::new(|
                                v|
                            {
                                ::std::io::_print(format_args!("difference {0:?}\n", v));
                            });
                            let op_25v1 = {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                pub fn op_25v1__for_each__loc_dfir_rs_tests_surface_loop_rs_447_34_447_78<
                                    Item,
                                    Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                >(
                                    input: Input,
                                ) -> impl ::dfir_rs::pusherator::Pusherator<Item = Item> {
                                    #[repr(transparent)]
                                    struct Push<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    > {
                                        inner: Input,
                                    }
                                    impl<
                                        Item,
                                        Input: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                    > ::dfir_rs::pusherator::Pusherator for Push<Item, Input> {
                                        type Item = Item;
                                        #[inline(always)]
                                        fn give(&mut self, item: Self::Item) {
                                            self.inner.give(item)
                                        }
                                    }
                                    Push { inner: input }
                                }
                                op_25v1__for_each__loc_dfir_rs_tests_surface_loop_rs_447_34_447_78(
                                    op_25v1,
                                )
                            };
                            #[inline(always)]
                            fn pivot_run_sg_8v1<
                                Pull: ::std::iter::Iterator<Item = Item>,
                                Push: ::dfir_rs::pusherator::Pusherator<Item = Item>,
                                Item,
                            >(pull: Pull, push: Push) {
                                ::dfir_rs::pusherator::pivot::Pivot::new(pull, push).run();
                            }
                            pivot_run_sg_8v1(op_24v1, op_25v1);
                            context.allow_another_iteration();
                            context.allow_another_iteration();
                        },
                    );
                df
            }
        }
    };
    df.run_available();
}
#[rustc_main]
#[coverage(off)]
#[doc(hidden)]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(
        &[
            &test_flo_nested,
            &test_flo_repeat_kmeans,
            &test_flo_repeat_n,
            &test_flo_repeat_n_multiple_nested,
            &test_flo_repeat_n_nested,
            &test_flo_syntax,
            &test_loop_lifetime_fold,
            &test_loop_lifetime_reduce,
            &test_state_codegen,
        ],
    )
}
