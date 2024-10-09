use hydroflow::util::deploy::{ConnectedDemux, ConnectedDirect, ConnectedSink, ConnectedSource};
use hydroflow::util::{deserialize_from_bytes, serialize_to_bytes};
use hydroflow_datalog::datalog;

#[hydroflow::main]
async fn main() {
    let ports = hydroflow::util::deploy::init::<()>().await;
    let to_replica_source = ports
        .port("to_replica")
        .connect::<ConnectedDirect>()
        .await
        .into_source();

    let from_replica_port = ports
        .port("from_replica")
        .connect::<ConnectedDemux<ConnectedDirect>>()
        .await;

    let peers = from_replica_port.keys.clone();
    let from_replica_sink = from_replica_port.into_sink();

    let my_id: Vec<u32> = serde_json::from_str(&std::env::args().nth(1).unwrap()).unwrap();
    println!("my_id: {:?}", my_id);

    let mut df = datalog!(
        r#"
            .input myID `source_iter(my_id.clone()) -> persist::<'static>() -> map(|p| (p,))`
            .input leader `source_iter(peers.clone()) -> persist::<'static>() -> map(|p| (p,))`
            .async voteToReplica `null::<(String,)>()` `source_stream(to_replica_source) -> map(|x| deserialize_from_bytes::<(String,)>(x.unwrap()).unwrap())`
            .async voteFromReplica `map(|(node_id, v)| (node_id, serialize_to_bytes(v))) -> dest_sink(from_replica_sink)` `null::<(u32,String,)>()`

            voteFromReplica@addr(i, v) :~ voteToReplica(v), leader(addr), myID(i)
        "#
    );

    df.run_async().await;
}
