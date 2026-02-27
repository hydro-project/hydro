use dfir_rs::dfir_syntax;
use dfir_rs::scheduled::graph::Dfir;
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::protocol::{Timestamp, Token};

pub(crate) fn rga_datalog_agg(
    input_recv: futures::stream::Fuse<UnboundedReceiverStream<(Token, Timestamp)>>,
    rga_send: UnboundedSender<(Token, Timestamp)>,
    list_send: UnboundedSender<(Timestamp, Timestamp)>,
) -> Dfir<'static> {
    dfir_syntax! {
        edges = source_stream(input_recv) -> tee();
        insertAfter =  edges -> map(|(c, p): (Token, Timestamp)| (c.ts, p)) -> tee();

        // isListElem(Oid) :- insertAfter(Oid, Parent)
        isListElem = insertAfter[isListElem] -> map(|(oid, _parent)| oid);

        // parents(Parent) :- insertAfter(Child, Parent)
        parents = insertAfter[parents] -> map(|(_child, parent)| parent);

        // firstLastChild(Parent, max<Child>, min<Child) :- insertAfter(Child, Parent)
        firstLastChild = insertAfter[firstLastChild]
            -> map(|(c, p)| (p, c))
            -> fold_keyed::<'static, Timestamp, (Timestamp, Timestamp)>(
                || (Timestamp{node_ts: 0, node_id: 0}, Timestamp{node_ts: usize::MAX, node_id: usize::MAX}),
                |(first, last): &mut (Timestamp, Timestamp), s2: Timestamp| {
                    if s2 > *first { *first = s2 };
                    if s2 < *last { *last = s2 };
                }
            )
            -> tee();

        // firstChild(Parent, First) :- firstLastChild(Parent, First, Last)
        firstChild = firstLastChild[first] -> map(|(p, (first, _last))| (p, first));

        // lastChild(Parent, Last) :- firstLastChild(Parent, First, Last)
        lastChild = firstLastChild[last] -> map(|(p, (_first, last))| (p, last));

        // sibling (Child1, Child2) :- insertAfter(Child1,Parent), insertAfter(Child2, Parent), Child1 > Child2
        sibling = join() -> map(|(_p, (c1, c2)): (Timestamp, (Timestamp, Timestamp))| (c1, c2)) -> filter(|(s1, s2)| s1 > s2);
        insertAfter[sib1] -> map(|(c, p)| (p, c)) -> [0]sibling;
        insertAfter[sib2] -> map(|(c, p)| (p, c)) -> [1]sibling;

        // nextSibling (Sib1, max<Sib2>) :- sibling (Sib1, Sib2)
        nextSibling = sibling
            -> fold_keyed::<'static, Timestamp, Timestamp>(
                || Timestamp{node_ts: 0, node_id: 0},
                |accum: &mut Timestamp, s2: Timestamp| {
                    eprintln!("accum {:?}, s2 {:?}", accum, s2);
                    if s2 > *accum {*accum = s2}
                }
            );

        // nextSiblingAnc (Start, Next) :- nextSibling (Start, Next), Next != 0
        nextSiblingAnc = union() -> tee();
        nextSibling -> filter(|(_s1, s2)| *s2 != Timestamp{node_ts: 0, node_id: 0}) -> nextSiblingAnc;

        // nextSiblingAnc(Node, Next) :- lastChild(Parent, Node), nextSiblingAnc(Parent, Next)
        upEdge = join() -> map(|(_parent, (node, next))| (node, next)) -> nextSiblingAnc;
        lastChild      -> [0]upEdge;
        nextSiblingAnc -> [1]upEdge;

        // nextElem(Prev, Next) :- firstChild(Prev, Next)
        nextElem = union();
        firstChild[firstChild] -> nextElem;

        // nextElem(Prev,Next) :- isListElem(Prev), !parents (Prev), nextSiblingAnc(Prev,Next)
        childless = difference();
        isListElem[all_nodes] -> [pos]childless;
        parents[parents] -> [neg]childless;
        ne_join = join() -> map(|(prev, (_, next))| (prev, next));
        childless[childless] -> map(|prev| (prev, ())) -> [0]ne_join;
        nextSiblingAnc[nextSiblingAnc] -> [1]ne_join;
        ne_join -> nextElem;

        edges -> for_each(|(c, p): (Token, Timestamp)| rga_send.send((c, p)).unwrap());
        nextElem -> for_each(|(first, second)| list_send.send((first, second)).unwrap());
    }
}
