use dfir_rs::dfir_syntax;
use futures::stream::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

// A simple stream that is Unpin but NOT FusedStream
// It will panic if polled after returning None
#[derive(Default)]
struct NotFusedStream {
    done: bool,
}

impl Stream for NotFusedStream {
    type Item = i32;
    
    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.done {
            panic!("NotFusedStream polled after returning None!");
        }
        self.done = true;
        Poll::Ready(None)
    }
}

fn main() {
    let stream = NotFusedStream::default();
    
    let mut df = dfir_syntax! {
        source_stream(stream) -> for_each(std::mem::drop);
    };
    df.run_available_sync();
}
