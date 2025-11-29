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
use tracing::{Subscriber, debug, instrument};
use tracing_subscriber::fmt::{FormatEvent, FormatFields, FormattedFields};
use tracing_subscriber::registry::LookupSpan;

use crate::location::dynamic::LocationId;
use crate::location::member_id::TaglessMemberId;
use crate::location::{MemberId, MembershipEvent};
use crate::staging_util::Invariant;

pub fn deploy_containerized_o2o(target: &str, bind_addr: &str) -> (syn::Expr, syn::Expr) {
    (
        q!(LazySink::<_, _, _, bytes::Bytes>::new(move || Box::pin(
            async move {
                debug!("HydroDeploy: connecting to: {}", target);
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
            debug!("HydroDeploy: accepting from: {peer}");
            Result::<_, std::io::Error>::Ok(FramedRead::new(stream, LengthDelimitedCodec::new()))
        })))
        .splice_untyped_ctx(&()),
    )
}

pub fn deploy_containerized_o2m(port: u16) -> (syn::Expr, syn::Expr) {
    (
        QuotedWithContext::<'static, LazyDemuxSink<TaglessMemberId, _, _>, ()>::splice_untyped_ctx(
            q!(sinktools::demux_map_lazy::<_, _, _, _>(
                move |key: TaglessMemberId| {
                    LazySink::<_, _, _, bytes::Bytes>::new(move || {
                        Box::pin(async move {
                            debug!(
                                "HydroDeploy: connecting to: {}:{}",
                                key.get_container_name(),
                                port
                            );
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
            debug!("HydroDeploy: listening on: {}", bind_addr);
            let listener = TcpListener::bind(bind_addr).await?;
            let (stream, peer) = listener.accept().await?;
            debug!("HydroDeploy: accepting from: {peer}");

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
                debug!("HydroDeploy: connecting to: {}", target);

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
                debug!("HydroDeploy: listening on: {}", bind_addr);
                let listener = TcpListener::bind(bind_addr).await?;
                Result::<_, std::io::Error>::Ok(
                    futures::stream::unfold(listener, |listener| {
                        Box::pin(async move {
                            let (stream, peer) = listener.accept().await.ok()?;
                            let mut source = FramedRead::new(stream, LengthDelimitedCodec::new());
                            let from =
                                bincode::deserialize::<String>(&source.next().await?.ok()?[..])
                                    .ok()?;

                            debug!("HydroDeploy: accepting from: {}:{}", peer, from);

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
                move |key: TaglessMemberId| {
                    LazySink::<_, _, _, bytes::Bytes>::new(move || {
                        Box::pin(async move {
                            debug!("HydroDeploy: connecting to: {}:{}", key.get_container_name(), port);
                            let mut sink = FramedWrite::new(
                                TcpStream::connect(format!(
                                    "{}:{}",
                                    key.get_container_name(),
                                    port
                                ))
                                .await?,
                                LengthDelimitedCodec::new(),
                            );

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
                debug!("HydroDeploy: listening on: {}", bind_addr);
                let listener = TcpListener::bind(bind_addr).await?;

                Result::<_, std::io::Error>::Ok(
                    futures::stream::unfold(listener, |listener| {
                        Box::pin(async move {
                            let (stream, peer) = listener.accept().await.ok()?;
                            let mut source = FramedRead::new(stream, LengthDelimitedCodec::new());
                            let from =
                                bincode::deserialize::<String>(&source.next().await?.ok()?[..])
                                    .ok()?;

                            debug!("HydroDeploy: accepting from: {}:{}", peer, from);

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

// pub fn deploy_containerized_external_sink_source(port: u16) -> syn::Expr {
//     q!(EagerLazySinkSource::<
//         _,
//         FramedRead<OwnedReadHalf, LengthDelimitedCodec>,
//         FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>,
//         bytes::Bytes,
//         std::io::Error,
//     >::new(async move {
//         let bind_addr = format!("0.0.0.0:{}", port);
//         debug!("HydroDeploy: external listening on: {}", bind_addr);
//         let listener = TcpListener::bind(bind_addr).await?;
//         let (stream, peer) = listener.accept().await?;
//         debug!("HydroDeploy: external accepting from: {peer:?}");
//         let (rx, tx) = stream.into_split();

//         let fr = FramedRead::new(rx, LengthDelimitedCodec::new());
//         let fw = FramedWrite::new(tx, LengthDelimitedCodec::new());

//         Result::<_, std::io::Error>::Ok((fr, fw))
//     }))
//     .splice_untyped_ctx(&())
// }

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

pub fn deploy_containerized_external_sink_source_ident(
    socket_ident: syn::Ident,
    debug_tag: String,
) -> syn::Expr {
    let socket_ident = SocketIdent { socket_ident };
    let debug_tag = Box::leak(Box::new(debug_tag)).as_str(); // TODO: remove when q! supports strings

    q!(LazySinkSource::<
        _,
        FramedRead<OwnedReadHalf, LengthDelimitedCodec>,
        FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>,
        bytes::Bytes,
        std::io::Error,
    >::new(async move {
        let (stream, peer) = socket_ident.accept().await?;
        debug!("HydroDeploy: external accepting from: {peer:?}");
        let (rx, tx) = stream.into_split();

        let fr = FramedRead::new(rx, LengthDelimitedCodec::new());
        let fw = FramedWrite::new(tx, LengthDelimitedCodec::new());

        Result::<_, std::io::Error>::Ok((fr, fw))
    },))
    .splice_untyped_ctx(&())
}

// pub fn deploy_containerized_e2o(port: u16) -> syn::Expr {
//     q!(LazySource::new(move || Box::pin(async move {
//         let bind_addr = format!("0.0.0.0:{}", port);
//         debug!("HydroDeploy: e2o external listening on: {}", bind_addr);
//         let listener = TcpListener::bind(bind_addr).await?;
//         let (stream, peer) = listener.accept().await?;
//         debug!("HydroDeploy: e2o external accepting from: {peer:?}");
//         Result::<_, std::io::Error>::Ok(FramedRead::new(stream, LengthDelimitedCodec::new()))
//     })))
//     .splice_untyped_ctx(&())
// }

// pub fn deploy_containerized_o2e(port: u16) -> syn::Expr {
//     // q!(LazySink::<_, _, _, bytes::Bytes>::new(move || Box::pin(
//     //     async move {
//     //         let bind_addr = format!("0.0.0.0:{}", port);
//     //         debug!("HydroDeploy: external listening on: {bind_addr}");
//     //         let listener = TcpListener::bind(bind_addr).await?;
//     //         let (stream, peer) = listener.accept().await?;
//     //         debug!("HydroDeploy: external accepting from: {peer:?}");

//     //         Result::<_, std::io::Error>::Ok(FramedWrite::new(stream, LengthDelimitedCodec::new()))
//     //     }
//     // )))
//     q!(EagerLazySink::new(move || Box::pin(async move {
//         let bind_addr = format!("0.0.0.0:{}", port);
//         debug!("HydroDeploy: o2e external listening on: {bind_addr}");
//         let listener = TcpListener::bind(bind_addr).await?;
//         let (stream, peer) = listener.accept().await?;
//         debug!("HydroDeploy: o2e external accepting from: {peer:?}");

//         Result::<_, std::io::Error>::Ok(Box::new(FramedWrite::new(
//             stream,
//             LengthDelimitedCodec::new(),
//         ))
//             as Box<dyn Sink<bytes::Bytes, Error = std::io::Error> + Unpin>)
//     })))
//     .splice_untyped_ctx(&())
// }

pub fn cluster_ids<'a>() -> impl QuotedWithContext<'a, &'a [TaglessMemberId], ()> + Clone {
    // This is a dummy piece of code, since clusters are dynamic when containerized.
    q!(Box::leak(Box::new([TaglessMemberId::from_container_name(
        "INVALID CONTAINER NAME cluster_ids"
    )]))
    .as_slice())
}

pub fn cluster_self_id<'a>() -> impl QuotedWithContext<'a, TaglessMemberId, ()> + Clone + 'a {
    // q!(MemberId::from_container_name(
    //     std::env::var("CONTAINER_NAME").unwrap()
    // ))

    q!(TaglessMemberId::from_container_name("fake container name"))
}

pub fn cluster_membership_stream<'a>(
    deployment_instance: String,
    location_id: &LocationId,
) -> impl QuotedWithContext<'a, Box<dyn Stream<Item = (TaglessMemberId, MembershipEvent)> + Unpin>, ()>
{
    let raw_id = location_id.raw_id();

    // TODO: can remove once String support lands in q!
    let deployment_instance = Box::leak(Box::new(deployment_instance.clone())).as_str();

    q!(
        Box::new(self::docker_membership_stream2(deployment_instance, raw_id))
            as Box<dyn Stream<Item = (TaglessMemberId, MembershipEvent)> + Unpin>
    )
}

fn docker_membership_stream(
    deployment_instance: &str,
    location_id: usize,
) -> impl Stream<Item = (TaglessMemberId, MembershipEvent)> + Unpin {
    debug!("DockerDeploy: docker_membership_stream: {deployment_instance} - on {location_id}");

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

    debug!(
        "DockerDeploy: listening for container events: {deployment_instance} - on {location_id}"
    );

    let events = docker.events(event_options).filter_map(move |event| {
        debug!("DockerDeploy: docker_membership_stream: events/filter_map");

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
    });

    let initial = once(async move {
        debug!("DockerDeploy: docker_membership_stream: initial");

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
            .inspect(|c: &bollard::secret::ContainerSummary| {
                debug!("docker initial container: {:?}", c.names)
            })
            .filter_map(|c| {
                c.names
                    .and_then(|names| names.first().map(|n| n.trim_start_matches('/').to_string()))
            })
            .map(|name| (name, MembershipEvent::Joined))
            .collect::<Vec<_>>();

        debug!(
            "DockerDeploy: docker_membership_stream: initial-end: {:?}",
            ret
        );

        ret
    })
    .flat_map(futures::stream::iter);

    Box::pin(
        initial
            .chain(events)
            .map(|(k, v)| (TaglessMemberId::from_container_name(k), v))
            .inspect(|v| debug!("docker membership event: {:?}", v)),
    )
}

fn docker_membership_stream2(
    deployment_instance: &str,
    location_id: usize,
) -> DockerMembershipStream {
    DockerMembershipStream::new(deployment_instance.to_string(), location_id)
}

pub struct DockerMembershipStream {
    state: State,
}

enum State {
    Init {
        deployment_instance: String,
        location_id: usize,
    },
    FetchingInitial {
        #[expect(clippy::type_complexity, reason = "internal code")]
        fut: Pin<Box<dyn Future<Output = Vec<(String, MembershipEvent)>> + Send>>,
        docker: Arc<bollard::Docker>,
        deployment_instance: String,
        location_id: usize,
    },
    StreamingInitial {
        initial: Vec<(String, MembershipEvent)>,
        idx: usize,
        docker: Arc<bollard::Docker>,
        deployment_instance: String,
        location_id: usize,
    },
    StreamingEvents {
        events: Pin<
            Box<
                dyn Stream<Item = Result<bollard::models::EventMessage, bollard::errors::Error>>
                    + Send,
            >,
        >,
        deployment_instance: String,
        location_id: usize,
    },
}

impl DockerMembershipStream {
    pub fn new(deployment_instance: String, location_id: usize) -> Self {
        debug!("DockerDeploy: docker_membership_stream2: {deployment_instance} - on {location_id}");
        Self {
            state: State::Init {
                deployment_instance,
                location_id,
            },
        }
    }
}

pub fn initialize_tracing() {
    use tracing::subscriber::set_global_default;
    use tracing_subscriber::filter::EnvFilter;
    use tracing_subscriber::fmt::format::FmtSpan;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{Layer, fmt, registry};

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("trace")); // Default to 'info' if RUST_LOG is not set

    set_global_default(
        registry().with(
            fmt::layer()
                .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                .event_format(crate::telemetry::Formatter)
                .with_filter(filter),
        ),
    )
    .unwrap();

    tracing::debug!("Tracing Initialized");
}

impl Stream for DockerMembershipStream {
    type Item = (TaglessMemberId, MembershipEvent);

    #[instrument(skip_all, level = "trace")]
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        debug!("DockerDeploy: docker_membership_stream2: poll_next");

        loop {
            match &mut self.state {
                State::Init {
                    deployment_instance,
                    location_id,
                } => {
                    debug!("DockerDeploy: docker_membership_stream2: Init");

                    use bollard::Docker;
                    use bollard::container::ListContainersOptions;

                    let docker = Arc::new(
                        Docker::connect_with_local_defaults()
                            .unwrap()
                            .with_timeout(Duration::from_secs(1)),
                    );

                    let deployment_instance_clone = deployment_instance.clone();
                    let location_id_clone = *location_id;
                    let docker_clone = docker.clone();

                    let fut = Box::pin(async move {
                        let mut filters = HashMap::new();
                        filters.insert(
                            "name".to_string(),
                            vec![format!("{deployment_instance_clone}-{location_id_clone}")],
                        );
                        let options = Some(ListContainersOptions {
                            filters,
                            ..Default::default()
                        });

                        docker_clone
                            .list_containers(options)
                            .await
                            .unwrap()
                            .into_iter()
                            .inspect(|c: &bollard::secret::ContainerSummary| {
                                debug!("docker initial container: {:?}", c.names)
                            })
                            .filter_map(|c| {
                                c.names.and_then(|names| {
                                    names.first().map(|n| n.trim_start_matches('/').to_string())
                                })
                            })
                            .map(|name| (name, MembershipEvent::Joined))
                            .collect::<Vec<_>>()
                    });

                    self.state = State::FetchingInitial {
                        fut,
                        docker,
                        deployment_instance: deployment_instance.clone(),
                        location_id: *location_id,
                    };
                }
                State::FetchingInitial {
                    fut,
                    docker,
                    deployment_instance,
                    location_id,
                } => {
                    debug!("DockerDeploy: docker_membership_stream2: FetchingInitial");

                    match fut.poll_unpin(cx) {
                        Poll::Ready(initial) => {
                            debug!(
                                "DockerDeploy: docker_membership_stream2: initial-end: {:?}",
                                initial
                            );
                            self.state = State::StreamingInitial {
                                initial,
                                idx: 0,
                                docker: docker.clone(),
                                deployment_instance: deployment_instance.clone(),
                                location_id: *location_id,
                            };
                        }
                        Poll::Pending => return Poll::Pending,
                    }
                }
                State::StreamingInitial {
                    initial,
                    idx,
                    docker,
                    deployment_instance,
                    location_id,
                } => {
                    debug!("DockerDeploy: docker_membership_stream2: StreamingInitial");

                    if *idx < initial.len() {
                        let (name, event) = initial[*idx].clone();
                        *idx += 1;
                        debug!(
                            "DockerDeploy: docker_membership_stream2: initial-emitting: {name} {event:?}"
                        );

                        return Poll::Ready(Some((
                            TaglessMemberId::from_container_name(name),
                            event,
                        )));
                    } else {
                        use bollard::system::EventsOptions;
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

                        debug!(
                            "DockerDeploy: listening for container events: {deployment_instance} - on {location_id}"
                        );
                        let events = docker.events(event_options);

                        self.state = State::StreamingEvents {
                            events: Box::pin(events),
                            deployment_instance: deployment_instance.clone(),
                            location_id: *location_id,
                        };
                    }
                }
                State::StreamingEvents {
                    events,
                    deployment_instance,
                    location_id,
                } => {
                    debug!("DockerDeploy: docker_membership_stream2: StreamingEvents");

                    match events.poll_next_unpin(cx) {
                        Poll::Ready(Some(Ok(e))) => {
                            if let Some(name) = e.actor.and_then(|a| {
                                a.attributes.and_then(|attrs| attrs.get("name").cloned())
                            }) && name.contains(&format!("{deployment_instance}-{location_id}"))
                            {
                                let event = match e.action.as_deref() {
                                    Some("start") => Some(MembershipEvent::Joined),
                                    Some("die") => Some(MembershipEvent::Left),
                                    _ => None,
                                };
                                if let Some(event) = event {
                                    debug!(
                                        "DockerDeploy: docker_membership_stream2: event-emitting: {name} {event:?}"
                                    );

                                    let result =
                                        (TaglessMemberId::from_container_name(name), event);

                                    return Poll::Ready(Some(result));
                                }
                            }
                        }
                        Poll::Ready(Some(Err(_))) => {}
                        Poll::Ready(None) => return Poll::Ready(None),
                        Poll::Pending => return Poll::Pending,
                    }
                }
            }
        }
    }
}

impl Unpin for DockerMembershipStream {}
