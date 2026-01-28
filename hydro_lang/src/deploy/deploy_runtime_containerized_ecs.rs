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
use tracing::{Instrument, debug, error, instrument, span, trace, trace_span};

use crate::location::dynamic::LocationId;
use crate::location::member_id::TaglessMemberId;
use crate::location::{LocationKey, MemberId, MembershipEvent};

pub fn deploy_containerized_o2o(target_container: &str, bind_port: u16) -> (syn::Expr, syn::Expr) {
    (
        q!(LazySink::<_, _, _, bytes::Bytes>::new(move || Box::pin(
            async move {
                let target_container = target_container;
                let ip = self::resolve_container_ip(target_container).await;
                let target = format!("{}:{}", ip, bind_port);
                debug!(name: "connecting", %target, %target_container);

                let stream = TcpStream::connect(&target).await?;

                Result::<_, std::io::Error>::Ok(FramedWrite::new(
                    stream,
                    LengthDelimitedCodec::new(),
                ))
            }
        )))
        .splice_untyped_ctx(&()),
        q!(LazySource::new(move || Box::pin(async move {
            let bind_addr = format!("0.0.0.0:{}", bind_port);
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
                            let container_name = key.get_container_name();
                            let ip = self::resolve_container_ip(&container_name).await;
                            let target = format!("{}:{}", ip, port);
                            debug!(name: "connecting", %target, %container_name);

                            let stream = TcpStream::connect(&target).await?;

                            let sink = FramedWrite::new(stream, LengthDelimitedCodec::new());
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

pub fn deploy_containerized_m2o(port: u16, target_container: &str) -> (syn::Expr, syn::Expr) {
    (
        q!(LazySink::<_, _, _, bytes::Bytes>::new(move || {
            Box::pin(async move {
                let target_container = target_container;
                let ip = self::resolve_container_ip(target_container).await;
                let target = format!("{}:{}", ip, port);
                debug!(name: "connecting", %target, %target_container);

                let stream = TcpStream::connect(&target).await?;

                let mut sink = FramedWrite::new(stream, LengthDelimitedCodec::new());

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
                            let container_name = key.get_container_name();
                            let ip = self::resolve_container_ip(&container_name).await;
                            let target = format!("{}:{}", ip, port);
                            debug!(name: "connecting", %target, %container_name);

                            let stream = TcpStream::connect(&target).await?;

                            let mut sink = FramedWrite::new(stream, LengthDelimitedCodec::new());
                            debug!(name: "connected", %target);

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

pub fn deploy_containerized_external_sink_source_ident(
    bind_addr: String,
    socket_ident: syn::Ident,
) -> syn::Expr {
    let socket_ident = SocketIdent { socket_ident };

    q!(LazySinkSource::<
        _,
        FramedRead<OwnedReadHalf, LengthDelimitedCodec>,
        FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>,
        bytes::Bytes,
        // Result<bytes::BytesMut, std::io::Error>,
        std::io::Error,
    >::new(async move {
        let span = span!(tracing::Level::TRACE, "lazy_sink_source");
        let guard = span.enter();
        let bind_addr = bind_addr;
        trace!(name: "attempting to accept from external", %bind_addr);
        std::mem::drop(guard);
        let (stream, peer) = socket_ident.accept().instrument(span.clone()).await?;
        let guard = span.enter();

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
    let location_key = location_id.key();

    q!(Box::new(self::ecs_membership_stream(
        std::env::var("DEPLOYMENT_INSTANCE").unwrap(),
        location_key
    ))
        as Box<
            dyn Stream<Item = (TaglessMemberId, MembershipEvent)> + Unpin,
        >)
}

#[instrument(skip_all, fields(%deployment_instance, %location_key))]
fn ecs_membership_stream(
    deployment_instance: String,
    location_key: LocationKey,
) -> impl Stream<Item = (TaglessMemberId, MembershipEvent)> + Unpin {
    use std::collections::HashSet;

    use futures::stream::{StreamExt, once};

    let ecs_poller_span = trace_span!("ecs_poller");

    let task_definition_arn_parser =
        regex::Regex::new(r#"arn:aws:ecs:(?<region>.*):(?<account_id>.*):task-definition\/(?<container_id>hy-(?<type>[^-]+)-(?<image_id>[^-]+)-(?<deployment_id>[^-]+)-(?<location_id>[0-9]+)-(?<instance_id>.*)):.*"#).unwrap();

    let poll_stream = futures::stream::unfold(
        (HashSet::<String>::new(), deployment_instance, location_key),
        move |(known_tasks, deployment_instance, location_key)| {
            let task_definition_arn_parser = task_definition_arn_parser.clone();

            async move {
                trace!(name: "polling_ecs", known_task_count = known_tasks.len());

                let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
                let ecs_client = aws_sdk_ecs::Client::new(&config);

                let cluster_name = format!("hydro-{}", deployment_instance);
                trace!(name: "querying_tasks", %cluster_name, %location_key);

                let tasks = match ecs_client
                    .list_tasks()
                    .cluster(&cluster_name)
                    .send()
                    .await
                {
                    Ok(tasks) => tasks,
                    Err(e) => {
                        trace!(name: "list_tasks_error", error = %e);
                        tokio::time::sleep(Duration::from_secs(2)).await;
                        return Some((Vec::new(), (known_tasks, deployment_instance, location_key)));
                    }
                };

                let task_arns: Vec<String> = tasks.task_arns().iter().map(|s| s.to_string()).collect();
                trace!(name: "tasks_found", task_count = task_arns.len());

                let mut events = Vec::new();
                let mut current_tasks = HashSet::<String>::new();

                if !task_arns.is_empty() {
                    let task_details = match ecs_client
                        .describe_tasks()
                        .cluster(&cluster_name)
                        .set_tasks(Some(task_arns.clone()))
                        .send()
                        .await
                    {
                        Ok(details) => details,
                        Err(e) => {
                            trace!(name: "describe_tasks_error", error = %e);
                            tokio::time::sleep(Duration::from_secs(2)).await;
                            return Some((Vec::new(), (known_tasks, deployment_instance, location_key)));
                        }
                    };

                    for task in task_details.tasks() {
                        let Some(last_status) = task.last_status() else {
                            trace!(name: "task_status_missing", ?task);
                            continue;
                        };

                        trace!(name: "task_status", %last_status, ?task);

                        if last_status != "RUNNING" {
                            trace!(name: "task_not_running", %last_status, ?task);
                            continue;
                        }

                        let Some(task_def_arn) = task.task_definition_arn() else {
                            trace!(name: "task_def_arn_missing", ?task);
                            continue;
                        };

                        let Some(captures) = task_definition_arn_parser.captures(task_def_arn) else {
                            trace!(name: "task_def_arn_parse_error", %task_def_arn, ?task);
                            continue;
                        };

                        let Some(container_id) = captures.name("container_id") else {
                            trace!(name: "container_id_missing", %task_def_arn, ?task);
                            continue;
                        };
                        let container_id = container_id.as_str();

                        let Some(task_location_key) = captures.name("location_key") else {
                            trace!(name: "location_key_missing", %task_def_arn, ?task);
                            continue;
                        };
                        let task_location_key: LocationKey = match task_location_key.as_str().parse() {
                            Ok(id) => id,
                            Err(_) => {
                                trace!(name: "location_key_parse_error", %task_def_arn, ?task);
                                continue;
                            }
                        };

                        // Filter by location_id - only include tasks for this specific cluster
                        if task_location_key != location_key {
                            trace!(name: "location_id_mismatch", %task_location_key, %location_key, %container_id);
                            continue;
                        }

                        // Use container_id directly (not DNS name)
                        trace!(name: "running_task", %container_id);
                        current_tasks.insert(container_id.to_string());
                        if !known_tasks.contains(container_id) {
                            trace!(name: "container_joined", %container_id);
                            events.push((container_id.to_string(), MembershipEvent::Joined));
                        }
                    }
                }

                #[expect(
                    clippy::disallowed_methods,
                    reason = "nondeterministic iteration order, container events are not deterministically ordered"
                )]
                for container_id in known_tasks.iter() {
                    if !current_tasks.contains(container_id) {
                        trace!(name: "container_left", %container_id);
                        events.push((container_id.to_owned(), MembershipEvent::Left));
                    }
                }

                trace!(name: "poll_complete", event_count = events.len(), current_task_count = current_tasks.len());
                tokio::time::sleep(Duration::from_secs(2)).await;

                Some((events, (current_tasks, deployment_instance, location_key)))
            }.instrument(ecs_poller_span.clone())
        }
    )
    .flat_map(futures::stream::iter);

    Box::pin(
        poll_stream
            .map(|(k, v)| (TaglessMemberId::from_container_name(k), v))
            .inspect(|(member_id, event)| trace!(name: "membership_event", ?member_id, ?event)),
    )
}

/// Resolve a container name to its private IP address via ECS API
async fn resolve_container_ip(container_name: &str) -> String {
    let deployment_instance = std::env::var("DEPLOYMENT_INSTANCE").unwrap();
    let cluster_name = format!("hydro-{}", deployment_instance);

    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let ecs_client = aws_sdk_ecs::Client::new(&config);

    loop {
        let tasks = match ecs_client
            .list_tasks()
            .cluster(&cluster_name)
            .family(container_name)
            .send()
            .await
        {
            Ok(t) => t,
            Err(e) => {
                trace!(name: "resolve_ip_list_error", %container_name, error = %e);
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };

        let Some(task_arn) = tasks.task_arns().first() else {
            trace!(name: "resolve_ip_no_task", %container_name);
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        };

        let task_details = match ecs_client
            .describe_tasks()
            .cluster(&cluster_name)
            .tasks(task_arn)
            .send()
            .await
        {
            Ok(d) => d,
            Err(e) => {
                trace!(name: "resolve_ip_describe_error", %container_name, error = %e);
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };

        if let Some(task) = task_details.tasks().first() {
            // Get private IP from task's network attachment
            if let Some(ip) = task
                .attachments()
                .iter()
                .flat_map(|a| a.details())
                .find(|d| d.name() == Some("privateIPv4Address"))
                .and_then(|d| d.value())
            {
                trace!(name: "resolved_ip", %container_name, %ip);
                return ip.to_string();
            }
        }

        trace!(name: "resolve_ip_no_ip", %container_name);
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
