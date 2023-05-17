use hydroflow::util::cli::{ConnectedBidi, ConnectedDemux, ConnectedSink, ConnectedSource};
use hydroflow::util::{deserialize_from_bytes, serialize_to_bytes};
use hydroflow_datalog::datalog;

#[tokio::main]
async fn main() {
    let mut ports = hydroflow::util::cli::init().await;
    let to_replica_port = ports
        .port("to_replica")
        .connect::<ConnectedDemux<ConnectedBidi>>()
        .await;

    let peers = to_replica_port.keys.clone();
    println!("peers: {:?}", peers);
    let to_replica_sink = to_replica_port.into_sink();

    let from_replica_source = ports
        .port("from_replica")
        .connect::<ConnectedBidi>()
        .await
        .into_source();

    let mut df = datalog!(
        r#"
        .input clientIn `repeat_iter([("vote".to_string(),),])`
        .output stdout `for_each(|_:(String,)| println!("voted"))`
        .input replicas `repeat_iter(peers.clone()) -> map(|p| (p,))`

        .async voteToReplica `map(|(node_id, v)| (node_id, serialize_to_bytes(v))) -> dest_sink(to_replica_sink)` `null::<(String,)>()`
        .async voteFromReplica `null::<(u32,String,)>()` `source_stream(from_replica_source) -> map(|v| deserialize_from_bytes::<(u32,String,)>(v.unwrap()).unwrap())`
        
        voteToReplica@addr(v) :~ clientIn(v), replicas(addr)
        allVotes(s, v) :- voteFromReplica(s, v)
        allVotes(s, v) :+ allVotes(s, v)
        voteCounts(count(l), v) :- allVotes(l, v)
        numReplicas(count(addr)) :- replicas(addr)
        stdout(v) :- clientIn(v), voteCounts(n, v), numReplicas(n)
    "#
    );

    df.run_async().await;
}
