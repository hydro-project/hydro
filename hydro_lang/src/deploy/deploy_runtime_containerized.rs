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
use stageleft::runtime_support::{FreeVariableWithContext, QuoteTokens};
use stageleft::{QuotedWithContext, q};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};
use tracing::{debug, instrument};

use crate::location::dynamic::LocationId;
use crate::location::member_id::TaglessMemberId;
use crate::location::{MemberId, MembershipEvent};

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

impl<Ctx> FreeVariableWithContext<Ctx> for SocketIdent {
    type O = TcpListener;

    fn to_tokens(self, _ctx: &Ctx) -> QuoteTokens
    where
        Self: Sized,
    {
        let ident = self.socket_ident;

        QuoteTokens {
            prelude: None,
            expr: Some(quote::quote! { #ident }),
        }
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

pub fn cluster_self_id<'a>() -> impl QuotedWithContext<'a, TaglessMemberId, ()> + Clone + 'a {
    q!(TaglessMemberId::from_container_name(
        std::env::var("CONTAINER_NAME").unwrap()
    ))
}

pub fn cluster_membership_stream<'a>(
    location_id: &LocationId,
) -> impl QuotedWithContext<'a, Box<dyn Stream<Item = (TaglessMemberId, MembershipEvent)> + Unpin>, ()>
{
    let raw_id = location_id.raw_id();

    q!(Box::new(self::docker_membership_stream(
        std::env::var("DEPLOYMENT_INSTANCE").unwrap(),
        raw_id
    ))
        as Box<
            dyn Stream<Item = (TaglessMemberId, MembershipEvent)> + Unpin,
        >)
}

#[instrument(skip_all, fields(%deployment_instance, %location_id))]
fn docker_membership_stream(
    deployment_instance: String,
    location_id: usize,
) -> impl Stream<Item = (TaglessMemberId, MembershipEvent)> + Unpin {
    use bollard::Docker;
    use bollard::container::ListContainersOptions;
    use bollard::system::EventsOptions;
    use futures::stream::{StreamExt, once};
    let docker = Docker::connect_with_local_defaults()
        .unwrap()
        .with_timeout(Duration::from_secs(1));

    let mut filters = HashMap::new();
    filters.insert("type".to_string(), vec!["container".to_string()]);
    filters.insert(
        "event".to_string(),
        vec!["start".to_string(), "die".to_string()],
    );
    let event_options = Some(EventsOptions {
        filters,
        ..Default::default()
    });

    let events = {
        let deployment_instance = deployment_instance.clone();
        docker.events(event_options).filter_map(move |event| {
            std::future::ready(event.ok().and_then(|e| {
                let name = e
                    .actor
                    .and_then(|a| a.attributes.and_then(|attrs| attrs.get("name").cloned()))?;

                if name.contains(format!("{deployment_instance}-{location_id}").as_str()) {
                    match e.action.as_deref() {
                        Some("start") => Some((name.clone(), MembershipEvent::Joined)),
                        Some("die") => Some((name, MembershipEvent::Left)),
                        _ => None,
                    }
                } else {
                    None
                }
            }))
        })
    };

    let initial = once(async move {
        let mut filters = HashMap::new();

        filters.insert(
            "name".to_string(),
            vec![format!("{deployment_instance}-{location_id}")],
        );

        let options = Some(ListContainersOptions {
            // all: true,
            filters,
            ..Default::default()
        });

        let ret = docker
            .list_containers(options)
            .await
            .unwrap()
            .into_iter()
            .filter_map(|c| {
                c.names
                    .and_then(|names| names.first().map(|n| n.trim_start_matches('/').to_string()))
            })
            .map(|name| (name, MembershipEvent::Joined))
            .collect::<Vec<_>>();

        ret
    })
    .flat_map(futures::stream::iter);

    Box::pin(
        initial
            .chain(events)
            .map(|(k, v)| (TaglessMemberId::from_container_name(k), v))
            .inspect(|(member_id, event)| debug!(name: "membership_event", ?member_id, ?event)),
    )
}
