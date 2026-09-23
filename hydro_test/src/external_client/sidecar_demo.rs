//! A simple length-delimited TCP sidecar for the `sidecar_bidi` example.
//!
//! Returns `(Stream<String>, Sink<String>)` to the framework. Internally
//! spawns tasks that bridge a TCP connection to mpsc channels backing
//! the stream and sink.

use futures::sink::SinkExt;
use futures::stream::StreamExt;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};
use tokio_util::sync::PollSender;

/// Create a sidecar that listens on the given port.
///
/// Spawns background tasks that:
/// 1. Accept a single TCP connection on the given port
/// 2. Read length-delimited frames and forward them as the returned Stream
/// 3. Write items sent to the returned Sink as length-delimited frames
pub fn create(port: u16) -> (ReceiverStream<String>, PollSender<String>) {
    // Channel from TCP reader → dataflow (the returned Stream)
    let (to_df_tx, to_df_rx) = mpsc::channel::<String>(1024);
    // Channel from dataflow → TCP writer (the returned Sink)
    let (from_df_tx, mut from_df_rx) = mpsc::channel::<String>(1024);

    // Spawn the TCP accept + bridge logic
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
            .await
            .unwrap();
        let (stream, _addr) = listener.accept().await.unwrap();
        let (reader, writer) = stream.into_split();

        let mut framed_read = FramedRead::new(reader, LengthDelimitedCodec::new());
        let mut framed_write = FramedWrite::new(writer, LengthDelimitedCodec::new());

        // Read from TCP, forward to dataflow
        let tx = to_df_tx;
        let read_task = tokio::spawn(async move {
            while let Some(Ok(frame)) = framed_read.next().await {
                let msg = String::from_utf8(frame.to_vec()).unwrap();
                if tx.send(msg).await.is_err() {
                    break;
                }
            }
        });

        // Read from dataflow, write to TCP
        while let Some(resp) = from_df_rx.recv().await {
            let bytes = resp.into_bytes();
            if framed_write.send(bytes.into()).await.is_err() {
                break;
            }
        }

        read_task.abort();
    });

    let stream = ReceiverStream::new(to_df_rx);
    let sink = PollSender::new(from_df_tx);

    (stream, sink)
}
