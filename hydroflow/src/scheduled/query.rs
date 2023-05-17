use std::borrow::Cow;
use std::cell::RefCell;
use std::rc::Rc;

use crate::scheduled::graph::Hydroflow;
use crate::scheduled::handoff::VecHandoff;

use super::context::Context;
use super::graph_ext::GraphExt;
use super::handoff::Iter;
use super::port::{RecvPort, SendCtx};

const QUERY_EDGE_NAME: Cow<'static, str> = Cow::Borrowed("query handoff");

#[derive(Default)]
pub struct Query {
    df: Rc<RefCell<Hydroflow>>,
}

impl Query {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn source<F, T>(&mut self, f: F) -> Operator<T>
    where
        T: 'static,
        F: 'static + FnMut(&Context, &SendCtx<VecHandoff<T>>),
    {
        let mut df = self.df.borrow_mut();

        let (send_port, recv_port) = df.make_edge(QUERY_EDGE_NAME);
        df.add_subgraph_source("source", send_port, f);

        Operator {
            df: self.df.clone(),
            recv_port,
        }
    }

    pub fn concat<T>(&mut self, ops: Vec<Operator<T>>) -> Operator<T>
    where
        T: 'static,
    {
        let mut df = self.df.borrow_mut();

        let (send_port, recv_port) = df.make_edge(QUERY_EDGE_NAME);
        df.add_subgraph_n_m(
            "concat",
            ops.into_iter().map(|op| op.recv_port).collect(),
            vec![send_port],
            |_ctx, ins, out| {
                for &input in ins {
                    out[0].give(input.take_inner());
                }
            },
        );

        Operator {
            df: self.df.clone(),
            recv_port,
        }
    }

    pub fn run_available(&mut self) {
        (*self.df).borrow_mut().run_available();
    }
}

pub struct Operator<T>
where
    T: 'static,
{
    df: Rc<RefCell<Hydroflow>>,
    recv_port: RecvPort<VecHandoff<T>>,
}

impl<T> Operator<T>
where
    T: 'static,
{
    pub fn map<U, F>(self, mut f: F) -> Operator<U>
    where
        F: 'static + Fn(T) -> U,
        U: 'static,
    {
        let mut df = self.df.borrow_mut();

        let (send_port, recv_port) = df.make_edge(QUERY_EDGE_NAME);
        df.add_subgraph_in_out("map", self.recv_port, send_port, move |_ctx, recv, send| {
            send.give(Iter(recv.take_inner().into_iter().map(&mut f)));
        });

        std::mem::drop(df);
        Operator {
            df: self.df,
            recv_port,
        }
    }

    #[must_use]
    pub fn filter<F>(self, mut f: F) -> Operator<T>
    where
        F: 'static + Fn(&T) -> bool,
    {
        let mut df = self.df.borrow_mut();

        let (send_port, recv_port) = df.make_edge(QUERY_EDGE_NAME);
        df.add_subgraph_in_out(
            "filter",
            self.recv_port,
            send_port,
            move |_ctx, recv, send| {
                send.give(Iter(recv.take_inner().into_iter().filter(&mut f)));
            },
        );

        std::mem::drop(df);
        Operator {
            df: self.df,
            recv_port,
        }
    }

    #[must_use]
    pub fn concat(self, other: Operator<T>) -> Operator<T> {
        // TODO(justin): this is very slow.

        let mut df = self.df.borrow_mut();

        let (send_port, recv_port) = df.make_edge(QUERY_EDGE_NAME);
        df.add_subgraph_2in_out(
            "concat",
            self.recv_port,
            other.recv_port,
            send_port,
            |_ctx, recv1, recv2, send| {
                send.give(recv1.take_inner());
                send.give(recv2.take_inner());
            },
        );

        std::mem::drop(df);
        Operator {
            df: self.df,
            recv_port,
        }
    }

    pub fn sink<F>(self, f: F)
    where
        F: 'static + Fn(T),
    {
        self.df
            .borrow_mut()
            .add_subgraph_sink("sink", self.recv_port, move |_ctx, recv| {
                for v in recv.take_inner() {
                    f(v)
                }
            });
    }
}

impl<T: Clone> Operator<T> {
    pub fn tee(self, n: usize) -> Vec<Operator<T>>
    where
        T: 'static,
    {
        // TODO(justin): this is very slow. TODO(mingwei) use teeing handoff once its added.

        let mut df = self.df.borrow_mut();

        let mut sends = Vec::with_capacity(n);
        let mut recvs = Vec::with_capacity(n);
        for _ in 0..n {
            let (send_port, recv_port) = df.make_edge(QUERY_EDGE_NAME);
            sends.push(send_port);
            recvs.push(Operator {
                df: self.df.clone(),
                recv_port,
            });
        }

        df.add_subgraph_n_m(
            "tee",
            vec![self.recv_port],
            sends,
            move |_ctx, recvs, sends| {
                let input = recvs.iter().next().unwrap().take_inner();
                if let Some((&last_output, outputs)) = sends.split_last() {
                    for output in outputs {
                        output.give(Iter(input.iter().cloned()));
                    }
                    last_output.give(input);
                }
            },
        );

        recvs
    }
}
