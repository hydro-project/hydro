use std::collections::VecDeque;

use dfir_rs::dfir_syntax;
use dfir_rs::scheduled::graph::Dfir;
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::protocol::{Timestamp, Token};

pub(crate) fn rga_adjacency(
    input_recv: futures::stream::Fuse<UnboundedReceiverStream<(Token, Timestamp)>>,
    rga_send: UnboundedSender<(Token, Timestamp)>,
    list_send: UnboundedSender<(Timestamp, Timestamp)>,
) -> Dfir<'static> {
    dfir_syntax! {
        insertAfter = source_stream(input_recv) -> tee();

        // adjacency(parent:Timestamp, kids:VecDeque<Timestamp>, sibs:Vec<(Timestamp,Timestamp>)) tuples
        adjacency_sibs = insertAfter[adjacency]
          -> map(|(child, parent): (Token, Timestamp)| (parent, child.ts))
          -> fold_keyed::<'tick, Timestamp, (VecDeque<Timestamp>, Vec<(Timestamp, Timestamp)>)>(
                    || (VecDeque::<Timestamp>::new(), Vec::<(Timestamp, Timestamp)>::new()),
                    |(kids, sibs): &mut (VecDeque<Timestamp>, Vec::<(Timestamp, Timestamp)>), val: Timestamp| {
                        // siblings (older, younger)
                        if !kids.is_empty() {
                            sibs.push((val, *kids.back().unwrap()));
                        }
                        kids.push_back(val);
            })
          -> map(|(parent, (kids, sibs))| ((parent, kids), sibs)) -> unzip();
        adjacency = adjacency_sibs[0]  -> tee();
        siblings = adjacency_sibs[1] -> flatten();
        leaves = difference();
        split = insertAfter[diff] -> unzip();
        split[0] -> map(|child| child.ts) -> [pos]leaves;
        split[1] -> [neg]leaves;

        // nextSiblingAnc
        nextSiblingAnc = union() -> tee();
        // siblings(bigger, smaller)
        siblings -> nextSiblingAnc;
        // lastChild(parent, lastChild)
        lastChild = adjacency[lastChild] -> map(|(parent, kids): (Timestamp, VecDeque<Timestamp>)| (parent, *kids.front().unwrap()));
        upEdge = join() -> map(|(_parent, (last_child, next_sib)): (Timestamp, (Timestamp, Timestamp))| (last_child, next_sib));
        lastChild -> [0]upEdge;
        nextSiblingAnc -> [1]upEdge;
        upEdge -> nextSiblingAnc;

        // nextElem
        nextElem = union();
        adjacency[nextElem] -> map(|(parent, kids): (Timestamp, VecDeque<Timestamp>)| (parent, *kids.back().unwrap())) -> nextElem;
        // nextElem(Prev,Next) :- isListElem(Prev), !hasChild (Prev), nextSiblingAnc(Prev,Next)
        ne_join = join() -> map(|(prev, (_, next))| (prev, next));
        leaves -> map(|prev| (prev, ())) -> [0]ne_join;
        nextSiblingAnc -> [1]ne_join;
        ne_join -> nextElem;

        insertAfter[print] -> for_each(|(c, p): (Token, Timestamp)| rga_send.send((c, p)).unwrap());
        nextElem -> for_each(|(first, second)| list_send.send((first, second)).unwrap());
    }
}
