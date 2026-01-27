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
use tracing::{debug, instrument};

use crate::location::dynamic::LocationId;
use crate::location::member_id::TaglessMemberId;
use crate::location::{LocationKey, MemberId, MembershipEvent};

pub fn deploy_containerized_o2o(target: &str, bind_addr: &str) -> (syn::Expr, syn::Expr) {
    (
        q!(LazySink::<_, _, _, bytes::Bytes>::new(move || Box::pin(
            async move {
                let target = target;
                debug!(name: "connecting", %target);
                Result::<_, std::io::Error>::Ok(FramedWrite::new(
                    TcpStream::connect(target).await?,
                    LengthDelimitedCodec::new(),
                ))
            }
        )))
        .splice_untyped_ctx(&()),
        q!(LazySource::new(move || Box::pin(async move {
            let listener = TcpListener::bind(bind_addr).await?;
            let (stream, peer) = listener.accept().await?;
            debug!(name: "accepting", ?peer);
            Result::<_, std::io::Error>::Ok(FramedRead::new(stream, LengthDelimitedCodec::new()))
        })))
        .splice_untyped_ctx(&()),
    )
}

pub fn deploy_containerized_o2m(port: u16) -> (syn::Expr, syn::Expr) {
    (
        QuotedWithContext::<'static, LazyDemuxSink<TaglessMemberId, _, _>, ()>::splice_untyped_ctx(
            q!(sinktools::demux_map_lazy::<_, _, _, _>(
                move |key: &TaglessMemberId| {
                    let key = key.clone();

                    LazySink::<_, _, _, bytes::Bytes>::new(move || {
                        Box::pin(async move {
                            let port = port;
                            debug!(name: "connecting", target = format!("{}:{}", key.get_container_name(), port));
                            let mut sink = FramedWrite::new(
                                TcpStream::connect(format!(
                                    "{}:{}",
                                    key.get_container_name(),
                                    port
                                ))
                                .await?,
                                LengthDelimitedCodec::new(),
                            );

                            Result::<_, std::io::Error>::Ok(sink)
                        })
                    })
                }
            )),
            &(),
        ),
        q!(LazySource::new(move || Box::pin(async move {
            let bind_addr = format!("0.0.0.0:{}", port);
            debug!(name: "listening", %bind_addr);
            let listener = TcpListener::bind(bind_addr).await?;
            let (stream, peer) = listener.accept().await?;
            debug!(name: "accepting", ?peer);

            Result::<_, std::io::Error>::Ok(FramedRead::new(stream, LengthDelimitedCodec::new()))
        })))
        .splice_untyped_ctx(&()),
    )
}

pub fn deploy_containerized_m2o(port: u16, target_host: &str) -> (syn::Expr, syn::Expr) {
    (
        q!(LazySink::<_, _, _, bytes::Bytes>::new(move || {
            Box::pin(async move {
                let target = format!("{}:{}", target_host, port);
                debug!(name: "connecting", %target);

                let mut sink = FramedWrite::new(
                    TcpStream::connect(target).await?,
                    LengthDelimitedCodec::new(),
                );

                sink.send(bytes::Bytes::from(
                    bincode::serialize(&std::env::var("CONTAINER_NAME").unwrap())
                        .unwrap(),
                ))
                .await?;

                Result::<_, std::io::Error>::Ok(sink)
            })
        }))
        .splice_untyped_ctx(&()),
        QuotedWithContext::<'static, LazySource<_, _, _, Result<(TaglessMemberId, BytesMut), _>>, ()>::splice_untyped_ctx(
            q!(LazySource::new(move || Box::pin(async move {
                let bind_addr = format!("0.0.0.0:{}", port);
                debug!(name: "listening", %bind_addr);
                let listener = TcpListener::bind(bind_addr).await?;
                Result::<_, std::io::Error>::Ok(
                    futures::stream::unfold(listener, |listener| {
                        Box::pin(async move {
                            let (stream, peer) = listener.accept().await.ok()?;
                            let mut source = FramedRead::new(stream, LengthDelimitedCodec::new());
                            let from =
                                bincode::deserialize::<String>(&source.next().await?.ok()?[..])
                                    .ok()?;

                            debug!(name: "accepting", endpoint = format!("{}:{}", peer, from));

                            Some((
                                source.map(move |v| {
                                    v.map(|v| (TaglessMemberId::from_container_name(from.clone()), v))
                                }),
                                listener,
                            ))
                        })
                    })
                    .flatten_unordered(None),
                )
            }))),
            &(),
        ),
    )
}

pub fn deploy_containerized_m2m(port: u16) -> (syn::Expr, syn::Expr) {
    (
        QuotedWithContext::<'static, LazyDemuxSink<TaglessMemberId, _, _>, ()>::splice_untyped_ctx(
            q!(sinktools::demux_map_lazy::<_, _, _, _>(
                move |key: &TaglessMemberId| {
                    let key = key.clone();

                    LazySink::<_, _, _, bytes::Bytes>::new(move || {
                        Box::pin(async move {
                            let port = port;
                            debug!(name: "connecting", target = format!("{}:{}", key.get_container_name(), port));
                            let mut sink = FramedWrite::new(
                                TcpStream::connect(format!(
                                    "{}:{}",
                                    key.get_container_name(),
                                    port
                                ))
                                .await?,
                                LengthDelimitedCodec::new(),
                            );
                            debug!(name: "connected", target = format!("{}:{}", key.get_container_name(), port));

                            sink.send(bytes::Bytes::from(
                                bincode::serialize(&std::env::var("CONTAINER_NAME").unwrap())
                                    .unwrap(),
                            ))
                            .await?;

                            Result::<_, std::io::Error>::Ok(sink)
                        })
                    })
                }
            )),
            &(),
        ),
        QuotedWithContext::<'static, LazySource<_, _, _, Result<(TaglessMemberId, BytesMut), _>>, ()>::splice_untyped_ctx(
            q!(LazySource::new(move || Box::pin(async move {
                let bind_addr = format!("0.0.0.0:{}", port);
                debug!(name: "listening", %bind_addr);
                let listener = TcpListener::bind(bind_addr).await?;

                Result::<_, std::io::Error>::Ok(
                    futures::stream::unfold(listener, |listener| {
                        Box::pin(async move {
                            let (stream, peer) = listener.accept().await.ok()?;
                            let mut source = FramedRead::new(stream, LengthDelimitedCodec::new());
                            let from =
                                bincode::deserialize::<String>(&source.next().await?.ok()?[..])
                                    .ok()?;

                            debug!(name: "accepting", endpoint = format!("{}:{}", peer, from));

                            Some((
                                source.map(move |v| {
                                    v.map(|v| (TaglessMemberId::from_container_name(from.clone()), v))
                                }),
                                listener,
                            ))
                        })
                    })
                    .flatten_unordered(None),
                )
            }))),
            &(),
        ),
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

pub fn deploy_containerized_external_many_sink_source_ident(socket_ident: syn::Ident) -> syn::Expr {
    let socket_ident = SocketIdent { socket_ident };

    q!({
        let listener = socket_ident;
        let (source, sink, membership) =
            hydro_deploy_integration::multi_connection::tcp_multi_connection(listener);
        (source, sink, membership)
    })
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

pub fn cluster_self_id<'a>() -> impl QuotedWithContext<'a, TaglessMemberId, ()> + Clone + 'a {
    q!(TaglessMemberId::from_container_name(
        std::env::var("CONTAINER_NAME").unwrap()
    ))
}

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
        filters.insert("type".to_string(), vec!["container".to_string()]);
        filters.insert(
            "event".to_string(),
            vec!["start".to_string(), "die".to_string()],
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
                    .and_then(|a| a.attributes.and_then(|attrs| attrs.get("name").cloned()))?;

                if name.contains(format!("{events_deployment_instance}-{location_key}").as_str()) {
                    match e.action.as_deref() {
                        Some("start") => Some((name.clone(), MembershipEvent::Joined)),
                        Some("die") => Some((name, MembershipEvent::Left)),
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
            "name".to_string(),
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
            .into_iter()
            .filter_map(|c| {
                c.names
                    .and_then(|names| names.first().map(|n| n.trim_start_matches('/').to_string()))
            })
            .filter_map(|name| {
                if seen_joined_snapshot.lock().unwrap().insert(name.clone()) {
                    Some((name, MembershipEvent::Joined))
                } else {
                    None
                }
            })
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
                    if seen.insert(name.clone()) {
                        Some((name, MembershipEvent::Joined))
                    } else {
                        None
                    }
                }
                MembershipEvent::Left => {
                    if seen.remove(&name) {
                        Some((name, MembershipEvent::Left))
                    } else {
                        None
                    }
                }
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
