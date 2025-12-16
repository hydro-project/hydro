//! [`BufferedLazySinkSource`], and related items.

use core::future::Future;
use core::marker::PhantomData;
use core::pin::Pin;
use core::task::{Context, Poll};

use futures_util::{Sink, SinkExt, Stream, StreamExt};
use tokio::sync::mpsc;

/// A buffered lazy sink-source that immediately spawns a task to initialize the connection.
/// Uses tokio channels to forward data between the spawned task and the sink/stream halves.
pub struct BufferedLazySinkSource<Item, StreamItem, Error> {
    sink_tx: mpsc::UnboundedSender<Item>,
    stream_rx: mpsc::UnboundedReceiver<StreamItem>,
    _phantom: PhantomData<Error>,
}

impl<Item, StreamItem, Error> BufferedLazySinkSource<Item, StreamItem, Error>
where
    Item: Send + 'static,
    StreamItem: Send + 'static,
    Error: Send + core::error::Error + 'static,
{
    /// Creates a new `BufferedLazySinkSource` with the given initialization future.
    /// Immediately spawns a task to run the future.
    pub fn new<Fut, St, Si>(future: Fut) -> Self
    where
        Fut: Future<Output = Result<(St, Si), Error>> + Send + 'static,
        St: Stream<Item = StreamItem> + Send + Unpin + 'static,
        Si: Sink<Item> + Send + Unpin + 'static,
        Si::Error: core::error::Error,
    {
        let (sink_tx, mut sink_rx) = mpsc::unbounded_channel::<Item>();
        let (stream_tx, stream_rx) = mpsc::unbounded_channel::<StreamItem>();

        tokio::task::spawn(async move {
            match future.await {
                Ok((mut stream, mut sink)) => loop {
                    tokio::select! {
                        item = sink_rx.recv() => {
                            if let Some(item) = item {
                                sink.send(item).await.unwrap();
                            }
                        }
                        item = stream.next() => {
                            if let Some(item) = item {
                                stream_tx.send(item).unwrap();
                            }
                        }
                    }
                },
                Err(err) => {
                    panic!("Failed to initialize BufferedLazySinkSource: {err}");
                }
            }
        });

        Self {
            sink_tx,
            stream_rx,
            _phantom: Default::default(),
        }
    }

    /// Splits into a sink and stream that share the same underlying connection.
    pub fn split(
        self,
    ) -> (
        BufferedLazySinkHalf<Item, Error>,
        BufferedLazySourceHalf<StreamItem, Error>,
    ) {
        let sink = BufferedLazySinkHalf {
            tx: self.sink_tx,
            _phantom: PhantomData,
        };
        let stream = BufferedLazySourceHalf {
            rx: self.stream_rx,
            _phantom: PhantomData,
        };
        (sink, stream)
    }
}

/// Sink half of the BufferedLazySinkSource
pub struct BufferedLazySinkHalf<Item, Error> {
    tx: mpsc::UnboundedSender<Item>,
    _phantom: PhantomData<Error>,
}

/// Stream half of the BufferedLazySinkSource
pub struct BufferedLazySourceHalf<StreamItem, Error> {
    rx: mpsc::UnboundedReceiver<StreamItem>,
    _phantom: PhantomData<Error>,
}

impl<Item, Error> Sink<Item> for BufferedLazySinkHalf<Item, Error> {
    type Error = Error;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        self.tx.send(item).unwrap();
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

impl<StreamItem, Error: Unpin> Stream for BufferedLazySourceHalf<StreamItem, Error> {
    type Item = StreamItem;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

#[cfg(test)]
mod test {
    use futures_util::{SinkExt, StreamExt};

    use super::*;

    #[tokio::test]
    async fn tcp_bidirectional_communication() {
        use tokio::net::TcpListener;
        use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let sink_source = BufferedLazySinkSource::new(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let (rx, tx) = stream.into_split();
            let fr = FramedRead::new(rx, LengthDelimitedCodec::new());
            let fw = FramedWrite::new(tx, LengthDelimitedCodec::new());
            Ok::<_, std::io::Error>((fr, fw))
        });

        let (mut sink, mut stream) = sink_source.split();

        let mut socket = tokio::net::TcpStream::connect(addr).await.unwrap();
        let (client_rx, client_tx) = socket.split();
        let mut client_tx = FramedWrite::new(client_tx, LengthDelimitedCodec::new());
        let mut client_rx = FramedRead::new(client_rx, LengthDelimitedCodec::new());

        SinkExt::send(&mut client_tx, bytes::Bytes::from("hello"))
            .await
            .unwrap();

        assert_eq!(&stream.next().await.unwrap().unwrap()[..], b"hello");

        SinkExt::send(&mut sink, bytes::Bytes::from("world"))
            .await
            .unwrap();

        assert_eq!(&client_rx.next().await.unwrap().unwrap()[..], b"world");
    }

    #[tokio::test]
    async fn immediate_initialization() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        use tokio::net::TcpListener;
        use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

        let initialized = Arc::new(AtomicBool::new(false));
        let initialized_clone = initialized.clone();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();

        let _sink_source = BufferedLazySinkSource::<
            bytes::Bytes,
            Result<bytes::BytesMut, std::io::Error>,
            std::io::Error,
        >::new(async move {
            initialized_clone.store(true, Ordering::SeqCst);
            let (stream, _) = listener.accept().await.unwrap();
            let (rx, tx) = stream.into_split();
            let fr = FramedRead::new(rx, LengthDelimitedCodec::new());
            let fw = FramedWrite::new(tx, LengthDelimitedCodec::new());
            Ok::<_, std::io::Error>((fr, fw))
        });

        tokio::task::yield_now().await;
        assert!(initialized.load(Ordering::SeqCst));
    }
}
