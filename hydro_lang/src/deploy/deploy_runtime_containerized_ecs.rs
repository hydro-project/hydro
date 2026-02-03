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

pub fn deploy_containerized_o2o(
    target_task_family: &str,
    bind_port: u16,
) -> (syn::Expr, syn::Expr) {
    (
        q!(LazySink::<_, _, _, bytes::Bytes>::new(move || Box::pin(
            async move {
                let target_task_family = target_task_family;
                let task_id = self::resolve_task_family_to_task_id(target_task_family).await;
                let ip = self::resolve_task_ip(&task_id).await;
                let target = format!("{}:{}", ip, bind_port);
                debug!(name: "connecting", %target, %target_task_family, %task_id);

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
                            let task_id = key.get_container_name();
                            let ip = self::resolve_task_ip(&task_id).await;
                            let target = format!("{}:{}", ip, port);
                            debug!(name: "connecting", %target, %task_id);

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

pub fn deploy_containerized_m2o(port: u16, target_task_family: &str) -> (syn::Expr, syn::Expr) {
    (
        q!(LazySink::<_, _, _, bytes::Bytes>::new(move || {
            Box::pin(async move {
                let target_task_family = target_task_family;
                let target_task_id = self::resolve_task_family_to_task_id(target_task_family).await;
                let ip = self::resolve_task_ip(&target_task_id).await;
                let target = format!("{}:{}", ip, port);
                debug!(name: "connecting", %target, %target_task_family, %target_task_id);

                let stream = TcpStream::connect(&target).await?;

                let mut sink = FramedWrite::new(stream, LengthDelimitedCodec::new());

                let self_task_id = self::get_self_task_id();
                sink.send(bytes::Bytes::from(
                    bincode::serialize(&self_task_id).unwrap(),
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
                            let from_task_id =
                                bincode::deserialize::<String>(&source.next().await?.ok()?[..])
                                    .ok()?;

                            debug!(name: "accepting", endpoint = format!("{}:{}", peer, from_task_id));

                            Some((
                                source.map(move |v| {
                                    v.map(|v| (TaglessMemberId::from_container_name(from_task_id.clone()), v))
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
                            let task_id = key.get_container_name();
                            let ip = self::resolve_task_ip(&task_id).await;
                            let target = format!("{}:{}", ip, port);
                            debug!(name: "connecting", %target, %task_id);

                            let stream = TcpStream::connect(&target).await?;

                            let mut sink = FramedWrite::new(stream, LengthDelimitedCodec::new());
                            debug!(name: "connected", %target);

                            let self_task_id = self::get_self_task_id();
                            sink.send(bytes::Bytes::from(
                                bincode::serialize(&self_task_id).unwrap(),
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
                            let from_task_id =
                                bincode::deserialize::<String>(&source.next().await?.ok()?[..])
                                    .ok()?;

                            debug!(name: "accepting", endpoint = format!("{}:{}", peer, from_task_id));

                            Some((
                                source.map(move |v| {
                                    v.map(|v| (TaglessMemberId::from_container_name(from_task_id.clone()), v))
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
        self::get_self_task_id()
    ))
}

pub fn cluster_membership_stream<'a>(
    location_id: &LocationId,
) -> impl QuotedWithContext<'a, Box<dyn Stream<Item = (TaglessMemberId, MembershipEvent)> + Unpin>, ()>
{
    let location_key = location_id.key();

    q!(Box::new(self::ecs_membership_stream(
        std::env::var("CLUSTER_NAME").unwrap(),
        location_key
    ))
        as Box<
            dyn Stream<Item = (TaglessMemberId, MembershipEvent)> + Unpin,
        >)
}

#[instrument(skip_all, fields(%cluster_name, %location_key))]
fn ecs_membership_stream(
    cluster_name: String,
    location_key: LocationKey,
) -> impl Stream<Item = (TaglessMemberId, MembershipEvent)> + Unpin {
    use std::collections::HashSet;

    use futures::stream::{StreamExt, once};

    trace!(name: "ecs_membership_stream_created", %cluster_name, %location_key);

    let ecs_poller_span = trace_span!("ecs_poller");

    // Task family format: hy-{name_hint}-loc{idx}v{version}
    // Example: hy-p1-loc2v1
    let task_definition_arn_parser =
        regex::Regex::new(r#"arn:aws:ecs:(?<region>.*):(?<account_id>.*):task-definition\/(?<container_id>hy-(?<type>[^-]+)-loc(?<location_idx>[0-9]+)v(?<location_version>[0-9]+)(?:-(?<instance_id>.*))?):.*"#).unwrap();

    let poll_stream = futures::stream::unfold(
        (HashSet::<String>::new(), cluster_name, location_key),
        move |(known_tasks, cluster_name, location_key)| {
            let task_definition_arn_parser = task_definition_arn_parser.clone();

            async move {
                let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
                let ecs_client = aws_sdk_ecs::Client::new(&config);

                let tasks = match ecs_client.list_tasks().cluster(&cluster_name).send().await {
                    Ok(tasks) => tasks,
                    Err(e) => {
                        trace!(name: "list_tasks_error", error = %e);
                        tokio::time::sleep(Duration::from_secs(2)).await;
                        return Some((Vec::new(), (known_tasks, cluster_name, location_key)));
                    }
                };

                let task_arns: Vec<String> =
                    tasks.task_arns().iter().map(|s| s.to_string()).collect();

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
                            return Some((Vec::new(), (known_tasks, cluster_name, location_key)));
                        }
                    };

                    for task in task_details.tasks() {
                        let Some(last_status) = task.last_status() else {
                            continue;
                        };

                        if last_status != "RUNNING" {
                            continue;
                        }

                        let Some(task_def_arn) = task.task_definition_arn() else {
                            continue;
                        };

                        let Some(captures) = task_definition_arn_parser.captures(task_def_arn)
                        else {
                            continue;
                        };

                        let Some(location_idx) = captures.name("location_idx") else {
                            continue;
                        };
                        let Some(location_version) = captures.name("location_version") else {
                            continue;
                        };
                        // Reconstruct the location key string and parse it
                        let location_key_str =
                            format!("loc{}v{}", location_idx.as_str(), location_version.as_str());
                        let task_location_key: LocationKey = match location_key_str.parse() {
                            Ok(key) => key,
                            Err(_) => {
                                continue;
                            }
                        };

                        // Filter by location_id - only include tasks for this specific cluster
                        if task_location_key != location_key {
                            continue;
                        }

                        // Extract task ID from task ARN (last segment after final /)
                        // Task ARN format: arn:aws:ecs:region:account:task/cluster-name/task-id
                        let Some(task_arn) = task.task_arn() else {
                            continue;
                        };
                        let Some(task_id) = task_arn.rsplit('/').next() else {
                            continue;
                        };

                        // Use task_id as the member identifier
                        current_tasks.insert(task_id.to_owned());
                        if !known_tasks.contains(task_id) {
                            trace!(name: "task_joined", %task_id);
                            events.push((task_id.to_owned(), MembershipEvent::Joined));
                        }
                    }
                }

                #[expect(
                    clippy::disallowed_methods,
                    reason = "nondeterministic iteration order, container events are not deterministically ordered"
                )]
                for task_id in known_tasks.iter() {
                    if !current_tasks.contains(task_id) {
                        trace!(name: "task_left", %task_id);
                        events.push((task_id.to_owned(), MembershipEvent::Left));
                    }
                }

                tokio::time::sleep(Duration::from_secs(2)).await;

                Some((events, (current_tasks, cluster_name, location_key)))
            }
            .instrument(ecs_poller_span.clone())
        },
    )
    .flat_map(futures::stream::iter);

    Box::pin(
        poll_stream
            .map(|(k, v)| (TaglessMemberId::from_container_name(k), v))
            .inspect(|(member_id, event)| trace!(name: "membership_event", ?member_id, ?event)),
    )
}

/// Resolve a task ID to its private IP address via ECS API.
async fn resolve_task_ip(task_id: &str) -> String {
    let cluster_name = std::env::var("CLUSTER_NAME").unwrap();

    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let ecs_client = aws_sdk_ecs::Client::new(&config);

    loop {
        let tasks = match ecs_client.list_tasks().cluster(&cluster_name).send().await {
            Ok(t) => t,
            Err(e) => {
                trace!(name: "resolve_ip_list_error", %task_id, error = %e);
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };

        let task_arns: Vec<_> = tasks.task_arns().to_vec();
        if task_arns.is_empty() {
            trace!(name: "resolve_ip_no_tasks", %task_id);
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        }

        let task_details = match ecs_client
            .describe_tasks()
            .cluster(&cluster_name)
            .set_tasks(Some(task_arns))
            .send()
            .await
        {
            Ok(d) => d,
            Err(e) => {
                trace!(name: "resolve_ip_describe_error", %task_id, error = %e);
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };

        // Find the task with matching task ID
        for task in task_details.tasks() {
            let Some(task_arn) = task.task_arn() else {
                continue;
            };
            let current_task_id = task_arn.rsplit('/').next().unwrap_or_default();

            if current_task_id == task_id
                && let Some(ip) = task
                    .attachments()
                    .iter()
                    .flat_map(|a| a.details())
                    .find(|d| d.name() == Some("privateIPv4Address"))
                    .and_then(|d| d.value())
            {
                trace!(name: "resolved_ip", %task_id, %ip);
                return ip.to_owned();
            }
        }

        trace!(name: "resolve_ip_not_found", %task_id);
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

/// Resolve a task family name to its task ID via ECS API.
/// Used for process-to-process connections where the target is known by task family at compile time.
async fn resolve_task_family_to_task_id(task_family: &str) -> String {
    let cluster_name = std::env::var("CLUSTER_NAME").unwrap();

    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let ecs_client = aws_sdk_ecs::Client::new(&config);

    loop {
        let tasks = match ecs_client
            .list_tasks()
            .cluster(&cluster_name)
            .family(task_family)
            .send()
            .await
        {
            Ok(t) => t,
            Err(e) => {
                trace!(name: "resolve_family_list_error", %task_family, error = %e);
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };

        let Some(task_arn) = tasks.task_arns().first() else {
            trace!(name: "resolve_family_no_task", %task_family);
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        };

        // Extract task ID from ARN
        let task_id = task_arn.rsplit('/').next().unwrap_or_default();
        if !task_id.is_empty() {
            trace!(name: "resolved_task_id", %task_family, %task_id);
            return task_id.to_owned();
        }

        trace!(name: "resolve_family_invalid_arn", %task_family, %task_arn);
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

/// Get the current task's ID from ECS metadata.
fn get_self_task_id() -> String {
    let metadata_uri = std::env::var("ECS_CONTAINER_METADATA_URI_V4")
        .expect("ECS_CONTAINER_METADATA_URI_V4 not set - are we running in ECS?");
    metadata_uri
        .rsplit('/')
        .next()
        .expect("Invalid ECS metadata URI format")
        .to_owned()
}
