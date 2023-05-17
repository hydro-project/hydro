use hydroflow::tokio_stream::wrappers::IntervalStream;
use hydroflow::util::cli::{ConnectedBidi, ConnectedDemux, ConnectedSink};
use hydroflow::util::serialize_to_bytes;
use hydroflow_datalog::datalog;

#[tokio::main]
async fn main() {
    let mut ports = hydroflow::util::cli::init().await;
    let broadcast_port = ports
        .port("broadcast")
        .connect::<ConnectedDemux<ConnectedBidi>>()
        .await;

    let (peers, sender_i): (Vec<u32>, u32) =
        serde_json::from_str(&std::env::args().nth(1).unwrap()).unwrap();
    let broadcast_sink = broadcast_port.into_sink();

    let periodic = IntervalStream::new(tokio::time::interval(std::time::Duration::from_secs(1)));
    let to_repeat = vec![
        (format!("Hello {sender_i}"),),
        (format!("world {sender_i}"),),
    ];

    let df = datalog!(
        r#"
        .input repeated `repeat_iter_external(to_repeat.iter().cloned())`
        .input periodic `source_stream(periodic) -> map(|_| ())`
        .input peers `repeat_iter(peers.clone()) -> map(|p| (p,))`
        .async broadcast `map(|(node_id, v)| (node_id, serialize_to_bytes(v))) -> dest_sink(broadcast_sink)` `null::<(String,)>()`

        broadcast@n(x) :~ repeated(x), periodic(), peers(n)
    "#
    );

    hydroflow::util::cli::launch_flow(df).await;
}
