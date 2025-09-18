use chrono::prelude::*;
use colored::Colorize;
use dfir_rs::dfir_syntax;
use dfir_rs::util::{bind_udp_bytes, ipv4_resolve};

use crate::protocol::Message;
use crate::{Opts, default_server_address};

fn pretty_print_msg(nickname: String, message: String, ts: DateTime<Utc>) {
    println!(
        "{} {}: {}",
        ts.with_timezone(&Local)
            .format("%b %-d, %-I:%M:%S")
            .to_string()
            .truecolor(126, 126, 126)
            .italic(),
        nickname.green().italic(),
        message,
    );
}

pub(crate) async fn run_client(opts: Opts) {
    // Client listens on a port picked by the OS.
    let client_addr = ipv4_resolve("localhost:0").unwrap();

    // Use the server address that was provided in the command-line arguments, or use the default
    // if one was not provided.
    let server_addr = opts.address.unwrap_or_else(default_server_address);

    let (outbound, inbound, allocated_client_addr) = bind_udp_bytes(client_addr).await;

    println!(
        "Client is live! Listening on {:?} and talking to server on {:?}",
        allocated_client_addr, server_addr
    );

    // let mut hf = dfir_syntax! {
    //     // set up channels
    //     outbound_chan = union() -> dest_sink_serde(outbound);
    //     inbound_chan = source_stream_serde(inbound)
    //         -> map(Result::unwrap)
    //         -> map(|(msg, _addr)| msg)
    //         -> demux_enum::<Message>();
    //     inbound_chan[ConnectRequest] -> for_each(|()| println!("Received unexpected connect request from server."));

    //     // send a single connection request on startup
    //     initialize() -> map(|_m| (Message::ConnectRequest, server_addr)) -> [0]outbound_chan;

    //     // take stdin and send to server as a msg
    //     // the batch serves to buffer msgs until the connection request is acked
    //     lines = source_stdin()
    //       -> map(|l| Message::ChatMsg {
    //                 nickname: opts.name.clone(),
    //                 message: l.unwrap(),
    //                 ts: Utc::now()})
    //       -> [input]msg_send;
    //     inbound_chan[ConnectResponse] -> persist::<'static>() -> [signal]msg_send;
    //     msg_send = defer_signal() -> map(|msg| (msg, server_addr)) -> [1]outbound_chan;

    //     // receive and print messages
    //     inbound_chan[ChatMsg] -> for_each(|(nick, msg, ts)| pretty_print_msg(nick, msg, ts));
    // };
    let mut hf = {
        {
            #[allow(unused_qualifications, clippy::await_holding_refcell_ref)]
            {
                use ::dfir_rs::{var_args, var_expr};
                let mut df = ::dfir_rs::scheduled::graph::Dfir::new();
                df.__assign_meta_graph("{\"nodes\":[{\"value\":null,\"version\":0},{\"value\":{\"Operator\":\"union ()\"},\"version\":1},{\"value\":{\"Operator\":\"dest_sink_serde (outbound)\"},\"version\":1},{\"value\":{\"Operator\":\"source_stream_serde (inbound)\"},\"version\":1},{\"value\":{\"Operator\":\"map (Result :: unwrap)\"},\"version\":1},{\"value\":{\"Operator\":\"map (| (msg , _addr) | msg)\"},\"version\":1},{\"value\":{\"Operator\":\"demux_enum :: < Message > ()\"},\"version\":1},{\"value\":{\"Operator\":\"for_each (| () | println ! (\\\"Received unexpected connect request from server.\\\"))\"},\"version\":1},{\"value\":{\"Operator\":\"initialize ()\"},\"version\":1},{\"value\":{\"Operator\":\"map (| _m | (Message :: ConnectRequest , server_addr))\"},\"version\":1},{\"value\":{\"Operator\":\"source_stdin ()\"},\"version\":1},{\"value\":{\"Operator\":\"map (| l | Message :: ChatMsg {nickname : opts . name . clone () , message : l . unwrap () , ts : Utc :: now ()})\"},\"version\":1},{\"value\":{\"Operator\":\"persist :: < 'static > ()\"},\"version\":1},{\"value\":{\"Operator\":\"defer_signal ()\"},\"version\":1},{\"value\":{\"Operator\":\"map (| msg | (msg , server_addr))\"},\"version\":1},{\"value\":{\"Operator\":\"for_each (| (nick , msg , ts) | pretty_print_msg (nick , msg , ts))\"},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1},{\"value\":{\"Handoff\":{}},\"version\":1}],\"operator_tag\":[{\"value\":null,\"version\":0}],\"graph\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":1,\"version\":1},{\"idx\":2,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":5,\"version\":1},{\"idx\":6,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":4,\"version\":1},{\"idx\":5,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":3,\"version\":1},{\"idx\":4,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":6,\"version\":1},{\"idx\":7,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":9,\"version\":1},{\"idx\":1,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":8,\"version\":1},{\"idx\":9,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":11,\"version\":1},{\"idx\":16,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":10,\"version\":1},{\"idx\":11,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":12,\"version\":1},{\"idx\":17,\"version\":1}],\"version\":3},{\"value\":[{\"idx\":6,\"version\":1},{\"idx\":12,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":14,\"version\":1},{\"idx\":1,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":13,\"version\":1},{\"idx\":14,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":6,\"version\":1},{\"idx\":15,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":16,\"version\":1},{\"idx\":13,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":17,\"version\":1},{\"idx\":13,\"version\":1}],\"version\":1}],\"ports\":[{\"value\":null,\"version\":0},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[{\"Path\":\"ConnectRequest\"},\"Elided\"],\"version\":1},{\"value\":[\"Elided\",{\"Int\":\"0\"}],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":3},{\"value\":[{\"Path\":\"ConnectResponse\"},\"Elided\"],\"version\":1},{\"value\":[\"Elided\",{\"Int\":\"1\"}],\"version\":1},{\"value\":[\"Elided\",\"Elided\"],\"version\":1},{\"value\":[{\"Path\":\"ChatMsg\"},\"Elided\"],\"version\":1},{\"value\":[\"Elided\",{\"Path\":\"input\"}],\"version\":1},{\"value\":[\"Elided\",{\"Path\":\"signal\"}],\"version\":1}],\"node_loops\":[{\"value\":null,\"version\":0}],\"loop_nodes\":[{\"value\":null,\"version\":0}],\"loop_parent\":[{\"value\":null,\"version\":0}],\"root_loops\":[],\"loop_children\":[{\"value\":null,\"version\":0}],\"node_subgraph\":[{\"value\":null,\"version\":0},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":2,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":1,\"version\":1},\"version\":1},{\"value\":{\"idx\":3,\"version\":1},\"version\":1}],\"subgraph_nodes\":[{\"value\":null,\"version\":0},{\"value\":[{\"idx\":8,\"version\":1},{\"idx\":9,\"version\":1},{\"idx\":13,\"version\":1},{\"idx\":14,\"version\":1},{\"idx\":1,\"version\":1},{\"idx\":2,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":10,\"version\":1},{\"idx\":11,\"version\":1}],\"version\":1},{\"value\":[{\"idx\":3,\"version\":1},{\"idx\":4,\"version\":1},{\"idx\":5,\"version\":1},{\"idx\":6,\"version\":1},{\"idx\":7,\"version\":1},{\"idx\":12,\"version\":1},{\"idx\":15,\"version\":1}],\"version\":1}],\"subgraph_stratum\":[{\"value\":null,\"version\":0},{\"value\":1,\"version\":1},{\"value\":0,\"version\":1},{\"value\":0,\"version\":1}],\"node_singleton_references\":[{\"value\":null,\"version\":0},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1},{\"value\":[],\"version\":1}],\"node_varnames\":[{\"value\":null,\"version\":0},{\"value\":\"outbound_chan\",\"version\":1},{\"value\":\"outbound_chan\",\"version\":1},{\"value\":\"inbound_chan\",\"version\":1},{\"value\":\"inbound_chan\",\"version\":1},{\"value\":\"inbound_chan\",\"version\":1},{\"value\":\"inbound_chan\",\"version\":1},{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":null,\"version\":0},{\"value\":\"lines\",\"version\":1},{\"value\":\"lines\",\"version\":1},{\"value\":null,\"version\":0},{\"value\":\"msg_send\",\"version\":1},{\"value\":\"msg_send\",\"version\":1}],\"subgraph_laziness\":[{\"value\":null,\"version\":0}]}");
                df.__assign_diagnostics("[]");
                let (hoff_16v1_send, hoff_16v1_recv) = df
                    .make_edge::<_, ::dfir_rs::scheduled::handoff::VecHandoff<_>>(
                        "handoff GraphNodeId(16v1)",
                    );
                let (hoff_17v1_send, hoff_17v1_recv) = df
                    .make_edge::<_, ::dfir_rs::scheduled::handoff::VecHandoff<_>>(
                        "handoff GraphNodeId(17v1)",
                    );
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_8v1__initialize__loc_nopath_1_0_1_0<T>(thunk: impl FnOnce() -> T) -> T {
                    thunk()
                }
                let mut sg_1v1_node_8v1_iter = {
                    #[inline(always)]
                    fn check_iter<IntoIter: ::std::iter::IntoIterator<Item = Item>, Item>(
                        into_iter: IntoIter,
                    ) -> impl ::std::iter::Iterator<Item = Item> {
                        ::std::iter::IntoIterator::into_iter(into_iter)
                    }
                    check_iter([()])
                };
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_9v1__map__loc_nopath_1_0_1_0<T>(thunk: impl FnOnce() -> T) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_13v1__defer_signal__loc_nopath_1_0_1_0<T>(thunk: impl FnOnce() -> T) -> T {
                    thunk()
                }
                let sg_1v1_node_13v1_internal_buffer =
                    df.add_state(::std::cell::RefCell::new(::std::vec::Vec::new()));
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_14v1__map__loc_nopath_1_0_1_0<T>(thunk: impl FnOnce() -> T) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_1v1__union__loc_nopath_1_0_1_0<T>(thunk: impl FnOnce() -> T) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_2v1__dest_sink_serde__loc_nopath_1_0_1_0<T>(thunk: impl FnOnce() -> T) -> T {
                    thunk()
                }
                let mut sg_1v1_node_2v1_sink = outbound;
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_10v1__source_stdin__loc_nopath_1_0_1_0<T>(thunk: impl FnOnce() -> T) -> T {
                    thunk()
                }
                #[expect(
                    clippy::let_and_return,
                    reason = "gives return value a self-documenting name"
                )]
                let mut sg_2v1_node_10v1_stream = {
                    use ::dfir_rs::tokio::io::AsyncBufReadExt;
                    let reader =
                        ::dfir_rs::tokio::io::BufReader::new(::dfir_rs::tokio::io::stdin());
                    let stdin_lines =
                        ::dfir_rs::tokio_stream::wrappers::LinesStream::new(reader.lines());
                    stdin_lines
                };
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_11v1__map__loc_nopath_1_0_1_0<T>(thunk: impl FnOnce() -> T) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_3v1__source_stream_serde__loc_nopath_1_0_1_0<T>(
                    thunk: impl FnOnce() -> T,
                ) -> T {
                    thunk()
                }
                let mut sg_3v1_node_3v1_stream = Box::pin(inbound);
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_4v1__map__loc_nopath_1_0_1_0<T>(thunk: impl FnOnce() -> T) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_5v1__map__loc_nopath_1_0_1_0<T>(thunk: impl FnOnce() -> T) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_15v1__for_each__loc_nopath_1_0_1_0<T>(thunk: impl FnOnce() -> T) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_12v1__persist__loc_nopath_1_0_1_0<T>(thunk: impl FnOnce() -> T) -> T {
                    thunk()
                }
                let singleton_op_12v1 =
                    df.add_state(::std::cell::RefCell::new(::std::vec::Vec::new()));
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_7v1__for_each__loc_nopath_1_0_1_0<T>(thunk: impl FnOnce() -> T) -> T {
                    thunk()
                }
                #[allow(non_snake_case)]
                #[inline(always)]
                fn op_6v1__demux_enum__loc_nopath_1_0_1_0<T>(thunk: impl FnOnce() -> T) -> T {
                    thunk()
                }
                let _ = |__val: Message| {
                    fn check_impl_demux_enum<
                        T: ?Sized + ::dfir_rs::util::demux_enum::DemuxEnumBase,
                    >(
                        _: &T,
                    ) {
                    }

                    check_impl_demux_enum(&__val);
                    match __val {
                        Message::ChatMsg { .. } => (),
                        Message::ConnectRequest { .. } => (),
                        Message::ConnectResponse { .. } => (),
                    };
                };
                let sgid_1v1 = df.add_subgraph_full("Subgraph GraphSubgraphId(1v1)",1,(hoff_16v1_recv,(hoff_17v1_recv,())),(),false,None,async move|context,(hoff_16v1_recv,(hoff_17v1_recv,())),()|{
                let mut hoff_16v1_recv = hoff_16v1_recv.borrow_mut_swap();
                let hoff_16v1_recv = hoff_16v1_recv.drain(..);
                let mut hoff_17v1_recv = hoff_17v1_recv.borrow_mut_swap();
                let hoff_17v1_recv = hoff_17v1_recv.drain(..);
                let op_8v1 = sg_1v1_node_8v1_iter.by_ref();
                let op_8v1 = {
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    pub fn op_8v1__initialize__loc_nopath_1_0_1_0<Item,Input: ::std::iter::Iterator<Item = Item>>(input:Input) -> impl ::std::iter::Iterator<Item = Item>{
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
                    op_8v1__initialize__loc_nopath_1_0_1_0(op_8v1)
                };
                #[allow(clippy::map_clone,reason = "dfir has no explicit `cloned`/`copied` operator")]
                let op_9v1 = op_8v1.map(|_m|(Message::ConnectRequest,server_addr));
                let op_9v1 = {
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    pub fn op_9v1__map__loc_nopath_1_0_1_0<Item,Input: ::std::iter::Iterator<Item = Item>>(input:Input) -> impl ::std::iter::Iterator<Item = Item>{
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
                    op_9v1__map__loc_nopath_1_0_1_0(op_9v1)
                };
                let mut sg_1v1_node_13v1_borrow_ident = unsafe {
                    context.state_ref_unchecked(sg_1v1_node_13v1_internal_buffer)
                }.borrow_mut();
                sg_1v1_node_13v1_borrow_ident.extend(hoff_16v1_recv);
                let op_13v1 = if hoff_17v1_recv.count()>0 {
                    ::std::option::Option::Some(sg_1v1_node_13v1_borrow_ident.drain(..))
                }else {
                    ::std::option::Option::None
                }.into_iter().flatten();
                let op_13v1 = {
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    pub fn op_13v1__defer_signal__loc_nopath_1_0_1_0<Item,Input: ::std::iter::Iterator<Item = Item>>(input:Input) -> impl ::std::iter::Iterator<Item = Item>{
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
                    op_13v1__defer_signal__loc_nopath_1_0_1_0(op_13v1)
                };
                #[allow(clippy::map_clone,reason = "dfir has no explicit `cloned`/`copied` operator")]
                let op_14v1 = op_13v1.map(|msg|(msg,server_addr));
                let op_14v1 = {
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    pub fn op_14v1__map__loc_nopath_1_0_1_0<Item,Input: ::std::iter::Iterator<Item = Item>>(input:Input) -> impl ::std::iter::Iterator<Item = Item>{
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
                    op_14v1__map__loc_nopath_1_0_1_0(op_14v1)
                };
                let op_1v1 = {
                    #[allow(unused)]
                    #[inline(always)]
                    fn check_inputs<A: ::std::iter::Iterator<Item = Item> ,B: ::std::iter::Iterator<Item = Item> ,Item>(a:A,b:B) -> impl ::std::iter::Iterator<Item = Item>{
                        a.chain(b)
                    }
                    check_inputs(op_9v1,op_14v1)
                };
                let op_1v1 = {
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    pub fn op_1v1__union__loc_nopath_1_0_1_0<Item,Input: ::std::iter::Iterator<Item = Item>>(input:Input) -> impl ::std::iter::Iterator<Item = Item>{
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
                    op_1v1__union__loc_nopath_1_0_1_0(op_1v1)
                };
                if true {
                    {
                        if!(::dfir_rs::tokio::runtime::Handle::try_current().is_ok()){
                            {
                                panic!();
                            };
                        }
                    };
                };
                let op_2v1 = {
                    fn sink_guard<Sink,Item>(sink:Sink) -> impl ::dfir_rs::futures::sink::Sink<Item,Error =  ::dfir_rs::Never>where Sink: ::dfir_rs::futures::sink::Sink<Item> ,Sink::Error: ::std::fmt::Debug,{
                        <Sink as ::dfir_rs::futures::sink::SinkExt<Item>> ::sink_map_err(sink, |e|{
                            panic!();
                        })
                    }
                    sink_guard(&mut sg_1v1_node_2v1_sink)
                };
                let op_2v1 =  ::dfir_rs::compiled::push::Map::new(|(payload,addr)|(::dfir_rs::util::serialize_to_bytes(payload),addr),op_2v1,);
                let op_2v1 = {
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    pub fn op_2v1__dest_sink_serde__loc_nopath_1_0_1_0<Item,Input>(input:Input) -> impl ::dfir_rs::futures::sink::Sink<Item,Error =  ::dfir_rs::Never>where Input: ::dfir_rs::futures::sink::Sink<Item,Error =  ::dfir_rs::Never>{
                        input
                    }
                    op_2v1__dest_sink_serde__loc_nopath_1_0_1_0(op_2v1)
                };
                #[inline(always)]
                async fn pivot_run_sg_1v1<Pull,Push,Item>(pull:Pull,push:Push)where Pull: ::std::iter::Iterator<Item = Item> ,Push: ::dfir_rs::futures::sink::Sink<Item,Error =  ::dfir_rs::Never> ,{
                    let mut push =  {
                        std::pin::pin!(push)
                    };
                    for item in pull {
                        ::dfir_rs::futures::sink::SinkExt::feed(&mut push,item).await.unwrap_or_else(|_|{
                            panic!();
                        });
                    }::dfir_rs::futures::sink::SinkExt::flush(&mut push).await.unwrap_or_else(|_|{
                        panic!();
                    });
                }
                (pivot_run_sg_1v1)(op_1v1,op_2v1).await;
            },);
                let sgid_2v1 = df.add_subgraph_full("Subgraph GraphSubgraphId(2v1)",0,(),(hoff_16v1_send,()),false,None,async move|context,(),(hoff_16v1_send,())|{
                let hoff_16v1_send =  ::dfir_rs::compiled::push::ForEach::new(|v|{
                    hoff_16v1_send.give(Some(v));
                });
                let op_10v1 = std::iter::from_fn(||{
                    match::dfir_rs::futures::stream::Stream::poll_next(std::pin::Pin::new(&mut sg_2v1_node_10v1_stream), &mut std::task::Context::from_waker(&context.waker())){
                        std::task::Poll::Ready(maybe) => maybe,
                        std::task::Poll::Pending => None,
                    
                        }
                });
                let op_10v1 = {
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    pub fn op_10v1__source_stdin__loc_nopath_1_0_1_0<Item,Input: ::std::iter::Iterator<Item = Item>>(input:Input) -> impl ::std::iter::Iterator<Item = Item>{
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
                    op_10v1__source_stdin__loc_nopath_1_0_1_0(op_10v1)
                };
                #[allow(clippy::map_clone,reason = "dfir has no explicit `cloned`/`copied` operator")]
                let op_11v1 = op_10v1.map(|l|Message::ChatMsg {
                    nickname:opts.name.clone(),message:l.unwrap(),ts:Utc::now()
                });
                let op_11v1 = {
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    pub fn op_11v1__map__loc_nopath_1_0_1_0<Item,Input: ::std::iter::Iterator<Item = Item>>(input:Input) -> impl ::std::iter::Iterator<Item = Item>{
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
                    op_11v1__map__loc_nopath_1_0_1_0(op_11v1)
                };
                #[inline(always)]
                async fn pivot_run_sg_2v1<Pull,Push,Item>(pull:Pull,push:Push)where Pull: ::std::iter::Iterator<Item = Item> ,Push: ::dfir_rs::futures::sink::Sink<Item,Error =  ::dfir_rs::Never> ,{
                    let mut push =  {
                        std::pin::pin!(push)
                    };
                    for item in pull {
                        ::dfir_rs::futures::sink::SinkExt::feed(&mut push,item).await.unwrap_or_else(|_|{
                            panic!();
                        });
                    }::dfir_rs::futures::sink::SinkExt::flush(&mut push).await.unwrap_or_else(|_|{
                        panic!();
                    });
                }
                (pivot_run_sg_2v1)(op_11v1,hoff_16v1_send).await;
            },);
                let sgid_3v1 = df.add_subgraph_full("Subgraph GraphSubgraphId(3v1)",0,(),(hoff_17v1_send,()),false,None,async move|context,(),(hoff_17v1_send,())|{
                let hoff_17v1_send =  ::dfir_rs::compiled::push::ForEach::new(|v|{
                    hoff_17v1_send.give(Some(v));
                });
                let op_3v1 =  ::std::iter::from_fn(||{
                    match::dfir_rs::futures::stream::Stream::poll_next(sg_3v1_node_3v1_stream.as_mut(), &mut ::std::task::Context::from_waker(&context.waker())){
                        ::std::task::Poll::Ready(Some(::std::result::Result::Ok((payload,addr)))) => Some(::dfir_rs::util::deserialize_from_bytes:: <_>(payload).map(|payload|(payload,addr))), 
                        ::std::task::Poll::Ready(Some(Err(_))) => None, 
                        ::std::task::Poll::Ready(None) => None, 
                        ::std::task::Poll::Pending => None,
                    
                        }
                });
                let op_3v1 = {
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    pub fn op_3v1__source_stream_serde__loc_nopath_1_0_1_0<Item,Input: ::std::iter::Iterator<Item = Item>>(input:Input) -> impl ::std::iter::Iterator<Item = Item>{
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
                    op_3v1__source_stream_serde__loc_nopath_1_0_1_0(op_3v1)
                };
                #[allow(clippy::map_clone,reason = "dfir has no explicit `cloned`/`copied` operator")]
                let op_4v1 = op_3v1.map(Result::unwrap);
                let op_4v1 = {
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    pub fn op_4v1__map__loc_nopath_1_0_1_0<Item,Input: ::std::iter::Iterator<Item = Item>>(input:Input) -> impl ::std::iter::Iterator<Item = Item>{
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
                    op_4v1__map__loc_nopath_1_0_1_0(op_4v1)
                };
                #[allow(clippy::map_clone,reason = "dfir has no explicit `cloned`/`copied` operator")]
                let op_5v1 = op_4v1.map(|(msg,_addr)|msg);
                let op_5v1 = {
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    pub fn op_5v1__map__loc_nopath_1_0_1_0<Item,Input: ::std::iter::Iterator<Item = Item>>(input:Input) -> impl ::std::iter::Iterator<Item = Item>{
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
                    op_5v1__map__loc_nopath_1_0_1_0(op_5v1)
                };
                let op_15v1 =  ::dfir_rs::compiled::push::ForEach::new(|(nick,msg,ts)|pretty_print_msg(nick,msg,ts));
                let op_15v1 = {
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    pub fn op_15v1__for_each__loc_nopath_1_0_1_0<Item,Input>(input:Input) -> impl ::dfir_rs::futures::sink::Sink<Item,Error =  ::dfir_rs::Never>where Input: ::dfir_rs::futures::sink::Sink<Item,Error =  ::dfir_rs::Never>{
                        input
                    }
                    op_15v1__for_each__loc_nopath_1_0_1_0(op_15v1)
                };
                let mut sg_3v1_node_12v1_persistvec = unsafe {
                    context.state_ref_unchecked(singleton_op_12v1)
                }.borrow_mut();
                let op_12v1 = {
                    #[allow(clippy::ptr_arg)]
                    fn constrain_types<'ctx,Push,Item>(vec: &'ctx mut Vec<Item> ,output:Push,is_new_tick:bool) -> impl 'ctx+ ::dfir_rs::futures::sink::Sink<Item,Error =  ::dfir_rs::Never>where Push:'ctx+ ::dfir_rs::futures::sink::Sink<Item,Error =  ::dfir_rs::Never> ,Item: ::std::clone::Clone,{
                        let replay = if is_new_tick {
                            vec.iter()
                        }else {
                            [].iter()
                        };
                        ::dfir_rs::compiled::push::Persist::new(replay,output)
                    }
                    constrain_types(&mut *sg_3v1_node_12v1_persistvec,hoff_17v1_send,context.is_first_run_this_tick())
                };
                let op_12v1 = {
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    pub fn op_12v1__persist__loc_nopath_1_0_1_0<Item,Input>(input:Input) -> impl ::dfir_rs::futures::sink::Sink<Item,Error =  ::dfir_rs::Never>where Input: ::dfir_rs::futures::sink::Sink<Item,Error =  ::dfir_rs::Never>{
                        input
                    }
                    op_12v1__persist__loc_nopath_1_0_1_0(op_12v1)
                };
                let op_7v1 =  ::dfir_rs::compiled::push::ForEach::new(|()|{
                    println!("Received unexpected connect request from server.");
                });
                let op_7v1 = {
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    pub fn op_7v1__for_each__loc_nopath_1_0_1_0<Item,Input>(input:Input) -> impl ::dfir_rs::futures::sink::Sink<Item,Error =  ::dfir_rs::Never>where Input: ::dfir_rs::futures::sink::Sink<Item,Error =  ::dfir_rs::Never>{
                        input
                    }
                    op_7v1__for_each__loc_nopath_1_0_1_0(op_7v1)
                };
                let sg_3v1_node_6v1_outputs = ({
                    std::pin::pin!(op_15v1)
                }, {
                    std::pin::pin!(op_7v1)
                }, {
                    std::pin::pin!(op_12v1)
                },);
                let op_6v1 =  ::dfir_rs::compiled::push::DemuxEnum::new(sg_3v1_node_6v1_outputs);
                let op_6v1 = {
                    #[allow(non_snake_case)]
                    #[inline(always)]
                    pub fn op_6v1__demux_enum__loc_nopath_1_0_1_0<Item,Input>(input:Input) -> impl ::dfir_rs::futures::sink::Sink<Item,Error =  ::dfir_rs::Never>where Input: ::dfir_rs::futures::sink::Sink<Item,Error =  ::dfir_rs::Never>{
                        input
                    }
                    op_6v1__demux_enum__loc_nopath_1_0_1_0(op_6v1)
                };
                #[inline(always)]
                async fn pivot_run_sg_3v1<Pull,Push,Item>(pull:Pull,push:Push)where Pull: ::std::iter::Iterator<Item = Item> ,Push: ::dfir_rs::futures::sink::Sink<Item,Error =  ::dfir_rs::Never> ,{
                    let mut push =  {
                        std::pin::pin!(push)
                    };
                    for item in pull {
                        ::dfir_rs::futures::sink::SinkExt::feed(&mut push,item).await.unwrap_or_else(|_|{
                            panic!();
                        });
                    }::dfir_rs::futures::sink::SinkExt::flush(&mut push).await.unwrap_or_else(|_|{
                        panic!();
                    });
                }
                (pivot_run_sg_3v1)(op_5v1,op_6v1).await;
                context.schedule_subgraph(context.current_subgraph(),false);
            },);
                df
            }
        }
    };

    // optionally print the dataflow graph
    #[cfg(feature = "debugging")]
    if let Some(graph) = opts.graph {
        let serde_graph = hf
            .meta_graph()
            .expect("No graph found, maybe failed to parse.");
        serde_graph.open_graph(graph, opts.write_config).unwrap();
    }

    hf.run().await.unwrap();
}
