#![expect(missing_docs, reason = "testing")]

use std::fmt::Debug;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::sink::Sink;
use pin_project_lite::pin_project;

pin_project! {
    #[must_use = "sinks do nothing unless polled"]
    pub struct Debugging<Si> {
        #[pin]
        sink: Si,
        tag: String,
    }
}

impl<Si> Debugging<Si> {
    pub fn new<Item>(tag: String, sink: Si) -> Self
    where
        Self: Sink<Item>,
    {
        Self { sink, tag }
    }
}

impl<Si, Item> Sink<Item> for Debugging<Si>
where
    Si: Sink<Item>,
    Item: Debug,
{
    type Error = Si::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        let out = this.sink.poll_ready(cx);
        println!("{} POLL_READY, ready: {}", this.tag, out.is_ready());
        out
    }
    fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        let this = self.project();
        println!("{} START_SEND START {:?}", this.tag, item);
        let out = this.sink.start_send(item);
        println!("{} START_SEND END", this.tag);
        out
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        let out = this.sink.poll_flush(cx);
        println!("{} POLL_FLUSH, ready: {}", this.tag, out.is_ready());
        out
    }
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        let out = this.sink.poll_close(cx);
        println!("{} POLL_CLOSE, ready: {}", this.tag, out.is_ready());
        out
    }
}
