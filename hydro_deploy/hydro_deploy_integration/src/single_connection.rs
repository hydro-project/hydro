use std::marker::PhantomData;
use std::ops::DerefMut;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::{Sink, SinkExt, Stream, StreamExt, ready};
#[cfg(unix)]
use tempfile::TempDir;
use tokio::sync::mpsc;
use tokio_util::codec::{Decoder, Encoder, Framed};

use crate::{AcceptedServer, BoundServer, Connected, Connection};

/// A connected implementation which only allows a single live connection for the
/// lifetime of the server. The first accepted connection is kept and its sink
/// and stream are exposed; any subsequent connections are ignored.
pub struct ConnectedSingleConnection<I, O, C: Decoder<Item = I> + Encoder<O>> {
    pub source: SingleConnectionSource<I, O, C>,
    pub sink: SingleConnectionSink<O, C>,
}

impl<
    I: 'static,
    O: Send + Sync + 'static,
    C: Decoder<Item = I> + Encoder<O> + Send + Sync + Default + 'static,
> Connected for ConnectedSingleConnection<I, O, C>
{
    fn from_defn(pipe: Connection) -> Self {
        match pipe {
            Connection::AsServer(AcceptedServer::MultiConnection(bound_server)) => {
                let (new_stream_sender, new_stream_receiver) = mpsc::unbounded_channel();
                let (new_sink_sender, new_sink_receiver) = mpsc::unbounded_channel();

                let dir = match *bound_server {
                    #[cfg(unix)]
                    BoundServer::UnixSocket(listener, dir) => {
                        tokio::spawn(async move {
                            tokio::task::yield_now().await;
                            match listener.accept().await {
                                Ok((stream, _)) => {
                                    let framed = Framed::new(stream, C::default());
                                    let (sink, stream) = framed.split();

                                    let boxed_stream: DynDecodedStream<I, C> = Box::pin(stream);
                                    let boxed_sink: DynEncodedSink<O, C> =
                                        Box::pin(sink.buffer(1024));

                                    let _ = new_stream_sender.send(boxed_stream);
                                    let _ = new_sink_sender.send(boxed_sink);
                                }
                                Err(e) => {
                                    eprintln!("Error accepting Unix connection: {}", e);
                                }
                            }
                        });

                        Some(dir)
                    }
                    BoundServer::TcpPort(listener, _) => {
                        tokio::spawn(async move {
                            tokio::task::yield_now().await;
                            match listener.into_inner().accept().await {
                                Ok((stream, _)) => {
                                    let framed = Framed::new(stream, C::default());
                                    let (sink, stream) = framed.split();

                                    let boxed_stream: DynDecodedStream<I, C> = Box::pin(stream);
                                    let boxed_sink: DynEncodedSink<O, C> =
                                        Box::pin(sink.buffer(1024));

                                    let _ = new_stream_sender.send(boxed_stream);
                                    let _ = new_sink_sender.send(boxed_sink);
                                }
                                Err(e) => {
                                    eprintln!("Error accepting TCP connection: {}", e);
                                }
                            }
                        });

                        None
                    }
                    _ => panic!("SingleConnection only supports UnixSocket and TcpPort"),
                };

                let source = SingleConnectionSource {
                    new_stream_receiver,
                    _dir_holder: dir,
                    active_stream: None,
                    _phantom: Default::default(),
                };

                let sink = SingleConnectionSink::<O, C> {
                    connection_sink: None,
                    new_sink_receiver,
                    _phantom: Default::default(),
                };

                ConnectedSingleConnection { source, sink }
            }
            _ => panic!("Cannot connect to a non-multi-connection pipe as a single-connection"),
        }
    }
}

type DynDecodedStream<I, C> =
    Pin<Box<dyn Stream<Item = Result<I, <C as Decoder>::Error>> + Send + Sync>>;
type DynEncodedSink<O, C> = Pin<Box<dyn Sink<O, Error = <C as Encoder<O>>::Error> + Send + Sync>>;

pub struct SingleConnectionSource<I, O, C: Decoder<Item = I> + Encoder<O>> {
    new_stream_receiver: mpsc::UnboundedReceiver<DynDecodedStream<I, C>>,
    #[cfg(unix)]
    _dir_holder: Option<TempDir>, // keeps the folder containing the socket alive
    /// The active stream for the single connection, if taken
    active_stream: Option<DynDecodedStream<I, C>>,
    _phantom: PhantomData<(Box<O>, Box<C>)>,
}

pub struct SingleConnectionSink<O, C: Encoder<O>> {
    connection_sink: Option<DynEncodedSink<O, C>>,
    new_sink_receiver: mpsc::UnboundedReceiver<DynEncodedSink<O, C>>,
    _phantom: PhantomData<(Box<O>, Box<C>)>,
}

impl<
    I,
    O: Send + Sync + 'static,
    C: Decoder<Item = I> + Encoder<O> + Send + Sync + Default + 'static,
> Stream for SingleConnectionSource<I, O, C>
{
    type Item = Result<I, <C as Decoder>::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let me = self.deref_mut();

        if me.active_stream.is_none() {
            if let Some(stream) = ready!(me.new_stream_receiver.poll_recv(cx)) {
                me.active_stream = Some(stream);
                return me.active_stream.as_mut().unwrap().as_mut().poll_next(cx);
            } else {
                return Poll::Ready(None);
            }
        }

        return me.active_stream.as_mut().unwrap().as_mut().poll_next(cx);
    }
}

impl<O, C: Encoder<O>> Sink<O> for SingleConnectionSink<O, C> {
    type Error = <C as Encoder<O>>::Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.connection_sink.is_none() {
            match self.new_sink_receiver.poll_recv(cx) {
                Poll::Ready(Some(sink)) => {
                    self.connection_sink = Some(sink);
                }
                Poll::Ready(None) => return Poll::Pending,
                Poll::Pending => return Poll::Pending,
            }
        }

        self.connection_sink
            .as_mut()
            .unwrap()
            .as_mut()
            .poll_ready(cx)
    }

    fn start_send(mut self: Pin<&mut Self>, item: O) -> Result<(), Self::Error> {
        self.connection_sink
            .as_mut()
            .unwrap()
            .as_mut()
            .start_send(item)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if let Some(sink) = self.connection_sink.as_mut() {
            sink.as_mut().poll_flush(cx)
        } else {
            Poll::Ready(Ok(()))
        }
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if let Some(sink) = self.connection_sink.as_mut() {
            sink.as_mut().poll_close(cx)
        } else {
            Poll::Ready(Ok(()))
        }
    }
}
