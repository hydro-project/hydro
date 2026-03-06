#![allow(
    unused,
    reason = "unused in trybuild but the __staged version is needed"
)]
#![allow(missing_docs, reason = "used internally")]

use std::collections::HashMap;
use std::future::Future;
use std::net::SocketAddr;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use bytes::BytesMut;
use futures::{FutureExt, Sink, SinkExt, Stream, StreamExt};
use proc_macro2::Span;
use sinktools::demux_map_lazy::LazyDemuxSink;
use sinktools::lazy::{LazySink, LazySource};
use sinktools::lazy_sink_source::LazySinkSource;
use stageleft::runtime_support::{
    FreeVariableWithContext, FreeVariableWithContextWithProps, QuoteTokens,
};
use stageleft::{QuotedWithContext, q};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};
use tracing::{debug, instrument, warn};

use crate::location::dynamic::LocationId;
use crate::location::member_id::TaglessMemberId;
use crate::location::{LocationKey, MemberId, MembershipEvent};

/// The single well-known port that every node listens on.
pub const CHANNEL_MUX_PORT: u16 = 10000;

/// Magic constant embedded in every [`ChannelMagic`] header.
pub const CHANNEL_MAGIC: u64 = 0x4859_4452_4f5f_4348;

/// Magic header sent as the very first frame of every channel handshake.
///
/// This is a fixed value that never changes across versions, used to confirm
/// both sides are speaking the same protocol family before anything else.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ChannelMagic {
    pub magic: u64,
}

/// Current protocol version for the channel handshake.
pub const CHANNEL_PROTOCOL_VERSION: u64 = 1;

/// Protocol version sent as the second frame, after [`ChannelMagic`].
///
/// Incremented when the handshake format changes. The receiver checks this
/// to decide how to deserialize the subsequent [`ChannelHandshake`] frame.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ChannelProtocolVersion {
    pub version: u64,
}

/// Handshake message sent by the connecting side to identify the channel.
///
/// The receiver reads the third frame (after [`ChannelMagic`] and
/// [`ChannelProtocolVersion`]) to know which logical channel the connection
/// belongs to, and optionally which cluster member is connecting.
/// cluster member is connecting.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ChannelHandshake {
    /// The logical channel name for this connection.
    pub channel_name: String,
    /// If the sender is a cluster member, this is its identifier
    /// (container name for Docker, task ID for ECS, etc.).
    /// `None` for process-to-process connections.
    pub sender_id: Option<String>,
}

/// A dispatched channel connection: optional sender ID and the read stream.
type MuxConnection = (
    Option<String>,
    FramedRead<OwnedReadHalf, LengthDelimitedCodec>,
);

/// A shared accept loop that listens on a single port and dispatches
/// incoming connections to the right consumer based on the channel name
/// sent in the handshake.
///
/// Each node creates one of these at startup. Individual channels register
/// themselves and receive their connection via a mpsc channel.
pub struct ChannelMux {
    /// Map from channel name to a sender that delivers accepted connections.
    channels: std::sync::Mutex<HashMap<String, tokio::sync::mpsc::UnboundedSender<MuxConnection>>>,
}

impl Default for ChannelMux {
    fn default() -> Self {
        Self::new()
    }
}

impl ChannelMux {
    pub fn new() -> Self {
        Self {
            channels: std::sync::Mutex::new(HashMap::new()),
        }
    }

    pub fn register(
        &self,
        channel_name: String,
    ) -> tokio::sync::mpsc::UnboundedReceiver<MuxConnection> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let mut channels = self.channels.lock().unwrap();
        channels.insert(channel_name, tx);
        rx
    }

    pub async fn run_accept_loop(self: Arc<Self>, listener: TcpListener) {
        loop {
            let (stream, peer) = match listener.accept().await {
                Ok(v) => v,
                Err(e) => {
                    warn!(name: "accept_error", error = %e);
                    continue;
                }
            };
            debug!(name: "mux_accepting", ?peer);

            let mux = self.clone();
            tokio::spawn(async move {
                let (rx, _tx) = stream.into_split();
                let mut source = FramedRead::new(rx, LengthDelimitedCodec::new());

                let magic_frame = match source.next().await {
                    Some(Ok(frame)) => frame,
                    result => {
                        warn!(name: "magic_failed", ?peer, ?result, "no magic frame");
                        return;
                    }
                };

                let magic: ChannelMagic = match bincode::deserialize(&magic_frame) {
                    Ok(m) => m,
                    Err(e) => {
                        warn!(name: "magic_deserialize_failed", ?peer, error = %e);
                        return;
                    }
                };

                if magic.magic != CHANNEL_MAGIC {
                    warn!(name: "bad_magic", ?peer, magic = magic.magic, expected = CHANNEL_MAGIC);
                    return;
                }

                let version_frame = match source.next().await {
                    Some(Ok(frame)) => frame,
                    result => {
                        warn!(name: "version_failed", ?peer, ?result, "no version frame");
                        return;
                    }
                };

                let version: ChannelProtocolVersion = match bincode::deserialize(&version_frame) {
                    Ok(v) => v,
                    Err(e) => {
                        warn!(name: "version_deserialize_failed", ?peer, error = %e);
                        return;
                    }
                };

                if version.version != CHANNEL_PROTOCOL_VERSION {
                    warn!(name: "version_mismatch", ?peer, version = version.version, expected = CHANNEL_PROTOCOL_VERSION);
                    return;
                }

                let handshake_frame = match source.next().await {
                    Some(Ok(frame)) => frame,
                    result => {
                        warn!(name: "handshake_failed", ?peer, ?result, "no handshake frame");
                        return;
                    }
                };

                let handshake: ChannelHandshake = match bincode::deserialize(&handshake_frame) {
                    Ok(h) => h,
                    Err(e) => {
                        warn!(name: "handshake_deserialize_failed", ?peer, error = %e);
                        return;
                    }
                };

                debug!(name: "handshake_received", ?peer, ?handshake);

                let channels = mux.channels.lock().unwrap();
                if let Some(tx_chan) = channels.get(&handshake.channel_name) {
                    let _ = tx_chan.send((handshake.sender_id, source));
                } else {
                    warn!(
                        name: "unknown_channel",
                        channel_name = %handshake.channel_name,
                        ?peer,
                        "no registered consumer for channel"
                    );
                }
            });
        }
    }
}

/// Get or initialize the global ChannelMux for this process.
///
/// The first call creates the TcpListener and spawns the accept loop.
/// Subsequent calls return the same `Arc<ChannelMux>`.
pub fn get_or_init_channel_mux() -> Arc<ChannelMux> {
    use std::sync::OnceLock;
    static MUX: OnceLock<Arc<ChannelMux>> = OnceLock::new();

    MUX.get_or_init(|| {
        let mux = Arc::new(ChannelMux::new());
        let mux_clone = mux.clone();

        // Spawn the accept loop in a background task.
        // We use tokio::spawn which requires a runtime to be active.
        tokio::spawn(async move {
            let bind_addr = format!("0.0.0.0:{}", CHANNEL_MUX_PORT);
            debug!(name: "mux_listening", %bind_addr);
            let listener = TcpListener::bind(&bind_addr)
                .await
                .expect("failed to bind channel mux listener");
            mux_clone.run_accept_loop(listener).await;
        });

        mux
    })
    .clone()
}

/// Sends a [`ChannelMagic`], then a [`ChannelProtocolVersion`], then a
/// [`ChannelHandshake`] as three separate frames over the given sink.
pub async fn send_handshake(
    sink: &mut FramedWrite<TcpStream, LengthDelimitedCodec>,
    channel_name: &str,
    sender_id: Option<&str>,
) -> Result<(), std::io::Error> {
    let magic = ChannelMagic {
        magic: CHANNEL_MAGIC,
    };
    sink.send(bytes::Bytes::from(bincode::serialize(&magic).unwrap()))
        .await?;

    let version = ChannelProtocolVersion {
        version: CHANNEL_PROTOCOL_VERSION,
    };
    sink.send(bytes::Bytes::from(bincode::serialize(&version).unwrap()))
        .await?;

    let handshake = ChannelHandshake {
        channel_name: channel_name.to_owned(),
        sender_id: sender_id.map(|s| s.to_owned()),
    };
    sink.send(bytes::Bytes::from(bincode::serialize(&handshake).unwrap()))
        .await?;
    Ok(())
}

pub fn deploy_containerized_o2o(target: &str, channel_name: &str) -> (syn::Expr, syn::Expr) {
    (
        q!(LazySink::<_, _, _, bytes::Bytes>::new(move || Box::pin(
            async move {
                let channel_name = channel_name;
                let target = format!("{}:{}", target, self::CHANNEL_MUX_PORT);
                debug!(name: "connecting", %target, %channel_name);

                let stream = TcpStream::connect(&target).await?;
                let mut sink = FramedWrite::new(stream, LengthDelimitedCodec::new());

                self::send_handshake(&mut sink, channel_name, None).await?;

                Result::<_, std::io::Error>::Ok(sink)
            }
        )))
        .splice_untyped_ctx(&()),
        q!(LazySource::new(move || Box::pin(async move {
            let channel_name = channel_name;
            let mux = self::get_or_init_channel_mux();
            let mut rx = mux.register(channel_name.to_owned());

            let (_sender_id, source) = rx.recv().await.ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::ConnectionReset, "channel mux closed")
            })?;

            debug!(name: "o2o_channel_connected", %channel_name);

            Result::<_, std::io::Error>::Ok(source)
        })))
        .splice_untyped_ctx(&()),
    )
}

pub fn deploy_containerized_o2m(channel_name: &str) -> (syn::Expr, syn::Expr) {
    (
        q!(sinktools::demux_map_lazy::<_, _, _, _>(
            move |key: &TaglessMemberId| {
                let key = key.clone();
                let channel_name = channel_name.to_owned();

                LazySink::<_, _, _, bytes::Bytes>::new(move || {
                    Box::pin(async move {
                        let target =
                            format!("{}:{}", key.get_container_name(), self::CHANNEL_MUX_PORT);
                        debug!(name: "connecting", %target, channel_name = %channel_name);

                        let stream = TcpStream::connect(&target).await?;
                        let mut sink = FramedWrite::new(stream, LengthDelimitedCodec::new());

                        self::send_handshake(&mut sink, &channel_name, None).await?;

                        Result::<_, std::io::Error>::Ok(sink)
                    })
                })
            }
        ))
        .splice_untyped_ctx(&()),
        q!(LazySource::new(move || Box::pin(async move {
            let channel_name = channel_name;
            let mux = self::get_or_init_channel_mux();
            let mut rx = mux.register(channel_name.to_owned());

            let (_sender_id, source) = rx.recv().await.ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::ConnectionReset, "channel mux closed")
            })?;

            debug!(name: "o2m_channel_connected", %channel_name);

            Result::<_, std::io::Error>::Ok(source)
        })))
        .splice_untyped_ctx(&()),
    )
}

pub fn deploy_containerized_m2o(target_host: &str, channel_name: &str) -> (syn::Expr, syn::Expr) {
    (
        q!(LazySink::<_, _, _, bytes::Bytes>::new(move || {
            Box::pin(async move {
                let channel_name = channel_name;
                let target = format!("{}:{}", target_host, self::CHANNEL_MUX_PORT);
                debug!(name: "connecting", %target, %channel_name);

                let stream = TcpStream::connect(&target).await?;
                let mut sink = FramedWrite::new(stream, LengthDelimitedCodec::new());

                let container_name = std::env::var("CONTAINER_NAME").unwrap();
                self::send_handshake(&mut sink, channel_name, Some(&container_name)).await?;

                Result::<_, std::io::Error>::Ok(sink)
            })
        }))
        .splice_untyped_ctx(&()),
        q!(LazySource::new(move || Box::pin(async move {
            let channel_name = channel_name;
            let mux = self::get_or_init_channel_mux();
            let mut rx = mux.register(channel_name.to_owned());

            Result::<_, std::io::Error>::Ok(
                futures::stream::unfold(rx, |mut rx| {
                    Box::pin(async move {
                        let (sender_id, source) = rx.recv().await?;
                        let from = sender_id.expect("m2o sender must provide container name");

                        debug!(name: "m2o_channel_connected", %from);

                        Some((
                            source.map(move |v| {
                                v.map(|v| (TaglessMemberId::from_container_name(from.clone()), v))
                            }),
                            rx,
                        ))
                    })
                })
                .flatten_unordered(None),
            )
        })))
        .splice_untyped_ctx(&()),
    )
}

pub fn deploy_containerized_m2m(channel_name: &str) -> (syn::Expr, syn::Expr) {
    (
        q!(sinktools::demux_map_lazy::<_, _, _, _>(
            move |key: &TaglessMemberId| {
                let key = key.clone();
                let channel_name = channel_name.to_owned();

                LazySink::<_, _, _, bytes::Bytes>::new(move || {
                    Box::pin(async move {
                        let target =
                            format!("{}:{}", key.get_container_name(), self::CHANNEL_MUX_PORT);
                        debug!(name: "connecting", %target, channel_name = %channel_name);

                        let stream = TcpStream::connect(&target).await?;
                        let mut sink = FramedWrite::new(stream, LengthDelimitedCodec::new());

                        let container_name = std::env::var("CONTAINER_NAME").unwrap();
                        self::send_handshake(&mut sink, &channel_name, Some(&container_name))
                            .await?;

                        Result::<_, std::io::Error>::Ok(sink)
                    })
                })
            }
        ))
        .splice_untyped_ctx(&()),
        q!(LazySource::new(move || Box::pin(async move {
            let channel_name = channel_name;
            let mux = self::get_or_init_channel_mux();
            let mut rx = mux.register(channel_name.to_owned());

            Result::<_, std::io::Error>::Ok(
                futures::stream::unfold(rx, |mut rx| {
                    Box::pin(async move {
                        let (sender_id, source) = rx.recv().await?;
                        let from = sender_id.expect("m2m sender must provide container name");

                        debug!(name: "m2m_channel_connected", %from);

                        Some((
                            source.map(move |v| {
                                v.map(|v| (TaglessMemberId::from_container_name(from.clone()), v))
                            }),
                            rx,
                        ))
                    })
                })
                .flatten_unordered(None),
            )
        })))
        .splice_untyped_ctx(&()),
    )
}

pub struct SocketIdent {
    pub socket_ident: syn::Ident,
}

impl<Ctx> FreeVariableWithContextWithProps<Ctx, ()> for SocketIdent {
    type O = TcpListener;

    fn to_tokens(self, _ctx: &Ctx) -> (QuoteTokens, ())
    where
        Self: Sized,
    {
        let ident = self.socket_ident;

        (
            QuoteTokens {
                prelude: None,
                expr: Some(quote::quote! { #ident }),
            },
            (),
        )
    }
}

pub fn deploy_containerized_external_sink_source_ident(socket_ident: syn::Ident) -> syn::Expr {
    let socket_ident = SocketIdent { socket_ident };

    q!(LazySinkSource::<
        _,
        FramedRead<OwnedReadHalf, LengthDelimitedCodec>,
        FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>,
        bytes::Bytes,
        std::io::Error,
    >::new(async move {
        let (stream, peer) = socket_ident.accept().await?;
        debug!(name: "external accepting", ?peer);
        let (rx, tx) = stream.into_split();

        let fr = FramedRead::new(rx, LengthDelimitedCodec::new());
        let fw = FramedWrite::new(tx, LengthDelimitedCodec::new());

        Result::<_, std::io::Error>::Ok((fr, fw))
    },))
    .splice_untyped_ctx(&())
}

pub fn cluster_ids<'a>() -> impl QuotedWithContext<'a, &'a [TaglessMemberId], ()> + Clone {
    // unimplemented!(); // this is unused.

    // This is a dummy piece of code, since clusters are dynamic when containerized.
    q!(Box::leak(Box::new([TaglessMemberId::from_container_name(
        "INVALID CONTAINER NAME cluster_ids"
    )]))
    .as_slice())
}

#[cfg(feature = "docker_runtime")]
pub fn cluster_self_id<'a>() -> impl QuotedWithContext<'a, TaglessMemberId, ()> + Clone + 'a {
    q!(TaglessMemberId::from_container_name(
        std::env::var("CONTAINER_NAME").unwrap()
    ))
}

#[cfg(feature = "docker_runtime")]
pub fn cluster_membership_stream<'a>(
    location_id: &LocationId,
) -> impl QuotedWithContext<'a, Box<dyn Stream<Item = (TaglessMemberId, MembershipEvent)> + Unpin>, ()>
{
    let key = location_id.key();

    q!(Box::new(self::docker_membership_stream(
        std::env::var("DEPLOYMENT_INSTANCE").unwrap(),
        key
    ))
        as Box<
            dyn Stream<Item = (TaglessMemberId, MembershipEvent)> + Unpin,
        >)
}

#[cfg(feature = "docker_runtime")]
// There's a risk of race conditions here since all the containers will be starting up at the same time.
// So we need to start listening for events and the take a snapshot of currently running containers, since they may have already started up before we started listening to events.
// Then we need to turn that into a usable stream for the consumer in this current hydro program. The way you do that is by emitting from the snapshot first, and then start emitting from the stream. Keep a hash set around to track whether a container is up or down.
#[instrument(skip_all, fields(%deployment_instance, %location_key))]
fn docker_membership_stream(
    deployment_instance: String,
    location_key: LocationKey,
) -> impl Stream<Item = (TaglessMemberId, MembershipEvent)> + Unpin {
    use std::collections::HashSet;
    use std::sync::{Arc, Mutex};

    use bollard::Docker;
    use bollard::query_parameters::{EventsOptions, ListContainersOptions};
    use tokio::sync::mpsc;

    let docker = Docker::connect_with_local_defaults()
        .unwrap()
        .with_timeout(Duration::from_secs(1));

    let (event_tx, event_rx) = mpsc::unbounded_channel::<(String, MembershipEvent)>();

    // 1. Start event subscription in a spawned task
    let events_docker = docker.clone();
    let events_deployment_instance = deployment_instance.clone();
    tokio::spawn(async move {
        let mut filters = HashMap::new();
        filters.insert("type".to_owned(), vec!["container".to_owned()]);
        filters.insert(
            "event".to_owned(),
            vec!["start".to_owned(), "die".to_owned()],
        );
        let event_options = Some(EventsOptions {
            filters: Some(filters),
            ..Default::default()
        });

        let mut events = events_docker.events(event_options);
        while let Some(event) = events.next().await {
            if let Some((name, membership_event)) = event.ok().and_then(|e| {
                let name = e
                    .actor
                    .as_ref()
                    .and_then(|a| a.attributes.as_ref())
                    .and_then(|attrs| attrs.get("name"))
                    .map(|s| &**s)?;

                if name.contains(format!("{events_deployment_instance}-{location_key}").as_str()) {
                    match e.action.as_deref() {
                        Some("start") => Some((name.to_owned(), MembershipEvent::Joined)),
                        Some("die") => Some((name.to_owned(), MembershipEvent::Left)),
                        _ => None,
                    }
                } else {
                    None
                }
            }) && event_tx.send((name, membership_event)).is_err()
            {
                break;
            }
        }
    });

    // Shared state for deduplication across snapshot and events phases
    let seen_joined = Arc::new(Mutex::new(HashSet::<String>::new()));
    let seen_joined_snapshot = seen_joined.clone();
    let seen_joined_events = seen_joined;

    // 2. Snapshot stream - fetch current containers and emit Joined events
    let snapshot_stream = futures::stream::once(async move {
        let mut filters = HashMap::new();
        filters.insert(
            "name".to_owned(),
            vec![format!("{deployment_instance}-{location_key}")],
        );
        let options = Some(ListContainersOptions {
            filters: Some(filters),
            ..Default::default()
        });

        docker
            .list_containers(options)
            .await
            .unwrap_or_default()
            .iter()
            .filter_map(|c| c.names.as_deref())
            .filter_map(|names| names.first())
            .map(|name| name.trim_start_matches('/'))
            .filter(|&name| seen_joined_snapshot.lock().unwrap().insert(name.to_owned()))
            .map(|name| (name.to_owned(), MembershipEvent::Joined))
            .collect::<Vec<_>>()
    })
    .flat_map(futures::stream::iter);

    // 3. Events stream - process live events with deduplication
    let events_stream = tokio_stream::StreamExt::filter_map(
        tokio_stream::wrappers::UnboundedReceiverStream::new(event_rx),
        move |(name, event)| {
            let mut seen = seen_joined_events.lock().unwrap();
            match event {
                MembershipEvent::Joined => {
                    if seen.insert(name.to_owned()) {
                        Some((name, MembershipEvent::Joined))
                    } else {
                        None
                    }
                }
                MembershipEvent::Left => seen.take(&name).map(|name| (name, MembershipEvent::Left)),
            }
        },
    );

    // 4. Chain snapshot then events
    Box::pin(
        snapshot_stream
            .chain(events_stream)
            .map(|(k, v)| (TaglessMemberId::from_container_name(k), v))
            .inspect(|(member_id, event)| debug!(name: "membership_event", ?member_id, ?event)),
    )
}
