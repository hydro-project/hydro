#![allow(
    unused,
    reason = "unused in trybuild but the __staged version is needed"
)]
#![allow(missing_docs, reason = "used internally")]

use std::future::Future;
use std::net::SocketAddr;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::{FutureExt, Sink, SinkExt, Stream, StreamExt};
use sinktools::lazy::{LazySink, LazySource};
use stageleft::{QuotedWithContext, q};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

pub fn deploy_containerized_o2o(target: &str, bind_addr: &str) -> (syn::Expr, syn::Expr) {
    (
        q!(LazySink::<_, _, bytes::Bytes, _>::new(move || Box::pin(
            async move {
                Result::<_, std::io::Error>::Ok(FramedWrite::new(
                    TcpStream::connect(target).await?,
                    LengthDelimitedCodec::new(),
                ))
            }
        )))
        .splice_untyped_ctx(&()),
        q!(LazySource::new(move || Box::pin(async move {
            let listener = TcpListener::bind(bind_addr).await?;
            let (stream, _) = listener.accept().await?;
            Result::<_, std::io::Error>::Ok(FramedRead::new(stream, LengthDelimitedCodec::new()))
        })))
        .splice_untyped_ctx(&()),
    )
}

pub fn deploy_containerized_e2o(bind_addr: &str) -> syn::Expr {
    q!(LazySource::new(move || Box::pin(async move {
        let listener = TcpListener::bind(bind_addr).await?;
        let (stream, _) = listener.accept().await?;
        Result::<_, std::io::Error>::Ok(FramedRead::new(stream, LengthDelimitedCodec::new()))
    })))
    .splice_untyped_ctx(&())
}

pub fn deploy_containerized_o2e(target: &str) -> syn::Expr {
    q!(LazySink::<_, _, bytes::Bytes, _>::new(move || Box::pin(
        async move {
            Result::<_, std::io::Error>::Ok(FramedWrite::new(
                TcpStream::connect(target).await?,
                LengthDelimitedCodec::new(),
            ))
        }
    )))
    .splice_untyped_ctx(&())
}
