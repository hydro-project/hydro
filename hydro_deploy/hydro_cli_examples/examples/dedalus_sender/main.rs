use hydroflow::tokio_stream::wrappers::IntervalStream;
use hydroflow::util::deploy::{ConnectedDemux, ConnectedDirect, ConnectedSink};
use hydroflow::util::serialize_to_bytes;
use hydroflow_datalog::datalog;

#[hydroflow::main]
async fn main() {
    let ports = hydroflow::util::deploy::init::<()>().await;
    let broadcast_port = ports
        .port("broadcast")
        .connect::<ConnectedDemux<ConnectedDirect>>()
        .await;

    let (peers, sender_i): (Vec<u32>, u32) =
        serde_json::from_str(&std::env::args().nth(1).unwrap()).unwrap();
    let broadcast_sink = broadcast_port.into_sink();

    let periodic = IntervalStream::new(tokio::time::interval(std::time::Duration::from_secs(1)));
    let to_repeat = [
        (format!("Hello {sender_i}"),),
        (format!("world {sender_i}"),),
    ];

    let df = datalog!(
        r#"
        .input repeated `spin() -> flat_map(|_| to_repeat.iter().cloned())`
        .input periodic `source_stream(periodic) -> map(|_| ())`
        .input peers `source_iter(peers.clone()) -> persist::<'static>() -> map(|p| (p,))`
        .async broadcast `map(|(node_id, v)| (node_id, serialize_to_bytes(v))) -> dest_sink(broadcast_sink)` `null::<(String,)>()`

        broadcast@n(x) :~ repeated(x), periodic(), peers(n)
    "#
    );

    hydroflow::util::deploy::launch_flow(df).await;
}
