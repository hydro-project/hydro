#![allow(
    unused,
    reason = "unused in trybuild but the __staged version is needed"
)]
#![allow(missing_docs, reason = "used internally")]

use std::{
    net::SocketAddr,
    pin::Pin,
    task::{Context, Poll},
};

use futures::{FutureExt, Sink, SinkExt, Stream, StreamExt};
use sha2::digest::block_buffer::Lazy;
use stageleft::{QuotedWithContext, q};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

enum LazyTcpSinkState {
    None,
    Connecting(Pin<Box<dyn Future<Output = std::io::Result<TcpStream>>>>),
    Connected(FramedWrite<TcpStream, LengthDelimitedCodec>),
}

/// A lazy tcp sink will attempt to connect on the target when the first item is sent into it
pub struct LazyTcpSink {
    target: String,
    state: LazyTcpSinkState,
}

impl LazyTcpSink {
    /// Creates a new LazyTcpSink.
    /// target should be in {ip address|domain name}:{port}.
    ///
    pub fn new(target: impl AsRef<str>) -> Self {
        Self {
            target: target.as_ref().to_string(),
            state: LazyTcpSinkState::None,
        }
    }
}

impl Sink<bytes::Bytes> for LazyTcpSink {
    type Error = std::io::Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if matches!(self.state, LazyTcpSinkState::None) {
            self.state =
                LazyTcpSinkState::Connecting(TcpStream::connect(self.target.clone()).boxed());
        }

        if let LazyTcpSinkState::Connecting(ref mut v) = self.state {
            match v.poll_unpin(cx) {
                Poll::Ready(Ok(stream)) => {
                    self.state = LazyTcpSinkState::Connected(FramedWrite::new(
                        stream,
                        LengthDelimitedCodec::new(),
                    ));
                }
                Poll::Ready(Err(err)) => return Poll::Ready(Err(err)),
                Poll::Pending => return Poll::Pending,
            }
        }

        let LazyTcpSinkState::Connected(ref mut sink) = self.state else {
            unreachable!()
        };

        sink.poll_ready_unpin(cx)
    }

    fn start_send(mut self: Pin<&mut Self>, item: bytes::Bytes) -> Result<(), Self::Error> {
        match self.state {
            LazyTcpSinkState::None => unreachable!(),
            LazyTcpSinkState::Connecting(_) => unreachable!(),
            LazyTcpSinkState::Connected(ref mut sink) => sink.start_send_unpin(item),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self.state {
            LazyTcpSinkState::None => unreachable!(),
            LazyTcpSinkState::Connecting(_) => unreachable!(),
            LazyTcpSinkState::Connected(ref mut sink) => sink.poll_flush_unpin(cx),
        }
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self.state {
            LazyTcpSinkState::None => unreachable!(),
            LazyTcpSinkState::Connecting(_) => unreachable!(),
            LazyTcpSinkState::Connected(ref mut sink) => sink.poll_close_unpin(cx),
        }
    }
}

enum LazyTcpSourceState {
    None,
    Binding(Pin<Box<dyn Future<Output = std::io::Result<TcpListener>> + Send>>),
    Listening(TcpListener),
    Connected(FramedRead<TcpStream, LengthDelimitedCodec>),
}

pub struct LazyTcpSource {
    bind_addr: SocketAddr,
    state: LazyTcpSourceState,
}

impl LazyTcpSource {
    pub fn new(bind_addr: SocketAddr) -> Self {
        Self {
            bind_addr,
            state: LazyTcpSourceState::None,
        }
    }
}

impl Stream for LazyTcpSource {
    type Item = Result<bytes::BytesMut, std::io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if matches!(self.state, LazyTcpSourceState::None) {
            self.state =
                LazyTcpSourceState::Binding(TcpListener::bind(self.bind_addr.clone()).boxed());
        }

        if let LazyTcpSourceState::Binding(ref mut listener_fut) = self.state {
            match listener_fut.poll_unpin(cx) {
                Poll::Ready(Ok(listener)) => {
                    self.state = LazyTcpSourceState::Listening(listener);
                }
                Poll::Ready(Err(err)) => return Poll::Ready(Some(Err(err))),
                Poll::Pending => return Poll::Pending,
            }
        }

        if let LazyTcpSourceState::Listening(ref mut listener) = self.state {
            match listener.poll_accept(cx) {
                Poll::Ready(Ok((stream, peer_addr))) => {
                    self.state = LazyTcpSourceState::Connected(FramedRead::new(
                        stream,
                        LengthDelimitedCodec::new(),
                    ));
                }
                Poll::Ready(Err(err)) => return Poll::Ready(Some(Err(err))),
                Poll::Pending => return Poll::Pending,
            }
        }

        let LazyTcpSourceState::Connected(ref mut sink) = self.state else {
            unreachable!()
        };

        sink.poll_next_unpin(cx)
    }
}

#[cfg(test)]
mod test {

    use futures::{Sink, StreamExt};

    use crate::deploy::deploy_runtime_containerized::{
        LazyTcpSink, LazyTcpSinkState, LazyTcpSource, LazyTcpSourceState,
    };

    #[tokio::test]
    async fn test_lazy_sink() -> anyhow::Result<()> {
        let mut lazy_source = LazyTcpSource::new("0.0.0.0:16000".parse().unwrap());
        let mut lazy_sink = LazyTcpSink::new("localhost:16000");

        let data = bincode::serialize("hello").unwrap();

        let f = tokio::task::spawn(async move { lazy_source.next().await });

        tokio::time::sleep(std::time::Duration::from_millis(100)).await; // TODO: need to add some way to wait for the socket to be bound so the sleep is unnecessary.

        futures::SinkExt::send(&mut lazy_sink, bytes::Bytes::copy_from_slice(&data)).await?;

        futures::SinkExt::flush(&mut lazy_sink).await?;

        assert_eq!(f.await.unwrap().unwrap().unwrap(), data);

        Ok(())
    }
}

pub fn deploy_containerized_o2o(target: &str, bind_addr: &str) -> (syn::Expr, syn::Expr) {
    (
        q!(LazyTcpSink::new(target)).splice_untyped_ctx(&()),
        q!(LazyTcpSource::new(bind_addr.parse().unwrap())).splice_untyped_ctx(&()),
    )
}

pub fn deploy_containerized_e2o(bind_addr: &str) -> syn::Expr {
    q!(LazyTcpSource::new(bind_addr.parse().unwrap())).splice_untyped_ctx(&())
}

pub fn deploy_containerized_o2e(target: &str) -> syn::Expr {
    q!(LazyTcpSink::new(target)).splice_untyped_ctx(&())
}
