use dfir_rs::dfir_syntax;
use dfir_rs::scheduled::graph::Dfir;
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::{Timestamp, Token};

pub(crate) fn rga_minimal(
    input_recv: futures::stream::Fuse<UnboundedReceiverStream<(Token, Timestamp)>>,
    rga_send: UnboundedSender<(Token, Timestamp)>,
    _list_send: UnboundedSender<(Timestamp, Timestamp)>,
) -> Dfir<'static> {
    dfir_syntax! {
        insertAfter = source_stream(input_recv);

        insertAfter -> for_each(|(c, p): (Token, Timestamp)| rga_send.send((c, p)).unwrap());
    }
}
