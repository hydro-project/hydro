//! Helper utilities for the Hydroflow surface syntax.

mod udp;
#[cfg(not(target_arch = "wasm32"))]
pub use udp::*;

mod tcp;
#[cfg(not(target_arch = "wasm32"))]
pub use tcp::*;

#[cfg(unix)]
mod socket;
#[cfg(unix)]
pub use socket::*;

#[cfg(feature = "cli_integration")]
pub mod cli;

use std::net::SocketAddr;
use std::task::{Context, Poll};
use std::time::Duration;

use bincode;
use futures::{Sink, SinkExt, Stream};
use serde::{Deserialize, Serialize};

pub fn unbounded_channel<T>() -> (
    tokio::sync::mpsc::UnboundedSender<T>,
    tokio_stream::wrappers::UnboundedReceiverStream<T>,
) {
    let (send, recv) = tokio::sync::mpsc::unbounded_channel();
    let recv = tokio_stream::wrappers::UnboundedReceiverStream::new(recv);
    (send, recv)
}

pub fn ready_iter<S>(stream: S) -> impl Iterator<Item = S::Item>
where
    S: Stream,
{
    let mut stream = Box::pin(stream);
    std::iter::from_fn(move || {
        match stream
            .as_mut()
            .poll_next(&mut Context::from_waker(futures::task::noop_waker_ref()))
        {
            Poll::Ready(opt) => opt,
            Poll::Pending => None,
        }
    })
}

/// Collects the immediately available items from the `Stream` into a `FromIterator` collection.
///
/// This consumes the stream, use [`futures::StreamExt::by_ref()`] (or just `&mut ...`) if you want
/// to retain ownership of your stream.
pub fn collect_ready<C, S>(stream: S) -> C
where
    C: FromIterator<S::Item>,
    S: Stream,
{
    assert!(tokio::runtime::Handle::try_current().is_err(), "Calling `collect_ready` from an async runtime may cause incorrect results, use `collect_ready_async` instead.");
    ready_iter(stream).collect()
}

/// Collects the immediately available items from the `Stream` into a collection (`Default` + `Extend`).
///
/// This consumes the stream, use [`futures::StreamExt::by_ref()`] (or just `&mut ...`) if you want
/// to retain ownership of your stream.
pub async fn collect_ready_async<C, S>(stream: S) -> C
where
    C: Default + Extend<S::Item>,
    S: Stream,
{
    let any = std::cell::Cell::new(true);
    let mut unfused_iter = ready_iter(stream).inspect(|_| any.set(true));
    let mut out = C::default();
    while any.replace(false) {
        out.extend(unfused_iter.by_ref());
        // Tokio unbounded channel returns items in lenght-128 chunks, so we have to be careful
        // that everything gets returned. That is why we yield here and loop.
        tokio::task::yield_now().await;
    }
    out
}

pub fn serialize_to_bytes<T>(msg: T) -> bytes::Bytes
where
    T: Serialize + for<'a> Deserialize<'a> + Clone,
{
    bytes::Bytes::from(bincode::serialize(&msg).unwrap())
}

pub fn deserialize_from_bytes<T>(msg: impl AsRef<[u8]>) -> bincode::Result<T>
where
    T: Serialize + for<'a> Deserialize<'a> + Clone,
{
    bincode::deserialize(msg.as_ref())
}

pub fn ipv4_resolve(addr: &str) -> Result<SocketAddr, std::io::Error> {
    use std::net::ToSocketAddrs;
    let mut addrs = addr.to_socket_addrs()?;
    let result = addrs.find(|addr| addr.is_ipv4());
    match result {
        Some(addr) => Ok(addr),
        None => Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Unable to resolve IPv4 address",
        )),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn bind_udp_bytes(addr: SocketAddr) -> (UdpSink, UdpStream, SocketAddr) {
    let socket = tokio::net::UdpSocket::bind(addr).await.unwrap();
    udp_bytes(socket)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn bind_udp_lines(addr: SocketAddr) -> (UdpLinesSink, UdpLinesStream, SocketAddr) {
    let socket = tokio::net::UdpSocket::bind(addr).await.unwrap();
    udp_lines(socket)
}

// from addendum in
// https://stackoverflow.com/questions/56105305/how-to-sort-a-vec-of-structs-by-a-string-field
pub fn sort_unstable_by_key_hrtb<T, F, K>(slice: &mut [T], f: F)
where
    F: for<'a> Fn(&'a T) -> &'a K,
    K: Ord,
{
    slice.sort_unstable_by(|a, b| f(a).cmp(f(b)))
}

pub fn batched_sink<I: Send + 'static, S: Sink<I> + Send + 'static>(
    s: S,
    cap: usize,
    timeout: Duration,
) -> impl Sink<I, Error = ()> + Unpin {
    let (send, recv) = tokio::sync::mpsc::unbounded_channel::<I>();

    use futures::{stream, StreamExt};
    use futures_batch::ChunksTimeoutStreamExt;

    tokio::spawn(async move {
        let recv_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(recv);
        let mut batched_recv = recv_stream.chunks_timeout(cap, timeout);
        let mut s = Box::pin(s);

        while let Some(batch) = batched_recv.next().await {
            if s.send_all(&mut stream::iter(batch).map(|v| Ok(v)))
                .await
                .is_err()
            {
                panic!("Batched sink failed")
            }
        }
    });

    Box::pin(futures::sink::unfold(send, |send, item| async move {
        send.send(item).map(|_| send).map_err(|_| ())
    }))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn test_collect_ready() {
        let (send, mut recv) = unbounded_channel::<usize>();
        for x in 0..1000 {
            send.send(x).unwrap();
        }
        assert_eq!(1000, collect_ready::<Vec<_>, _>(&mut recv).len());
    }

    #[tokio::test]
    pub async fn test_collect_ready_async() {
        // Tokio unbounded channel returns items in 128 item long chunks, so we have to be careful that everything gets returned.
        let (send, mut recv) = unbounded_channel::<usize>();
        for x in 0..1000 {
            send.send(x).unwrap();
        }
        assert_eq!(
            1000,
            collect_ready_async::<Vec<_>, _>(&mut recv).await.len()
        );
    }
}
