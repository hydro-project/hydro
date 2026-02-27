use dfir_rs::dfir_syntax;
use dfir_rs::scheduled::graph::Dfir;
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::protocol::{Timestamp, Token};

pub(crate) fn rga_datalog(
    input_recv: futures::stream::Fuse<UnboundedReceiverStream<(Token, Timestamp)>>,
    rga_send: UnboundedSender<(Token, Timestamp)>,
    list_send: UnboundedSender<(Timestamp, Timestamp)>,
) -> Dfir<'static> {
    dfir_syntax! {
        edges = source_stream(input_recv) -> tee();
        insertAfter = edges -> map(|(c, p): (Token, Timestamp) | (c.ts, p)) -> tee();

        // isListElem(Oid) :- insertAfter(Oid, Parent)
        isListElem = insertAfter -> map(|(oid, _parent)| oid);

        // hasChild(Parent) :- insertAfter(Child, Parent)
        hasChild = insertAfter -> map(|(_child, parent)| parent);

        // laterChild(Parent, Child2) :- insertAfter( Child1, Parent), insertAfter( Child2, Parent), Child1 > Child2
        laterChild = join() -> filter_map(|(parent, (child1, child2))| if child1 > child2 {Some((parent, child2))} else {None} );
        insertAfter -> map(|(child, parent)| (parent, child)) -> [0]laterChild;
        insertAfter -> map(|(child, parent)| (parent, child)) -> [1]laterChild;

        // firstChild(Parent, Child) :- insertAfter(Child, Parent), !laterChild (Parent, Child)
        firstChild = difference();
        insertAfter -> map(|(child, parent)| (parent, child)) -> [pos]firstChild;
        laterChild -> [neg]firstChild;

        // sibling (Child1, Child2) :- insertAfter(Child1,Parent), insertAfter(Child2, Parent)
        sibling = join() -> map(|(_p, (c1, c2))| (c1, c2)) -> tee();
        insertAfter -> map(|(c, p)| (p, c)) -> [0]sibling;
        insertAfter -> map(|(c, p)| (p, c)) -> [1]sibling;

        // laterSibling (Sib1, Sib2) :- sibling (Sibl,Sib2), Sibl > Sib2
        laterSibling = sibling -> filter(|(s1, s2)| s1 > s2) -> tee();

        // laterSibling2(SibI, Sib3) :- sibling(Sibl,Sib2), sibling (Sibl,Sib3), Sibl> Sib2, Sib2 > Sib3
        laterSibling2 = join() -> filter_map(|(s1, (s2, s3))| if s1 > s2 && s2 > s3 {Some((s1, s3))} else {None});
        sibling -> [0]laterSibling2;
        sibling -> [1]laterSibling2;

        // nextSibling (Sib1,Sib2) :- laterSibling (Sibl, Sib2), - laterSibling2(Sibl, Sib2)
        nextSibling = difference();
        laterSibling -> [pos]nextSibling;
        laterSibling2 -> [neg]nextSibling;

        // hasNextSibling(Sib1) :- laterSibling (Sibl, Sib2)
        hasNextSibling = laterSibling -> map(|(s1, _s2)| s1);

        // nextSiblingAnc(Start, Next) :- nextSibling (Start, Next)
        nextSiblingAnc = union() -> tee();
        nextSibling -> nextSiblingAnc;

        // nextSiblingAnc (Start, Next) :- !hasNextSibling (Start), insertAfter(Start, Parent), nextSiblingAnc( Parent, Next)
        nsa_diff = difference();
        hasNextSibling -> [neg]nsa_diff;
        insertAfter -> map(|(start, _parent)| start) -> [pos]nsa_diff;
        nsa_join1 = join() -> map(|(start, (_, parent))| (parent, start));
        nsa_diff -> map(|start| (start, ())) -> [0]nsa_join1;
        insertAfter -> [1]nsa_join1;
        nsa_join2 = join() -> map(|(_parent, (start, next))| (start, next)) -> nextSiblingAnc;
        nsa_join1 -> [0]nsa_join2;
        nsa = nextSiblingAnc -> [1]nsa_join2;

        // hasSiblingAnc (Start) :- nextSiblingAnc( Start, Next)
        hasSiblingAnc = nextSiblingAnc -> map(|(start, _next)| start) -> null();

        // nextElem(Prev,Next) :- firstChild(Prev, Next)
        nextElem = union();
        firstChild -> nextElem;

        // nextElem(Prev,Next) :- isListElem(Prev), - hasChild (Prev), nextSiblingAnc(Prev,Next)
        ne_diff = difference();
        isListElem -> [pos]ne_diff;
        hasChild -> [neg]ne_diff;
        ne_join = join() -> map(|(prev, (_, next))| (prev, next));
        ne_diff -> map(|prev| (prev, ())) -> [0]ne_join;
        nextSiblingAnc -> [1]ne_join;
        ne_join -> nextElem;

        edges -> for_each(|(c, p): (Token, Timestamp)| rga_send.send((c, p)).unwrap());
        nextElem -> for_each(|(first, second)| list_send.send((first, second)).unwrap());
    }
}
