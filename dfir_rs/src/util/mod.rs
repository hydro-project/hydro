#![warn(missing_docs)]
//! Helper utilities for the DFIR syntax.

pub mod clear;
#[cfg(feature = "dfir_macro")]
pub mod demux_enum;
pub mod monotonic_map;
pub mod multiset;
pub mod priority_stack;
pub mod slot_vec;
pub mod sparse_vec;
pub mod unsync;

pub mod simulation;

mod monotonic;
pub use monotonic::*;

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

#[cfg(feature = "deploy_integration")]
pub mod deploy;

use std::io::Read;
use std::net::SocketAddr;
use std::num::NonZeroUsize;
use std::process::{Child, ChildStdin, ChildStdout, Stdio};
use std::task::{Context, Poll};

use futures::Stream;
use serde::de::DeserializeOwned;
use serde::ser::Serialize;

/// Persit or delete tuples
pub enum Persistence<T> {
    /// Persist T values
    Persist(T),
    /// Delete all values that exactly match
    Delete(T),
}

/// Persit or delete key-value pairs
pub enum PersistenceKeyed<K, V> {
    /// Persist key-value pairs
    Persist(K, V),
    /// Delete all tuples that have the key K
    Delete(K),
}

/// Returns a channel as a (1) unbounded sender and (2) unbounded receiver `Stream` for use in DFIR.
pub fn unbounded_channel<T>() -> (
    tokio::sync::mpsc::UnboundedSender<T>,
    tokio_stream::wrappers::UnboundedReceiverStream<T>,
) {
    let (send, recv) = tokio::sync::mpsc::unbounded_channel();
    let recv = tokio_stream::wrappers::UnboundedReceiverStream::new(recv);
    (send, recv)
}

/// Returns an unsync channel as a (1) sender and (2) receiver `Stream` for use in DFIR.
pub fn unsync_channel<T>(
    capacity: Option<NonZeroUsize>,
) -> (unsync::mpsc::Sender<T>, unsync::mpsc::Receiver<T>) {
    unsync::mpsc::channel(capacity)
}

/// Returns an [`Iterator`] of any immediately available items from the [`Stream`].
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
    assert!(
        tokio::runtime::Handle::try_current().is_err(),
        "Calling `collect_ready` from an async runtime may cause incorrect results, use `collect_ready_async` instead."
    );
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
    use std::sync::atomic::Ordering;

    // Yield to let any background async tasks send to the stream.
    tokio::task::yield_now().await;

    let got_any_items = std::sync::atomic::AtomicBool::new(true);
    let mut unfused_iter =
        ready_iter(stream).inspect(|_| got_any_items.store(true, Ordering::Relaxed));
    let mut out = C::default();
    while got_any_items.swap(false, Ordering::Relaxed) {
        out.extend(unfused_iter.by_ref());
        // Tokio unbounded channel returns items in lenght-128 chunks, so we have to be careful
        // that everything gets returned. That is why we yield here and loop.
        tokio::task::yield_now().await;
    }
    out
}

/// Serialize a message to bytes using bincode.
pub fn serialize_to_bytes<T>(msg: T) -> bytes::Bytes
where
    T: Serialize,
{
    bytes::Bytes::from(bincode::serialize(&msg).unwrap())
}

/// Serialize a message from bytes using bincode.
pub fn deserialize_from_bytes<T>(msg: impl AsRef<[u8]>) -> bincode::Result<T>
where
    T: DeserializeOwned,
{
    bincode::deserialize(msg.as_ref())
}

/// Resolve the `ipv4` [`SocketAddr`] from an IP or hostname string.
pub fn ipv4_resolve(addr: &str) -> Result<SocketAddr, std::io::Error> {
    use std::net::ToSocketAddrs;
    let mut addrs = addr.to_socket_addrs()?;
    let result = addrs.find(|addr| addr.is_ipv4());
    match result {
        Some(addr) => Ok(addr),
        None => Err(std::io::Error::other("Unable to resolve IPv4 address")),
    }
}

/// Returns a length-delimited bytes `Sink`, `Stream`, and `SocketAddr` bound to the given address.
/// The input `addr` may have a port of `0`, the returned `SocketAddr` will have the chosen port.
#[cfg(not(target_arch = "wasm32"))]
pub async fn bind_udp_bytes(addr: SocketAddr) -> (UdpSink, UdpStream, SocketAddr) {
    let socket = tokio::net::UdpSocket::bind(addr).await.unwrap();
    udp_bytes(socket)
}

/// Returns a newline-delimited bytes `Sink`, `Stream`, and `SocketAddr` bound to the given address.
/// The input `addr` may have a port of `0`, the returned `SocketAddr` will have the chosen port.
#[cfg(not(target_arch = "wasm32"))]
pub async fn bind_udp_lines(addr: SocketAddr) -> (UdpLinesSink, UdpLinesStream, SocketAddr) {
    let socket = tokio::net::UdpSocket::bind(addr).await.unwrap();
    udp_lines(socket)
}

/// Returns a newline-delimited bytes `Sender`, `Receiver`, and `SocketAddr` bound to the given address.
///
/// The input `addr` may have a port of `0`, the returned `SocketAddr` will be the address of the newly bound endpoint.
/// The inbound connections can be used in full duplex mode. When a `(T, SocketAddr)` pair is fed to the `Sender`
/// returned by this function, the `SocketAddr` will be looked up against the currently existing connections.
/// If a match is found then the data will be sent on that connection. If no match is found then the data is silently dropped.
#[cfg(not(target_arch = "wasm32"))]
pub async fn bind_tcp_bytes(
    addr: SocketAddr,
) -> (
    unsync::mpsc::Sender<(bytes::Bytes, SocketAddr)>,
    unsync::mpsc::Receiver<Result<(bytes::BytesMut, SocketAddr), std::io::Error>>,
    SocketAddr,
) {
    bind_tcp(addr, tokio_util::codec::LengthDelimitedCodec::new())
        .await
        .unwrap()
}

/// This is the same thing as `bind_tcp_bytes` except instead of using a length-delimited encoding scheme it uses new lines to separate frames.
#[cfg(not(target_arch = "wasm32"))]
pub async fn bind_tcp_lines(
    addr: SocketAddr,
) -> (
    unsync::mpsc::Sender<(String, SocketAddr)>,
    unsync::mpsc::Receiver<Result<(String, SocketAddr), tokio_util::codec::LinesCodecError>>,
    SocketAddr,
) {
    bind_tcp(addr, tokio_util::codec::LinesCodec::new())
        .await
        .unwrap()
}

/// The inverse of [`bind_tcp_bytes`].
///
/// `(Bytes, SocketAddr)` pairs fed to the returned `Sender` will initiate new tcp connections to the specified `SocketAddr`.
/// These connections will be cached and reused, so that there will only be one connection per destination endpoint. When the endpoint sends data back it will be available via the returned `Receiver`
#[cfg(not(target_arch = "wasm32"))]
pub fn connect_tcp_bytes() -> (
    TcpFramedSink<bytes::Bytes>,
    TcpFramedStream<tokio_util::codec::LengthDelimitedCodec>,
) {
    connect_tcp(tokio_util::codec::LengthDelimitedCodec::new())
}

/// This is the same thing as `connect_tcp_bytes` except instead of using a length-delimited encoding scheme it uses new lines to separate frames.
#[cfg(not(target_arch = "wasm32"))]
pub fn connect_tcp_lines() -> (
    TcpFramedSink<String>,
    TcpFramedStream<tokio_util::codec::LinesCodec>,
) {
    connect_tcp(tokio_util::codec::LinesCodec::new())
}

/// Sort a slice using a key fn which returns references.
///
/// From addendum in
/// <https://stackoverflow.com/questions/56105305/how-to-sort-a-vec-of-structs-by-a-string-field>
pub fn sort_unstable_by_key_hrtb<T, F, K>(slice: &mut [T], f: F)
where
    F: for<'a> Fn(&'a T) -> &'a K,
    K: Ord,
{
    slice.sort_unstable_by(|a, b| f(a).cmp(f(b)))
}

/// Waits for a specific process output before returning.
///
/// When a child process is spawned often you want to wait until the child process is ready before
/// moving on. One way to do that synchronization is by waiting for the child process to output
/// something and match regex against that output. For example, you could wait until the child
/// process outputs "Client live!" which would indicate that it is ready to receive input now on
/// stdin.
pub fn wait_for_process_output(
    output_so_far: &mut String,
    output: &mut ChildStdout,
    wait_for: &str,
) {
    let re = regex::Regex::new(wait_for).unwrap();

    while !re.is_match(output_so_far) {
        println!("waiting: {}", output_so_far);
        let mut buffer = [0u8; 1024];
        let bytes_read = output.read(&mut buffer).unwrap();

        if bytes_read == 0 {
            panic!();
        }

        output_so_far.push_str(&String::from_utf8_lossy(&buffer[0..bytes_read]));

        println!("XXX {}", output_so_far);
    }
}

/// Terminates the inner [`Child`] process when dropped.
///
/// When a `Child` is dropped normally nothing happens but in unit tests you usually want to
/// terminate the child and wait for it to terminate. `DroppableChild` does that for us.
pub struct DroppableChild(Child);

impl Drop for DroppableChild {
    fn drop(&mut self) {
        #[cfg(target_family = "windows")]
        let _ = self.0.kill(); // Windows throws `PermissionDenied` if the process has already exited.
        #[cfg(not(target_family = "windows"))]
        self.0.kill().unwrap();

        self.0.wait().unwrap();
    }
}

/// Run a rust example as a test.
///
/// Rust examples are meant to be run by people and have a natural interface for that. This makes
/// unit testing them cumbersome. This function wraps calling cargo run and piping the stdin/stdout
/// of the example to easy to handle returned objects. The function also returns a `DroppableChild`
/// which will ensure that the child processes will be cleaned up appropriately.
pub fn run_cargo_example(test_name: &str, args: &str) -> (DroppableChild, ChildStdin, ChildStdout) {
    let mut server = if args.is_empty() {
        std::process::Command::new("cargo")
            .args(["run", "-p", "dfir_rs", "--example"])
            .arg(test_name)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap()
    } else {
        std::process::Command::new("cargo")
            .args(["run", "-p", "dfir_rs", "--example"])
            .arg(test_name)
            .arg("--")
            .args(args.split(' '))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap()
    };

    let stdin = server.stdin.take().unwrap();
    let stdout = server.stdout.take().unwrap();

    (DroppableChild(server), stdin, stdout)
}

/// Converts an iterator into a stream that emits `n` items at a time, yielding between each batch.
///
/// This is useful for breaking up a large iterator across several ticks: `source_iter(...)` always
/// releases all items in the first tick. However using `iter_batches_stream` with `source_stream(...)`
/// will cause `n` items to be released each tick. (Although more than that may be emitted if there
/// are loops in the stratum).
pub fn iter_batches_stream<I>(
    iter: I,
    n: usize,
) -> futures::stream::PollFn<impl FnMut(&mut Context<'_>) -> Poll<Option<I::Item>>>
where
    I: IntoIterator + Unpin,
{
    let mut count = 0;
    let mut iter = iter.into_iter();
    futures::stream::poll_fn(move |ctx| {
        count += 1;
        if n < count {
            count = 0;
            ctx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(iter.next())
        }
    })
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

    #[crate::test]
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
