#![cfg(not(target_arch = "wasm32"))]

//! Surface syntax tests of asynchrony and networking.

use std::collections::{BTreeSet, HashSet};
use std::error::Error;
use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;

use bytes::Bytes;
use dfir_rs::scheduled::graph::Dfir;
use dfir_rs::util::{collect_ready_async, tcp_lines};
use dfir_rs::{assert_graphvis_snapshots, dfir_syntax, rassert, rassert_eq};
use multiplatform_test::multiplatform_test;
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::task::LocalSet;
use tokio_util::codec::{BytesCodec, FramedWrite, LinesCodec};
use tracing::Instrument;

#[multiplatform_test(dfir, env_tracing)]
pub async fn test_echo_udp() -> Result<(), Box<dyn Error>> {
    let local = LocalSet::new();

    let server_socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).await?;
    let server_addr = server_socket.local_addr()?;
    let server_addr: SocketAddr = (Ipv4Addr::LOCALHOST, server_addr.port()).into();

    // Server:
    let serv = local.spawn_local(async {
        let socket = server_socket;
        let (udp_send, udp_recv, _) = dfir_rs::util::udp_lines(socket);
        println!("Server live!");

        let (seen_send, seen_recv) = dfir_rs::util::unbounded_channel();

        let mut df: Dfir = dfir_syntax! {
            recv = source_stream(udp_recv)
                -> map(|r| r.unwrap())
                -> tee();
            // Echo
            recv[0] -> dest_sink(udp_send);
            // Testing
            recv[1] -> map(|(s, _addr)| s) -> for_each(|s| seen_send.send(s).unwrap());
        };

        tokio::select! {
            _ = df.run() => (),
            _ = tokio::time::sleep(Duration::from_secs(1)) => (),
        };

        let seen: HashSet<_> = collect_ready_async(seen_recv).await;
        rassert_eq!(4, seen.len())?;
        rassert!(seen.contains("Hello"))?;
        rassert!(seen.contains("World"))?;
        rassert!(seen.contains("Raise"))?;
        rassert!(seen.contains("Count"))?;

        Ok(()) as Result<(), Box<dyn Error>>
    });

    // Client A:
    let client_a = local.spawn_local(async move {
        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))
            .await
            .unwrap();
        let (send_udp, recv_udp, _) = dfir_rs::util::udp_lines(socket);

        let (seen_send, seen_recv) = dfir_rs::util::unbounded_channel();

        let mut df = dfir_syntax! {
            recv = source_stream(recv_udp)
                -> map(|r| r.unwrap())
                -> tee();
            recv[0] -> for_each(|x| println!("client A recv: {:?}", x));
            recv[1] -> map(|(s, _addr)| s) -> for_each(|s| seen_send.send(s).unwrap());

            // Sending
            source_iter([ "Hello", "World" ]) -> map(|s| (s.to_owned(), server_addr)) -> dest_sink(send_udp);
        };

        tokio::select! {
            _ = df.run() => (),
            _ = tokio::time::sleep(Duration::from_secs(1)) => (),
        };

        let seen: Vec<_> = collect_ready_async(seen_recv).await;
        rassert_eq!(&["Hello".to_owned(), "World".to_owned()], &*seen)?;

        Ok(()) as Result<(), Box<dyn Error>>
    });

    // Client B:
    let client_b = local.spawn_local(async move {
        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))
            .await
            .unwrap();
        let (send_udp, recv_udp, _) = dfir_rs::util::udp_lines(socket);

        let (seen_send, seen_recv) = dfir_rs::util::unbounded_channel();

        let mut df = dfir_syntax! {
            recv = source_stream(recv_udp)
                -> map(|r| r.unwrap())
                -> tee();
            recv[0] -> for_each(|x| println!("client B recv: {:?}", x));
            recv[1] -> map(|(s, _addr)| s) -> for_each(|s| seen_send.send(s).unwrap());

            // Sending
            source_iter([ "Raise", "Count" ]) -> map(|s| (s.to_owned(), server_addr)) -> dest_sink(send_udp);
        };

        tokio::select! {
            _ = df.run() => (),
            _ = tokio::time::sleep(Duration::from_secs(1)) => (),
        };

        let seen: Vec<_> = collect_ready_async(seen_recv).await;
        rassert_eq!(&["Raise".to_owned(), "Count".to_owned()], &*seen)?;

        Ok(()) as Result<(), Box<dyn Error>>
    });

    local.await;
    serv.await??;
    client_a.await??;
    client_b.await??;

    Ok(())
}

#[multiplatform_test(dfir, env_tracing)]
pub async fn test_echo_tcp() -> Result<(), Box<dyn Error>> {
    let local = LocalSet::new();

    // Port 0 -> picks any available port.
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await?;
    let addr = listener.local_addr()?;

    // Server:
    let serv = local.spawn_local(async move {
        let (server_stream, _) = listener.accept().await?;
        let (server_send, server_recv) = tcp_lines(server_stream);

        println!("Server accepted connection!");

        let (seen_send, seen_recv) = dfir_rs::util::unbounded_channel();

        let mut df: Dfir = dfir_syntax! {
            rev = source_stream(server_recv)
                -> map(|x| x.unwrap())
                -> tee();
            rev[0] -> dest_sink(server_send);
            rev[1] -> for_each(|s| seen_send.send(s).unwrap());
        };

        tokio::time::timeout(Duration::from_secs(1), df.run())
            .await
            .expect_err("Expected time out");

        let seen: Vec<_> = collect_ready_async(seen_recv).await;
        rassert_eq!(&["Hello".to_owned(), "World".to_owned()], &*seen)?;

        Ok(()) as Result<(), Box<dyn Error>>
    });

    // Client:
    let client = local.spawn_local(async move {
        let client_stream = TcpStream::connect(addr).await?;
        let (client_send, client_recv) = tcp_lines(client_stream);

        println!("Client connected!");

        let (seen_send, seen_recv) = dfir_rs::util::unbounded_channel();

        let mut df = dfir_syntax! {
            recv = source_stream(client_recv)
                -> map(|x| x.unwrap())
                -> tee();

            recv[0] -> for_each(|s| println!("echo {}", s));
            recv[1] -> for_each(|s| seen_send.send(s).unwrap());

            source_iter([
                "Hello",
                "World",
            ]) -> dest_sink(client_send);
        };

        println!("Client running!");

        tokio::time::timeout(Duration::from_secs(1), df.run())
            .await
            .expect_err("Expected time out");

        let seen: Vec<_> = collect_ready_async(seen_recv).await;
        rassert_eq!(&["Hello".to_owned(), "World".to_owned()], &*seen)?;

        Ok(()) as Result<(), Box<dyn Error>>
    });

    local.await;
    serv.await??;
    client.await??;

    Ok(())
}

#[multiplatform_test(dfir, env_tracing)]
pub async fn test_echo() {
    // An edge in the input data = a pair of `usize` vertex IDs.
    let (lines_send, lines_recv) = dfir_rs::util::unbounded_channel::<String>();

    // LinesCodec separates each line from `lines_recv` with `\n`.
    let stdout_lines = FramedWrite::new(tokio::io::stdout(), LinesCodec::new());

    let mut df: Dfir = dfir_syntax! {
        source_stream(lines_recv) -> dest_sink(stdout_lines);
    };
    assert_graphvis_snapshots!(df);
    df.run_available().await;

    lines_send.send("Hello".to_owned()).unwrap();
    lines_send.send("World".to_owned()).unwrap();
    df.run_available().await;

    lines_send.send("Hello".to_owned()).unwrap();
    lines_send.send("World".to_owned()).unwrap();
    df.run_available().await;

    // Allow background thread to catch up.
    tokio::time::sleep(Duration::from_secs(1)).await;
}

#[multiplatform_test(dfir, env_tracing)]
pub async fn test_futures_stream_sink() {
    const MAX: usize = 20;

    let (mut send, recv) = futures::channel::mpsc::channel::<usize>(5);
    send.try_send(0).unwrap();

    let (seen_send, seen_recv) = dfir_rs::util::unbounded_channel();

    let mut df = dfir_syntax! {
        recv = source_stream(recv) -> tee();
        recv[0] -> map(|x| x + 1)
            -> filter(|&x| x < MAX)
            -> dest_sink(send);
        recv[1] -> for_each(|x| seen_send.send(x).unwrap());
    };

    tokio::time::timeout(Duration::from_secs(1), df.run())
        .await
        .expect_err("Expected timeout, `run_async` doesn't return.");

    let seen: Vec<_> = collect_ready_async(seen_recv).await;
    assert_eq!(&std::array::from_fn::<_, MAX, _>(|i| i), &*seen);
}

#[multiplatform_test(dfir, env_tracing)]
async fn asynctest_dest_sink_bounded_channel() {
    // In this example we use a _bounded_ channel for our `Sink`. Users should consider using
    // `for_each()` with an `dfir_rs::util::unbounded_channel`.
    let (send, recv) = tokio::sync::mpsc::channel::<usize>(5);
    let send = tokio_util::sync::PollSender::new(send);
    let mut recv = tokio_stream::wrappers::ReceiverStream::new(recv);

    let mut flow = dfir_syntax! {
        source_iter(0..10) -> dest_sink(send);
    };
    let mut run_fut = std::pin::pin!(flow.run());
    tokio::time::timeout(Duration::from_millis(100), &mut run_fut)
        .await
        .expect_err("Expected time out");

    // Only 5 elemts received due to buffer size
    let out: Vec<_> = collect_ready_async(&mut recv).await;
    assert_eq!(&[0, 1, 2, 3, 4], &*out);

    tokio::time::timeout(Duration::from_millis(100), &mut run_fut)
        .await
        .expect_err("Expected time out");

    let out: Vec<_> = collect_ready_async(&mut recv).await;
    assert_eq!(&[5, 6, 7, 8, 9], &*out);
}

#[multiplatform_test(dfir, env_tracing)]
async fn asynctest_dest_sink_duplex() {
    use bytes::Bytes;
    use tokio::io::AsyncReadExt;
    use tokio_util::codec;

    // Like a channel, but for a stream of bytes instead of discrete objects.
    let (asyncwrite, mut asyncread) = tokio::io::duplex(256);
    // Now instead handle discrete byte lists by length-encoding them.
    let sink = codec::LengthDelimitedCodec::builder()
        // Use 1 byte len field (max 255) so we don't have to worry about endianness.
        .length_field_length(1)
        .new_write(asyncwrite);

    let mut flow = dfir_syntax! {
        source_iter([
            Bytes::from_static(b"hello"),
            Bytes::from_static(b"world"),
        ]) -> dest_sink(sink);
    };
    tokio::time::timeout(Duration::from_secs(1), flow.run())
        .await
        .expect_err("Expected time out");

    let mut buf = Vec::<u8>::new();
    asyncread.read_buf(&mut buf).await.unwrap();
    // `\x05` is length prefix of "5".
    assert_eq!(b"\x05hello\x05world", &*buf);
}

#[multiplatform_test(dfir, env_tracing)]
async fn asynctest_dest_asyncwrite_duplex() {
    use tokio::io::AsyncReadExt;

    // Like a channel, but for a stream of bytes instead of discrete objects.
    // This could be an output file, network port, stdout, etc.
    let (asyncwrite, mut asyncread) = tokio::io::duplex(256);
    let sink = FramedWrite::new(asyncwrite, BytesCodec::new());

    let mut flow = dfir_syntax! {
        source_iter([
            Bytes::from_static("hello".as_bytes()),
            Bytes::from_static("world".as_bytes()),
        ]) -> dest_sink(sink);
    };
    tokio::time::timeout(Duration::from_secs(1), flow.run())
        .await
        .expect_err("Expected time out");

    let mut buf = Vec::<u8>::new();
    asyncread.read_buf(&mut buf).await.unwrap();
    // `\x05` is length prefix of "5".
    assert_eq!(b"helloworld", &*buf);
}

#[multiplatform_test(dfir, env_tracing)]
async fn asynctest_source_stream() {
    let (a_send, a_recv) = dfir_rs::util::unbounded_channel::<usize>();
    let (b_send, b_recv) = dfir_rs::util::unbounded_channel::<usize>();
    let (c_send, c_recv) = dfir_rs::util::unbounded_channel::<usize>();

    let task_a = tokio::task::spawn_local(async move {
        let mut flow = dfir_syntax! {
            source_stream(a_recv) -> for_each(|x| { b_send.send(x).unwrap(); });
        };
        flow.run().await.unwrap();
    });
    let task_b = tokio::task::spawn_local(async move {
        let mut flow = dfir_syntax! {
            source_stream(b_recv) -> for_each(|x| { c_send.send(x).unwrap(); });
        };
        flow.run().await.unwrap();
    });

    a_send.send(1).unwrap();
    a_send.send(2).unwrap();
    a_send.send(3).unwrap();

    tokio::select! {
        biased;
        _ = task_a => unreachable!(),
        _ = task_b => unreachable!(),
        _ = tokio::task::yield_now() => (),
    };

    assert_eq!(
        &[1, 2, 3],
        &*collect_ready_async::<Vec<_>, _>(&mut { c_recv }).await
    );
}

/// Check to make sure hf.run() does not hang due to replaying stateful operators saturating
/// `run_available()`.
///
/// This test is a little bit race-ey... if for some insane reason a tick (task_b) runs longer than
/// the send loop delay (task_a).
#[multiplatform_test(dfir, env_tracing)]
async fn asynctest_check_state_yielding() {
    let (a_send, a_recv) = dfir_rs::util::unbounded_channel::<usize>();
    let (b_send, mut b_recv) = dfir_rs::util::unbounded_channel::<usize>();

    let task_a = tokio::task::spawn_local(
        async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            for a in 0..10 {
                tracing::debug!(a = a, "Sending.");
                a_send.send(a).unwrap();
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            tokio::task::yield_now().await;
        }
        .instrument(tracing::debug_span!("task_a")),
    );

    let task_b = tokio::task::spawn_local(
        async move {
            let mut hf = dfir_syntax! {
                source_stream(a_recv)
                    -> reduce::<'static>(|a: &mut _, b| *a += b)
                    -> for_each(|x| b_send.send(x).unwrap());
            };

            tokio::select! {
                biased;
                _ = hf.run() => panic!("`run_async()` should run forever."),
                _ = task_a => tracing::info!("`task_a` (sending) complete."),
            }

            assert_eq!(
                [0, 1, 3, 6, 10, 15, 21, 28, 36, 45]
                    .into_iter()
                    .collect::<BTreeSet<_>>(),
                collect_ready_async(&mut b_recv).await
            );
        }
        .instrument(tracing::debug_span!("task_b")),
    );

    task_b.await.unwrap();
}

#[multiplatform_test(dfir, env_tracing)]
async fn asynctest_repeat_iter() {
    let (b_send, b_recv) = dfir_rs::util::unbounded_channel::<usize>();

    let mut hf = dfir_syntax! {
        source_iter(0..3) -> persist::<'static>()
            -> for_each(|x| b_send.send(x).unwrap());
    };
    hf.run_available().await;

    let seen: Vec<_> = collect_ready_async(b_recv).await;
    assert_eq!(&[0, 1, 2], &*seen);
}

#[multiplatform_test(dfir, env_tracing)]
async fn asynctest_event_repeat_iter() {
    let (a_send, a_recv) = dfir_rs::util::unbounded_channel::<usize>();
    let (b_send, b_recv) = dfir_rs::util::unbounded_channel::<usize>();

    let mut hf = dfir_syntax! {
        source_iter(0..3) -> persist::<'static>() -> my_union;
        source_stream(a_recv) -> my_union;
        my_union = union() -> for_each(|x| b_send.send(x).unwrap());
    };

    let send_task = tokio::task::spawn({
        async move {
            // Wait, then send `10`.
            tokio::time::sleep(Duration::from_millis(100)).await;
            tracing::debug!("sending `10`.");
            a_send.send(10).unwrap();
        }
        .instrument(tracing::debug_span!("sender"))
    });

    // Run until barrier completes.
    tokio::select! {
        biased; // `run_async` needs to be polled to process the data first, otherwise the task may complete before data is processed.
        _ = hf.run() => panic!("`run_async()` should run forever."),
        _ = send_task => tracing::info!("sending complete"),
    };

    let seen: Vec<_> = collect_ready_async(b_recv).await;
    assert_eq!(&[0, 1, 2, 0, 1, 2, 10], &*seen);
}

#[multiplatform_test(dfir, env_tracing)]
async fn asynctest_tcp() {
    let (tx_out, rx_out) = dfir_rs::util::unbounded_channel::<String>();

    let (tx, rx, server_addr) = dfir_rs::util::bind_tcp_lines("127.0.0.1:0".parse().unwrap()).await;
    let mut echo_server = dfir_syntax! {
        source_stream(rx)
            -> filter_map(Result::ok)
            -> dest_sink(tx);
    };

    let (tx, rx) = dfir_rs::util::connect_tcp_lines();
    let mut echo_client = dfir_syntax! {
        source_iter([("Hello".to_owned(), server_addr)])
            -> dest_sink(tx);

        source_stream(rx)
            -> filter_map(Result::ok)
            -> map(|(string, _)| string)
            -> for_each(|x| tx_out.send(x).unwrap());
    };

    tokio::time::timeout(
        Duration::from_millis(200),
        futures::future::join(echo_server.run(), echo_client.run()),
    )
    .await
    .expect_err("Expected timeout");

    let seen: Vec<_> = collect_ready_async(rx_out).await;
    assert_eq!(&["Hello".to_owned()], &*seen);
}

#[multiplatform_test(dfir, env_tracing)]
async fn asynctest_udp() {
    let (tx_out, rx_out) = dfir_rs::util::unbounded_channel::<String>();

    let (tx, rx, server_addr) = dfir_rs::util::bind_udp_lines("127.0.0.1:0".parse().unwrap()).await;
    let mut echo_server = dfir_syntax! {
        source_stream(rx)
            -> filter_map(Result::ok)
            -> dest_sink(tx);
    };

    let (tx, rx, _) = dfir_rs::util::bind_udp_lines("127.0.0.1:0".parse().unwrap()).await;
    let mut echo_client = dfir_syntax! {
        source_iter([("Hello".to_owned(), server_addr)])
            -> dest_sink(tx);

        source_stream(rx)
            -> filter_map(Result::ok)
            -> map(|(string, _)| string)
            -> for_each(|x| tx_out.send(x).unwrap());
    };

    tokio::time::timeout(
        Duration::from_millis(200),
        futures::future::join(echo_server.run(), echo_client.run()),
    )
    .await
    .expect_err("Expected timeout");

    let seen: Vec<_> = collect_ready_async(rx_out).await;
    assert_eq!(&["Hello".to_owned()], &*seen);
}
