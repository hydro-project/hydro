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
use sinktools::demux_map_lazy::LazyDemuxSink;
use sinktools::lazy::{LazySink, LazySource};
use stageleft::{QuotedWithContext, q};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

use crate::location::dynamic::LocationId;
use crate::location::{MemberId, MembershipEvent};

pub fn deploy_containerized_o2o(target: &str, bind_addr: &str) -> (syn::Expr, syn::Expr) {
    (
        q!(LazySink::<_, _, _, bytes::Bytes>::new(move || Box::pin(
            async move {
                eprintln!("CONNECTING TO: {}", target);
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
            eprintln!("ACCEPTED FROM: {peer:?}");
            Result::<_, std::io::Error>::Ok(FramedRead::new(stream, LengthDelimitedCodec::new()))
        })))
        .splice_untyped_ctx(&()),
    )
}

pub fn deploy_containerized_o2m<Tag: Unpin>(port: u16, bind_addr: &str) -> (syn::Expr, syn::Expr) {
    (
        QuotedWithContext::<'static, LazyDemuxSink<MemberId<Tag>, _, _>, ()>::splice_untyped_ctx(
            q!(sinktools::demux_map_lazy::<_, _, _, _>(
                move |key: MemberId<_>| {
                    LazySink::<_, _, _, bytes::Bytes>::new(move || {
                        Box::pin(async move {
                            eprintln!("CONNECTING TO: {}:{}", key.get_container_name(), port);
                            let mut sink = FramedWrite::new(
                                TcpStream::connect(format!(
                                    "{}:{}",
                                    key.get_container_name(),
                                    port
                                ))
                                .await?,
                                LengthDelimitedCodec::new(),
                            );

                            // sink.send(bytes::Bytes::copy_from_slice(
                            //     bincode::serialize(&std::env::var("CONTAINER_NAME").unwrap())
                            //         .unwrap()
                            //         .as_slice(),
                            // ))
                            // .await?;

                            Result::<_, std::io::Error>::Ok(sink)
                        })
                    })
                }
            )),
            &(),
        ),
        q!(LazySource::new(move || Box::pin(async move {
            let listener = TcpListener::bind(bind_addr).await?;
            let (stream, peer) = listener.accept().await?;
            eprintln!("ACCEPTED FROM: {peer}");

            Result::<_, std::io::Error>::Ok(FramedRead::new(stream, LengthDelimitedCodec::new()))
        })))
        .splice_untyped_ctx(&()),
    )
}

pub fn deploy_containerized_m2o<'a, Tag>(
    target: &str,
    bind_addr: &'a str,
) -> (syn::Expr, syn::Expr) {
    (
        q!(LazySink::<_, _, _, bytes::Bytes>::new(move || {
            Box::pin(async move {
                eprintln!("CONNECTING TO: {}", target);

                let mut sink = FramedWrite::new(
                    TcpStream::connect(target).await?,
                    LengthDelimitedCodec::new(),
                );

                sink.send(bytes::Bytes::copy_from_slice(
                    bincode::serialize(&std::env::var("CONTAINER_NAME").unwrap())
                        .unwrap()
                        .as_slice(),
                ))
                .await?;

                Result::<_, std::io::Error>::Ok(sink)
            })
        }))
        .splice_untyped_ctx(&()),
        QuotedWithContext::<'a, LazySource<_, _, _, Result<(MemberId<Tag>, BytesMut), _>>, ()>::splice_untyped_ctx(
            q!(LazySource::new(move || Box::pin(async move {
                let listener = TcpListener::bind(bind_addr).await?;
                Result::<_, std::io::Error>::Ok(
                    futures::stream::unfold(listener, |listener| {
                        Box::pin(async move {
                            let (stream, peer) = listener.accept().await.ok()?;
                            let mut source = FramedRead::new(stream, LengthDelimitedCodec::new());
                            let from =
                                bincode::deserialize::<String>(&source.next().await?.ok()?[..])
                                    .ok()?;

                            eprintln!("ACCEPTING FROM: {} - {}", from, peer);

                            Some((
                                source.map(move |v| {
                                    v.map(|v| (MemberId::from_container_name(from.clone()), v))
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

pub fn deploy_containerized_e2o(bind_addr: &str) -> syn::Expr {
    q!(LazySource::new(move || Box::pin(async move {
        let listener = TcpListener::bind(bind_addr).await?;
        let (stream, peer) = listener.accept().await?;
        eprintln!("ACCEPTED FROM: {peer:?}");
        Result::<_, std::io::Error>::Ok(FramedRead::new(stream, LengthDelimitedCodec::new()))
    })))
    .splice_untyped_ctx(&())
}

pub fn deploy_containerized_o2e(target: &str) -> syn::Expr {
    q!(LazySink::<_, _, _, bytes::Bytes>::new(move || Box::pin(
        async move {
            eprintln!("CONNECTING TO: {}", target);

            Result::<_, std::io::Error>::Ok(FramedWrite::new(
                TcpStream::connect(target).await?,
                LengthDelimitedCodec::new(),
            ))
        }
    )))
    .splice_untyped_ctx(&())
}

pub fn cluster_membership_stream<'a>(
    location_id: &LocationId,
) -> impl QuotedWithContext<'a, Box<dyn Stream<Item = (MemberId<()>, MembershipEvent)> + Unpin>, ()>
{
    let raw_id = location_id.raw_id();

    q!(Box::new(self::docker_membership_stream(raw_id))
        as Box<
            dyn Stream<Item = (MemberId<()>, MembershipEvent)> + Unpin,
        >)
}

fn docker_membership_stream(
    location_id: usize,
) -> impl Stream<Item = (MemberId<()>, MembershipEvent)> + Unpin {
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

    let events = docker.events(event_options).filter_map(move |event| {
        std::future::ready(event.ok().and_then(|e| {
            let name = e
                .actor
                .and_then(|a| a.attributes.and_then(|attrs| attrs.get("name").cloned()))?;

            if name.contains(format!("loc-{location_id}-").as_str()) {
                match e.action.as_deref() {
                    Some("start") => Some((name.clone(), MembershipEvent::Joined)),
                    Some("die") => Some((name, MembershipEvent::Left)),
                    _ => None,
                }
            } else {
                None
            }
        }))
    });

    let initial = once(async move {
        let mut filters = HashMap::new();
        filters.insert("name".to_string(), vec![format!("loc-{location_id}-")]);
        let options = Some(ListContainersOptions {
            // all: true,
            filters,
            ..Default::default()
        });
        docker
            .list_containers(options)
            .await
            .unwrap()
            .into_iter()
            .filter_map(|c| {
                c.names
                    .and_then(|names| names.first().map(|n| n.trim_start_matches('/').to_string()))
            })
            .map(|name| (name, MembershipEvent::Joined))
            .collect::<Vec<_>>()
    })
    .flat_map(futures::stream::iter);

    Box::pin(
        initial
            .chain(events)
            .map(|(k, v)| (MemberId::from_container_name(k), v))
            .inspect(|v| eprintln!("docker membership event: {:?}", v)),
    )
}
