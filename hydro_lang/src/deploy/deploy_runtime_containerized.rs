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

use bytes::BytesMut;
use futures::{FutureExt, Sink, SinkExt, Stream, StreamExt};
use sinktools::demux_map_lazy::LazyDemuxSink;
use sinktools::lazy::{LazySink, LazySource};
use stageleft::{QuotedWithContext, q};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

use crate::location::MemberId;

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
