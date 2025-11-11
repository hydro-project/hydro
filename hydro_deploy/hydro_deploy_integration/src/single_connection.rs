use std::ops::DerefMut;
use std::pin::Pin;
#[cfg(unix)]
use std::sync::Arc;
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
    pub source: SingleConnectionSource<I, C>,
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

                #[cfg_attr(
                    not(unix),
                    expect(unused_variables, reason = "dir is only used on non-Unix")
                )]
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

                        #[cfg(unix)]
                        {
                            None
                        }

                        #[cfg(not(unix))]
                        {
                            None::<()>
                        }
                    }
                    _ => panic!("SingleConnection only supports UnixSocket and TcpPort"),
                };

                #[cfg(unix)]
                let dir_holder_arc = dir.map(Arc::new);

                let source = SingleConnectionSource {
                    new_stream_receiver,
                    #[cfg(unix)]
                    _dir_holder: dir_holder_arc.clone(),
                    active_stream: None,
                };

                let sink = SingleConnectionSink::<O, C> {
                    #[cfg(unix)]
                    _dir_holder: dir_holder_arc,
                    connection_sink: None,
                    new_sink_receiver,
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

pub struct SingleConnectionSource<I, C: Decoder<Item = I>> {
    new_stream_receiver: mpsc::UnboundedReceiver<DynDecodedStream<I, C>>,
    #[cfg(unix)]
    _dir_holder: Option<Arc<TempDir>>, // keeps the folder containing the socket alive
    /// The active stream for the single connection, if taken
    active_stream: Option<DynDecodedStream<I, C>>,
}

pub struct SingleConnectionSink<O, C: Encoder<O>> {
    #[cfg(unix)]
    _dir_holder: Option<Arc<TempDir>>, // keeps the folder containing the socket alive
    connection_sink: Option<DynEncodedSink<O, C>>,
    new_sink_receiver: mpsc::UnboundedReceiver<DynEncodedSink<O, C>>,
}

impl<I, C: Decoder<Item = I> + Send + Sync + Default + 'static> Stream
    for SingleConnectionSource<I, C>
{
    type Item = Result<I, <C as Decoder>::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let me = self.deref_mut();

        if me.active_stream.is_none() {
            if let Some(stream) = ready!(me.new_stream_receiver.poll_recv(cx)) {
                me.active_stream = Some(stream);
            } else {
                return Poll::Ready(None);
            }
        }

        me.active_stream.as_mut().unwrap().as_mut().poll_next(cx)
    }
}

impl<O, C: Encoder<O>> Sink<O> for SingleConnectionSink<O, C> {
    type Error = <C as Encoder<O>>::Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.connection_sink.is_none() {
            match ready!(self.new_sink_receiver.poll_recv(cx)) {
                Some(sink) => {
                    self.connection_sink = Some(sink);
                }
                None => return Poll::Pending,
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
