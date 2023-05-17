#![allow(dead_code)]

use std::cell::RefCell;
use std::rc::Rc;

use hydroflow::scheduled::graph::Hydroflow;
use hydroflow::scheduled::graph_ext::GraphExt;
use hydroflow::scheduled::handoff::{Iter, VecHandoff};
use hydroflow::scheduled::port::RecvPort;

use crate::{Datum, RelExpr};

pub(crate) fn run_dataflow(r: RelExpr) -> Vec<Vec<Datum>> {
    let mut df = Hydroflow::new();

    let output_port = render_relational(&mut df, r);

    let output = Rc::new(RefCell::new(Vec::new()));
    let inner = output.clone();

    df.add_subgraph_sink("output sink", output_port, move |_ctx, recv| {
        for v in recv.take_inner() {
            (*inner).borrow_mut().push(v);
        }
    });

    df.run_available();

    let v = (*output).borrow();
    v.clone()
}

fn render_relational(df: &mut Hydroflow, r: RelExpr) -> RecvPort<VecHandoff<Vec<Datum>>> {
    let (send_port, recv_port) = df.make_edge("handoff");
    match r {
        RelExpr::Values(mut v) => {
            // TODO: drip-feed data?
            let scope = Vec::new();
            df.add_subgraph_source("value source", send_port, move |_ctx, send| {
                send.give(Iter(
                    v.drain(..)
                        .map(|row| row.into_iter().map(|e| e.eval(&scope)).collect()),
                ));
            });
        }
        RelExpr::Filter(preds, v) => {
            let input = render_relational(df, *v);
            df.add_subgraph_in_out("filter", input, send_port, move |_ctx, recv, send| {
                send.give(Iter(recv.take_inner().into_iter().filter(|row| {
                    preds.iter().all(|p| p.eval(row) == Datum::Bool(true))
                })));
            });
        }
        RelExpr::Project(exprs, v) => {
            let input = render_relational(df, *v);
            df.add_subgraph_in_out("project", input, send_port, move |_ctx, recv, send| {
                send.give(Iter(
                    recv.take_inner()
                        .into_iter()
                        .map(|row| exprs.iter().map(|e| e.eval(&row)).collect()),
                ));
            });
        }
    }
    recv_port
}
