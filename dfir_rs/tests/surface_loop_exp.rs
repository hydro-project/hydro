use dfir_rs::util::{collect_ready, iter_batches_stream};
use dfir_rs::{assert_graphvis_snapshots, dfir_syntax};
use multiplatform_test::multiplatform_test;

#[multiplatform_test(test, wasm, env_tracing)]
pub fn test_flo_repeat_kmeans() {
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

    let mut df = {
        {
            #[allow(unused_qualifications)]
            {
                use::dfir_rs::{
                    var_expr,var_args
                };
                let mut df =  ::dfir_rs::scheduled::graph::Dfir::new();
                df.__assign_meta_graph("{\"nodes\":[{\"value\":null,\"version\":0},{\"value\":{\"Operator\":\"source_iter (POINTS)\"},\"version\":1},{\"value\":{\"Operator\":\"map (std :: clone :: Clone :: clone)\"},\"version\":1},{\"value\":{\"Operator\":\"source_iter (CENTROIDS)\"},\"version\":1},{\"value\":{\"Operator\":\"map (std :: clone :: Clone :: clone)\"},\"version\":1},{\"value\":{\"Operator\":\"batch ()\"},\"version\":1},{\"value\":{\"Operator\":\"batch ()\"},\"version\":1},{\"value\":{\"Operator\":\"repeat_n (10)\"},\"version\":1},{\"value\":{\"Operator\":\"all_once ()\"},\"version\":1},{\"value\":{\"Operator\":\"union ()\"},\"version\":1},{\"value\":{\"Operator\":\"identity :: < [i32 ; 2] > ()\"},\"version\":1},{\"value\":{\"Operator\":\"tee ()\"},\"version\":1},{\"value\":{\"Operator\":\"cross_join_multiset ()\"},\"version\":1},{\"value\":{\"Operator\":\"map (| (point , centroid) : ([i32 ; 2] , [i32 ; 2]) | {let dist2 = (point [0] - centroid [0]) . pow (2) + (point [1] - centroid [1]) . pow (2) ; (point , (dist2 , centroid))})\"},\"version\":1},{\"value\":{\"Operator\":\"reduce_keyed (| (a_dist2 , a_centroid) , (b_dist2 , b_centroid) | {if b_dist2 < * a_dist2 {* a_dist2 = b_dist2 ; * a_centroid = b_centroid ;}})\"},\"version\":1},{\"value\":{\"Operator\":\"map (| (point , (_dist2 , centroid)) | {(centroid , (point , 1))})\"},\"version\":1},{\"value\":{\"Operator\":\"reduce_keyed (| (p1 , n1) , (p2 , n2) : ([i32 ; 2] , i32) | {p1 [0] += p2 [0] ; p1 [1] += p2 [1] ; * n1 += n2 ;})\"},\"version\":1},{\"value\":{\"Operator\":\"map (| (_centroid , (p , n)) : (_ , ([i32 ; 2] , i32)) | {[p [0] / n , p [1] / n]})\"},\"version\":1},{\"value\":{\"Operator\":\"next_iteration ()\"},\"version\":1},{\"value\":{\"Operator\":\"inspect (| x | println ! (\\\"centroid: {:?}\\\" , x))\"},\"version\":1},{\"value\":{\"Operator\":\"last_iteration ()\"},\"version\":1},{\"value\":{\"Operator\":\"for_each (| x | println ! (\\\"XXX {:?}\\\" , x))\"},\"version\":1},{\"value\":{\"Handoff\":{\"is_lazy\":false}},\"version\":1},{\"value\":{\"Handoff\":{\"is_lazy\":false}},\"version\":1},{\"value\":{\"Handoff\":{\"is_lazy\":false}},\"version\":1},{\"value\":{\"Handoff\":{\"is_lazy\":false}},\"version\":1},{\"value\":{\"Handoff\":{\"is_lazy\":true}},\"version\":1},{\"value\":{\"Handoff\":{\"is_lazy\":false}},\"version\":1}],\"operator_tag\":[{\"value\":null,\"version\":0}],\"graph\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":1,\"version\":1},{\"idx\":2,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":3,\"version\":1},{\"idx\":4,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":2,\"version\":1},{\"idx\":22,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":4,\"version\":1},{\"idx\":23,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":7,\"version\":1},{\"idx\":12,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":5,\"version\":1},{\"idx\":24,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":8,\"version\":1},{\"idx\":9,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":6,\"version\":1},{\"idx\":25,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":10,\"version\":1},{\"idx\":11,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":9,\"version\":1},{\"idx\":10,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":11,\"version\":1},{\"idx\":12,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":19,\"version\":1},{\"idx\":9,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":18,\"version\":1},{\"idx\":19,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":17,\"version\":1},{\"idx\":26,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":16,\"version\":1},{\"idx\":17,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":15,\"version\":1},{\"idx\":16,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":14,\"version\":1},{\"idx\":15,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":13,\"version\":1},{\"idx\":14,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":12,\"version\":1},{\"idx\":13,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":20,\"version\":1},{\"idx\":21,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":11,\"version\":1},{\"idx\":27,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":22,\"version\":1},{\"idx\":5,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":23,\"version\":1},{\"idx\":6,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":24,\"version\":1},{\"idx\":7,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":25,\"version\":1},{\"idx\":8,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":26,\"version\":1},{\"idx\":18,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":27,\"version\":1},{\"idx\":20,\"version\":1}],\"version\":1}],\"ports\":[{\"value\":null,\"version\":0},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",{\"Int\":\"0\"}],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",{\"Int\":\"1\"}],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1}],\"node_loops\":[{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1}],\"loop_nodes\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":5,\"version\":1},{\"idx\":6,\"version\":1},{\"idx\":20,\"version\":1},{\"idx\":21,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":7,\"version\":1},{\"idx\":8,\"version\":1},{\"idx\":9,\"version\":1},{\"idx\":10,\"version\":1},{\"idx\":11,\"version\":1},{\"idx\":12,\"version\":1},{\"idx\":13,\"version\":1},{\"idx\":14,\"version\":1},{\"idx\":15,\"version\":1},{\"idx\":16,\"version\":1},{\"idx\":17,\"version\":1},{\"idx\":18,\"version\":1},{\"idx\":19,\"version\":1}],\"version\":1}],\"loop_parent\":[{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":{\"idx\":1,\"version\":1},\"version\":1}],\"root_loops\":[{\"idx\":1,\"version\":1}],\"loop_children\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":2,\"version\":1}],\"version\":1},{\"value\":[],\"version\":1}],\"node_subgraph\":[{\"value\":null,\"version\":0},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1},{\"value\":{\"idx\":4,\"version\":1},\"version\":1},{\"value\":{\"idx\":5,\"version\":1},\"version\":1},{\"value\":{\"idx\":5,\"version\":1},\"version\":1},{\"value\":{\"idx\":5,\"version\":1},\"version\":1},{\"value\":{\"idx\":5,\"version\":1},\"version\":1},{\"value\":{\"idx\":5,\"version\":1},\"version\":1},{\"value\":{\"idx\":5,\"version\":1},\"version\":1},{\"value\":{\"idx\":5,\"version\":1},\"version\":1},{\"value\":{\"idx\":5,\"version\":1},\"version\":1},{\"value\":{\"idx\":5,\"version\":1},\"version\":1},{\"value\":{\"idx\":5,\"version\":1},\"version\":1},{\"value\":{\"idx\":5,\"version\":1},\"version\":1},{\"value\":{\"idx\":5,\"version\":1},\"version\":1},{\"value\":{\"idx\":5,\"version\":1},\"version\":1},{\"value\":{\"idx\":6,\"version\":1},\"version\":1},{\"value\":{\"idx\":6,\"version\":1},\"version\":1}],\"subgraph_nodes\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":1,\"version\":1},{\"idx\":2,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":3,\"version\":1},{\"idx\":4,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":5,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":6,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":7,\"version\":1},{\"idx\":8,\"version\":1},{\"idx\":18,\"version\":1},{\"idx\":19,\"version\":1},{\"idx\":9,\"version\":1},{\"idx\":10,\"version\":1},{\"idx\":11,\"version\":1},{\"idx\":12,\"version\":1},{\"idx\":13,\"version\":1},{\"idx\":14,\"version\":1},{\"idx\":15,\"version\":1},{\"idx\":16,\"version\":1},{\"idx\":17,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":20,\"version\":1},{\"idx\":21,\"version\":1}],\"version\":1}],\"subgraph_stratum\":[{\"value\":null,\"version\":0},{\"value\":0,\"version\":1},{\"value\":0,\"version\":1},{\"value\":1,\"version\":1},{\"value\":1,\"version\":1},{\"value\":2,\"version\":1},{\"value\":3,\"version\":1}],\"node_singleton_references\":[{\"value\":null,\"version\":0},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1}],\"node_varnames\":[{\"value\":null,\"version\":0},{\"value\":\"init_points\",\"version\":1},{\"value\":\"init_points\",\"version\":1},{\"value\":\"init_centroids\",\"version\":1},{\"value\":\"init_centroids\",\"version\":1},{\"value\":\"batch_points\",\"version\":1},{\"value\":\"batch_centroids\",\"version\":1},{\"value\":\"points\",\"version\":1},{\"value\":null,\"version\":0},{\"value\":\"centroids\",\"version\":1},{\"value\":\"centroids\",\"version\":1},{\"value\":\"centroids\",\"version\":1},{\"value\":\"cj\",\"version\":1},{\"value\":\"cj\",\"version\":1},{\"value\":\"cj\",\"version\":1},{\"value\":\"cj\",\"version\":1},{\"value\":\"cj\",\"version\":1},{\"value\":\"cj\",\"version\":1},{\"value\":\"cj\",\"version\":1},{\"value\":\"cj\",\"version\":1}],\"subgraph_laziness\":[{\"value\":null,\"version\":0}]}");
                df.__assign_diagnostics("[]");
                let(hoff_22v1_send,hoff_22v1_recv) = df.make_edge:: <_, ::dfir_rs::scheduled::handoff::VecHandoff<_,false>>("handoff GraphNodeId(22v1)");
                let(hoff_23v1_send,hoff_23v1_recv) = df.make_edge:: <_, ::dfir_rs::scheduled::handoff::VecHandoff<_,false>>("handoff GraphNodeId(23v1)");
                let(hoff_24v1_send,hoff_24v1_recv) = df.make_edge:: <_, ::dfir_rs::scheduled::handoff::VecHandoff<_,false>>("handoff GraphNodeId(24v1)");
                let(hoff_25v1_send,hoff_25v1_recv) = df.make_edge:: <_, ::dfir_rs::scheduled::handoff::VecHandoff<_,false>>("handoff GraphNodeId(25v1)");
                let(hoff_26v1_send,hoff_26v1_recv) = df.make_edge:: <_, ::dfir_rs::scheduled::handoff::VecHandoff<_,true>>("handoff GraphNodeId(26v1)");
                let(hoff_27v1_send,hoff_27v1_recv) = df.make_edge:: <_, ::dfir_rs::scheduled::handoff::VecHandoff<_,false>>("handoff GraphNodeId(27v1)");
                let loop_1v1 = df.add_loop(None);
                let loop_2v1 = df.add_loop(Some(loop_1v1));
                let mut sg_1v1_node_1v1_iter = {
                    #[inline(always)]
                    fn check_iter<IntoIter: ::std::iter::IntoIterator<Item = Item> ,Item>(into_iter:IntoIter) -> impl ::std::iter::Iterator<Item = Item>{
                        ::std::iter::IntoIterator::into_iter(into_iter)
                    }
                    check_iter(POINTS)
                };
                let mut sg_2v1_node_3v1_iter = {
                    #[inline(always)]
                    fn check_iter<IntoIter: ::std::iter::IntoIterator<Item = Item> ,Item>(into_iter:IntoIter) -> impl ::std::iter::Iterator<Item = Item>{
                        ::std::iter::IntoIterator::into_iter(into_iter)
                    }
                    check_iter(CENTROIDS)
                };
                #[allow(clippy::redundant_closure_call)]
                let singleton_op_7v1 = df.add_state(::std::cell::RefCell::new(::std::vec::Vec::new()));
                df.set_state_tick_hook(singleton_op_7v1,move|rcell|{
                    rcell.take();
                });
                let sg_5v1_node_14v1_groupbydata = df.add_state(::std::cell::RefCell::new(::dfir_rs::rustc_hash::FxHashMap:: <_,_> ::default()));
                let sg_5v1_node_16v1_groupbydata = df.add_state(::std::cell::RefCell::new(::dfir_rs::rustc_hash::FxHashMap:: <_,_> ::default()));
                df.add_subgraph_full("Subgraph GraphSubgraphId(1v1)",0,var_expr!(),var_expr!(hoff_22v1_send),false,None,move|context,var_args!(),var_args!(hoff_22v1_send)|{
                    let hoff_22v1_send =  ::dfir_rs::pusherator::for_each::ForEach::new(|v|{
                        hoff_22v1_send.give(Some(v));
                    });
                    let op_1v1 = sg_1v1_node_1v1_iter.by_ref();
                    let op_1v1 = {
                        #[allow(non_snake_case)]
                        #[inline(always)]
                        pub fn op_1v1__source_iter__loc__1_1_1_1<Item,Input: ::std::iter::Iterator<Item = Item>>(input:Input) -> impl ::std::iter::Iterator<Item = Item>{
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
                        op_1v1__source_iter__loc__1_1_1_1(op_1v1)
                    };
                    #[allow(clippy::map_clone,reason = "dfir has no explicit `cloned`/`copied` operator")]
                    let op_2v1 = op_1v1.map(std::clone::Clone::clone);
                    let op_2v1 = {
                        #[allow(non_snake_case)]
                        #[inline(always)]
                        pub fn op_2v1__map__loc__1_1_1_1<Item,Input: ::std::iter::Iterator<Item = Item>>(input:Input) -> impl ::std::iter::Iterator<Item = Item>{
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
                        op_2v1__map__loc__1_1_1_1(op_2v1)
                    };
                    #[inline(always)]
                    fn pivot_run_sg_1v1<Pull: ::std::iter::Iterator<Item = Item> ,Push: ::dfir_rs::pusherator::Pusherator<Item = Item> ,Item>(pull:Pull,push:Push){
                        ::dfir_rs::pusherator::pivot::Pivot::new(pull,push).run();
                    }
                    pivot_run_sg_1v1(op_2v1,hoff_22v1_send);
                },);
                df.add_subgraph_full("Subgraph GraphSubgraphId(2v1)",0,var_expr!(),var_expr!(hoff_23v1_send),false,None,move|context,var_args!(),var_args!(hoff_23v1_send)|{
                    let hoff_23v1_send =  ::dfir_rs::pusherator::for_each::ForEach::new(|v|{
                        hoff_23v1_send.give(Some(v));
                    });
                    let op_3v1 = sg_2v1_node_3v1_iter.by_ref();
                    let op_3v1 = {
                        #[allow(non_snake_case)]
                        #[inline(always)]
                        pub fn op_3v1__source_iter__loc__1_1_1_1<Item,Input: ::std::iter::Iterator<Item = Item>>(input:Input) -> impl ::std::iter::Iterator<Item = Item>{
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
                        op_3v1__source_iter__loc__1_1_1_1(op_3v1)
                    };
                    #[allow(clippy::map_clone,reason = "dfir has no explicit `cloned`/`copied` operator")]
                    let op_4v1 = op_3v1.map(std::clone::Clone::clone);
                    let op_4v1 = {
                        #[allow(non_snake_case)]
                        #[inline(always)]
                        pub fn op_4v1__map__loc__1_1_1_1<Item,Input: ::std::iter::Iterator<Item = Item>>(input:Input) -> impl ::std::iter::Iterator<Item = Item>{
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
                        op_4v1__map__loc__1_1_1_1(op_4v1)
                    };
                    #[inline(always)]
                    fn pivot_run_sg_2v1<Pull: ::std::iter::Iterator<Item = Item> ,Push: ::dfir_rs::pusherator::Pusherator<Item = Item> ,Item>(pull:Pull,push:Push){
                        ::dfir_rs::pusherator::pivot::Pivot::new(pull,push).run();
                    }
                    pivot_run_sg_2v1(op_4v1,hoff_23v1_send);
                },);
                df.add_subgraph_full("Subgraph GraphSubgraphId(3v1)",1,var_expr!(hoff_22v1_recv),var_expr!(hoff_24v1_send),false,Some(loop_1v1),move|context,var_args!(hoff_22v1_recv),var_args!(hoff_24v1_send)|{
                    let mut hoff_22v1_recv = hoff_22v1_recv.borrow_mut_swap();
                    let hoff_22v1_recv = hoff_22v1_recv.drain(..);
                    let hoff_24v1_send =  ::dfir_rs::pusherator::for_each::ForEach::new(|v|{
                        hoff_24v1_send.give(Some(v));
                    });
                    let op_5v1 = {
                        fn check_input<Iter: ::std::iter::Iterator<Item = Item> ,Item>(iter:Iter) -> impl ::std::iter::Iterator<Item = Item>{
                            iter
                        }
                        check_input:: <_,_>(hoff_22v1_recv)
                    };
                    let op_5v1 = {
                        #[allow(non_snake_case)]
                        #[inline(always)]
                        pub fn op_5v1__batch__loc__1_1_1_1<Item,Input: ::std::iter::Iterator<Item = Item>>(input:Input) -> impl ::std::iter::Iterator<Item = Item>{
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
                        op_5v1__batch__loc__1_1_1_1(op_5v1)
                    };
                    #[inline(always)]
                    fn pivot_run_sg_3v1<Pull: ::std::iter::Iterator<Item = Item> ,Push: ::dfir_rs::pusherator::Pusherator<Item = Item> ,Item>(pull:Pull,push:Push){
                        ::dfir_rs::pusherator::pivot::Pivot::new(pull,push).run();
                    }
                    pivot_run_sg_3v1(op_5v1,hoff_24v1_send);
                },);
                df.add_subgraph_full("Subgraph GraphSubgraphId(4v1)",1,var_expr!(hoff_23v1_recv),var_expr!(hoff_25v1_send),false,Some(loop_1v1),move|context,var_args!(hoff_23v1_recv),var_args!(hoff_25v1_send)|{
                    let mut hoff_23v1_recv = hoff_23v1_recv.borrow_mut_swap();
                    let hoff_23v1_recv = hoff_23v1_recv.drain(..);
                    let hoff_25v1_send =  ::dfir_rs::pusherator::for_each::ForEach::new(|v|{
                        hoff_25v1_send.give(Some(v));
                    });
                    let op_6v1 = {
                        fn check_input<Iter: ::std::iter::Iterator<Item = Item> ,Item>(iter:Iter) -> impl ::std::iter::Iterator<Item = Item>{
                            iter
                        }
                        check_input:: <_,_>(hoff_23v1_recv)
                    };
                    let op_6v1 = {
                        #[allow(non_snake_case)]
                        #[inline(always)]
                        pub fn op_6v1__batch__loc__1_1_1_1<Item,Input: ::std::iter::Iterator<Item = Item>>(input:Input) -> impl ::std::iter::Iterator<Item = Item>{
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
                        op_6v1__batch__loc__1_1_1_1(op_6v1)
                    };
                    #[inline(always)]
                    fn pivot_run_sg_4v1<Pull: ::std::iter::Iterator<Item = Item> ,Push: ::dfir_rs::pusherator::Pusherator<Item = Item> ,Item>(pull:Pull,push:Push){
                        ::dfir_rs::pusherator::pivot::Pivot::new(pull,push).run();
                    }
                    pivot_run_sg_4v1(op_6v1,hoff_25v1_send);
                },);
                df.add_subgraph_full("Subgraph GraphSubgraphId(5v1)",2,var_expr!(hoff_24v1_recv,hoff_25v1_recv,hoff_26v1_recv),var_expr!(hoff_26v1_send,hoff_27v1_send),false,Some(loop_2v1),move|context,var_args!(hoff_24v1_recv,hoff_25v1_recv,hoff_26v1_recv),var_args!(hoff_26v1_send,hoff_27v1_send)|{
                    let mut hoff_24v1_recv = hoff_24v1_recv.borrow_mut_swap();
                    let hoff_24v1_recv = hoff_24v1_recv.drain(..);
                    let mut hoff_25v1_recv = hoff_25v1_recv.borrow_mut_swap();
                    let hoff_25v1_recv = hoff_25v1_recv.drain(..);
                    let mut hoff_26v1_recv = hoff_26v1_recv.borrow_mut_swap();
                    let hoff_26v1_recv = hoff_26v1_recv.drain(..);
                    let hoff_26v1_send =  ::dfir_rs::pusherator::for_each::ForEach::new(|v|{
                        hoff_26v1_send.give(Some(v));
                    });
                    let hoff_27v1_send =  ::dfir_rs::pusherator::for_each::ForEach::new(|v: [i32; 2]|{
                        hoff_27v1_send.give(Some(v));
                    });
                    let mut sg_5v1_node_7v1_vec = context.state_ref(singleton_op_7v1).borrow_mut();
                    if 0==context.loop_iter_count(){
                        *sg_5v1_node_7v1_vec = hoff_24v1_recv.collect:: < ::std::vec::Vec<_>>();
                    }let op_7v1 = std::iter::IntoIterator::into_iter(::std::clone::Clone::clone(& *sg_5v1_node_7v1_vec));
                    let op_7v1 = {
                        #[allow(non_snake_case)]
                        #[inline(always)]
                        pub fn op_7v1__repeat_n__loc__1_1_1_1<Item,Input: ::std::iter::Iterator<Item = Item>>(input:Input) -> impl ::std::iter::Iterator<Item = Item>{
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
                        op_7v1__repeat_n__loc__1_1_1_1(op_7v1)
                    };
                    let op_8v1 = {
                        fn check_input<Iter: ::std::iter::Iterator<Item = Item> ,Item>(iter:Iter) -> impl ::std::iter::Iterator<Item = Item>{
                            iter
                        }
                        check_input:: <_,_>(hoff_25v1_recv)
                    };
                    let op_8v1 = {
                        #[allow(non_snake_case)]
                        #[inline(always)]
                        pub fn op_8v1__all_once__loc__1_1_1_1<Item,Input: ::std::iter::Iterator<Item = Item>>(input:Input) -> impl ::std::iter::Iterator<Item = Item>{
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
                        op_8v1__all_once__loc__1_1_1_1(op_8v1)
                    };
                    let op_18v1 =  ::std::iter::Iterator::filter(hoff_26v1_recv, |_|0!=context.loop_iter_count());
                    let op_18v1 = {
                        #[allow(non_snake_case)]
                        #[inline(always)]
                        pub fn op_18v1__next_iteration__loc__1_1_1_1<Item,Input: ::std::iter::Iterator<Item = Item>>(input:Input) -> impl ::std::iter::Iterator<Item = Item>{
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
                        op_18v1__next_iteration__loc__1_1_1_1(op_18v1)
                    };
                    let op_19v1 = op_18v1.inspect(|x|{
                        println!("centroid: {:?}",x);
                    });
                    let op_19v1 = {
                        #[allow(non_snake_case)]
                        #[inline(always)]
                        pub fn op_19v1__inspect__loc__1_1_1_1<Item,Input: ::std::iter::Iterator<Item = Item>>(input:Input) -> impl ::std::iter::Iterator<Item = Item>{
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
                        op_19v1__inspect__loc__1_1_1_1(op_19v1)
                    };
                    let op_9v1 = {
                        #[allow(unused)]
                        #[inline(always)]
                        fn check_inputs<A: ::std::iter::Iterator<Item = Item> ,B: ::std::iter::Iterator<Item = Item> ,Item>(a:A,b:B) -> impl ::std::iter::Iterator<Item = Item>{
                            a.chain(b)
                        }
                        check_inputs(op_8v1,op_19v1)
                    };
                    let op_9v1 = {
                        #[allow(non_snake_case)]
                        #[inline(always)]
                        pub fn op_9v1__union__loc__1_1_1_1<Item,Input: ::std::iter::Iterator<Item = Item>>(input:Input) -> impl ::std::iter::Iterator<Item = Item>{
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
                        op_9v1__union__loc__1_1_1_1(op_9v1)
                    };
                    let op_10v1 = {
                        fn check_input<Iter: ::std::iter::Iterator<Item = Item> ,Item>(iter:Iter) -> impl ::std::iter::Iterator<Item = Item>{
                            iter
                        }
                        check_input:: <_,[i32;
                        2]>(op_9v1)
                    };
                    let op_10v1 = {
                        #[allow(non_snake_case)]
                        #[inline(always)]
                        pub fn op_10v1__identity__loc__1_1_1_1<Item,Input: ::std::iter::Iterator<Item = Item>>(input:Input) -> impl ::std::iter::Iterator<Item = Item>{
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
                        op_10v1__identity__loc__1_1_1_1(op_10v1)
                    };
                    let op_11v1 = op_10v1;
                    let op_11v1 = {
                        #[allow(non_snake_case)]
                        #[inline(always)]
                        pub fn op_11v1__tee__loc__1_1_1_1<Item,Input: ::std::iter::Iterator<Item = Item>>(input:Input) -> impl ::std::iter::Iterator<Item = Item>{
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
                        op_11v1__tee__loc__1_1_1_1(op_11v1)
                    };
                    let op_7v1 = op_7v1.map(|a|((),a));
                    let op_11v1 = op_11v1.map(|b|((),b));
                    let mut sg_5v1_node_12v1_joindata_lhs_borrow =  ::std::default::Default::default();
                    let mut sg_5v1_node_12v1_joindata_rhs_borrow =  ::std::default::Default::default();
                    let op_12v1 = {
                        #[inline(always)]
                        fn check_inputs<'a,K,I1,V1,I2,V2>(lhs:I1,rhs:I2,lhs_state: &'a mut ::dfir_rs::compiled::pull::HalfMultisetJoinState<K,V1,V2> ,rhs_state: &'a mut ::dfir_rs::compiled::pull::HalfMultisetJoinState<K,V2,V1> ,is_new_tick:bool,) -> impl 'a+Iterator<Item = (K,(V1,V2))>where K:Eq+std::hash::Hash+Clone,V1:Clone,V2:Clone,I1:'a+Iterator<Item = (K,V1)> ,I2:'a+Iterator<Item = (K,V2)> ,{
                            ::dfir_rs::compiled::pull::symmetric_hash_join_into_iter(lhs,rhs,lhs_state,rhs_state,is_new_tick)
                        }
                        check_inputs(op_7v1,op_11v1, &mut sg_5v1_node_12v1_joindata_lhs_borrow, &mut sg_5v1_node_12v1_joindata_rhs_borrow,true)
                    };
                    let op_12v1 = op_12v1.map(|((),(a,b))|(a,b));
                    let op_12v1 = {
                        #[allow(non_snake_case)]
                        #[inline(always)]
                        pub fn op_12v1__cross_join_multiset__loc__1_1_1_1<Item,Input: ::std::iter::Iterator<Item = Item>>(input:Input) -> impl ::std::iter::Iterator<Item = Item>{
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
                        op_12v1__cross_join_multiset__loc__1_1_1_1(op_12v1)
                    };
                    #[allow(clippy::map_clone,reason = "dfir has no explicit `cloned`/`copied` operator")]
                    let op_13v1 = op_12v1.map(|(point,centroid):([i32;
                    2],[i32;
                    2])|{
                        let dist2 = (point[0]-centroid[0]).pow(2)+(point[1]-centroid[1]).pow(2);
                        (point,(dist2,centroid))
                    });
                    let op_13v1 = {
                        #[allow(non_snake_case)]
                        #[inline(always)]
                        pub fn op_13v1__map__loc__1_1_1_1<Item,Input: ::std::iter::Iterator<Item = Item>>(input:Input) -> impl ::std::iter::Iterator<Item = Item>{
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
                        op_13v1__map__loc__1_1_1_1(op_13v1)
                    };
                    let mut sg_5v1_node_14v1_hashtable = context.state_ref(sg_5v1_node_14v1_groupbydata).borrow_mut();
                    {
                        #[inline(always)]
                        fn check_input<Iter: ::std::iter::Iterator<Item = (A,B)> ,A: ::std::clone::Clone,B: ::std::clone::Clone>(iter:Iter) -> impl ::std::iter::Iterator<Item = (A,B)>{
                            iter
                        }
                        #[inline(always)]
                        #[doc = r" A: accumulator type"]
                        #[doc = r" O: output type"]
                        fn call_comb_type<A,O>(acc: &mut A,item:A,f:impl Fn(&mut A,A) -> O) -> O {
                            f(acc,item)
                        }
                        for kv in check_input(op_13v1){
                            match sg_5v1_node_14v1_hashtable.entry(kv.0){
                                ::std::collections::hash_map::Entry::Vacant(vacant) => {
                                    vacant.insert(kv.1);
                                }
                                ::std::collections::hash_map::Entry::Occupied(mut occupied) => {
                                    #[allow(clippy::redundant_closure_call)]
                                    call_comb_type(occupied.get_mut(),kv.1, |(a_dist2,a_centroid),(b_dist2,b_centroid)|{
                                        if b_dist2< *a_dist2 {
                                            *a_dist2 = b_dist2;
                                            *a_centroid = b_centroid;
                                        }
                                    });
                                }
                            
                                }
                        }
                    }let op_14v1 = sg_5v1_node_14v1_hashtable.drain();
                    let op_14v1 = {
                        #[allow(non_snake_case)]
                        #[inline(always)]
                        pub fn op_14v1__reduce_keyed__loc__1_1_1_1<Item,Input: ::std::iter::Iterator<Item = Item>>(input:Input) -> impl ::std::iter::Iterator<Item = Item>{
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
                        op_14v1__reduce_keyed__loc__1_1_1_1(op_14v1)
                    };
                    #[allow(clippy::map_clone,reason = "dfir has no explicit `cloned`/`copied` operator")]
                    let op_15v1 = op_14v1.map(|(point,(_dist2,centroid))|{
                        (centroid,(point,1))
                    });
                    let op_15v1 = {
                        #[allow(non_snake_case)]
                        #[inline(always)]
                        pub fn op_15v1__map__loc__1_1_1_1<Item,Input: ::std::iter::Iterator<Item = Item>>(input:Input) -> impl ::std::iter::Iterator<Item = Item>{
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
                        op_15v1__map__loc__1_1_1_1(op_15v1)
                    };
                    let mut sg_5v1_node_16v1_hashtable = context.state_ref(sg_5v1_node_16v1_groupbydata).borrow_mut();
                    {
                        #[inline(always)]
                        fn check_input<Iter: ::std::iter::Iterator<Item = (A,B)> ,A: ::std::clone::Clone,B: ::std::clone::Clone>(iter:Iter) -> impl ::std::iter::Iterator<Item = (A,B)>{
                            iter
                        }
                        #[inline(always)]
                        #[doc = r" A: accumulator type"]
                        #[doc = r" O: output type"]
                        fn call_comb_type<A,O>(acc: &mut A,item:A,f:impl Fn(&mut A,A) -> O) -> O {
                            f(acc,item)
                        }
                        for kv in check_input(op_15v1){
                            match sg_5v1_node_16v1_hashtable.entry(kv.0){
                                ::std::collections::hash_map::Entry::Vacant(vacant) => {
                                    vacant.insert(kv.1);
                                }
                                ::std::collections::hash_map::Entry::Occupied(mut occupied) => {
                                    #[allow(clippy::redundant_closure_call)]
                                    call_comb_type(occupied.get_mut(),kv.1, |(p1,n1),(p2,n2):([i32;
                                    2],i32)|{
                                        p1[0]+=p2[0];
                                        p1[1]+=p2[1];
                                        *n1+=n2;
                                    });
                                }
                            
                                }
                        }
                    }let op_16v1 = sg_5v1_node_16v1_hashtable.drain();
                    let op_16v1 = {
                        #[allow(non_snake_case)]
                        #[inline(always)]
                        pub fn op_16v1__reduce_keyed__loc__1_1_1_1<Item,Input: ::std::iter::Iterator<Item = Item>>(input:Input) -> impl ::std::iter::Iterator<Item = Item>{
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
                        op_16v1__reduce_keyed__loc__1_1_1_1(op_16v1)
                    };
                    let op_17v1 =  ::dfir_rs::pusherator::map::Map::new(|(_centroid,(p,n)):(_,([i32;
                    2],i32))|{
                        [p[0]/n,p[1]/n]
                    },hoff_26v1_send);
                    let op_17v1 = {
                        #[allow(non_snake_case)]
                        #[inline(always)]
                        pub fn op_17v1__map__loc__1_1_1_1<Item,Input: ::dfir_rs::pusherator::Pusherator<Item = Item>>(input:Input) -> impl ::dfir_rs::pusherator::Pusherator<Item = Item>{
                            #[repr(transparent)]
                            struct Push<Item,Input: ::dfir_rs::pusherator::Pusherator<Item = Item>>{
                                inner:Input
                            }
                            impl <Item,Input: ::dfir_rs::pusherator::Pusherator<Item = Item>> ::dfir_rs::pusherator::Pusherator for Push<Item,Input>{
                                type Item = Item;
                                #[inline(always)]
                                fn give(&mut self,item:Self::Item){
                                    self.inner.give(item)
                                }
                            
                                }
                            Push {
                                inner:input
                            }
                        }
                        op_17v1__map__loc__1_1_1_1(op_17v1)
                    };
                    #[inline(always)]
                    fn pivot_run_sg_5v1<Pull: ::std::iter::Iterator<Item = Item> ,Push: ::dfir_rs::pusherator::Pusherator<Item = Item> ,Item>(pull:Pull,push:Push){
                        ::dfir_rs::pusherator::pivot::Pivot::new(pull,push).run();
                    }
                    pivot_run_sg_5v1(op_16v1,op_17v1);
                    {
                        if context.loop_iter_count()+1<10 {
                            context.reschedule_loop_block();
                        }
                    }
                },);
                df.add_subgraph_full("Subgraph GraphSubgraphId(6v1)",3,var_expr!(hoff_27v1_recv),var_expr!(),false,Some(loop_1v1),move|context,var_args!(hoff_27v1_recv),var_args!()|{
                    let mut hoff_27v1_recv = hoff_27v1_recv.borrow_mut_swap();
                    let hoff_27v1_recv = hoff_27v1_recv.drain(..);
                    let op_20v1 = {
                        fn check_input<Iter: ::std::iter::Iterator<Item = Item> ,Item>(iter:Iter) -> impl ::std::iter::Iterator<Item = Item>{
                            iter
                        }
                        check_input:: <_,_>(hoff_27v1_recv)
                    };
                    let op_20v1 = {
                        #[allow(non_snake_case)]
                        #[inline(always)]
                        pub fn op_20v1__last_iteration__loc__1_1_1_1<Item,Input: ::std::iter::Iterator<Item = Item>>(input:Input) -> impl ::std::iter::Iterator<Item = Item>{
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
                        op_20v1__last_iteration__loc__1_1_1_1(op_20v1)
                    };
                    let op_21v1 =  ::dfir_rs::pusherator::for_each::ForEach::new(|x|{
                        println!("XXX {:?}",x);
                    });
                    let op_21v1 = {
                        #[allow(non_snake_case)]
                        #[inline(always)]
                        pub fn op_21v1__for_each__loc__1_1_1_1<Item,Input: ::dfir_rs::pusherator::Pusherator<Item = Item>>(input:Input) -> impl ::dfir_rs::pusherator::Pusherator<Item = Item>{
                            #[repr(transparent)]
                            struct Push<Item,Input: ::dfir_rs::pusherator::Pusherator<Item = Item>>{
                                inner:Input
                            }
                            impl <Item,Input: ::dfir_rs::pusherator::Pusherator<Item = Item>> ::dfir_rs::pusherator::Pusherator for Push<Item,Input>{
                                type Item = Item;
                                #[inline(always)]
                                fn give(&mut self,item:Self::Item){
                                    self.inner.give(item)
                                }
                            
                                }
                            Push {
                                inner:input
                            }
                        }
                        op_21v1__for_each__loc__1_1_1_1(op_21v1)
                    };
                    #[inline(always)]
                    fn pivot_run_sg_6v1<Pull: ::std::iter::Iterator<Item = Item> ,Push: ::dfir_rs::pusherator::Pusherator<Item = Item> ,Item>(pull:Pull,push:Push){
                        ::dfir_rs::pusherator::pivot::Pivot::new(pull,push).run();
                    }
                    pivot_run_sg_6v1(op_20v1,op_21v1);
                },);
                df
            }
        }
    };
    // assert_graphvis_snapshots!(df);
    df.run_available();
}
