use hydroflow::util::cli::{ConnectedBidi, ConnectedDemux, ConnectedSink, ConnectedSource};
use hydroflow::util::{deserialize_from_bytes, serialize_to_bytes};
use hydroflow_datalog::datalog;

#[tokio::main]
async fn main() {
    let mut ports = hydroflow::util::cli::init().await;
    let vote_to_participant_port = ports
        .port("vote_to_participant")
        .connect::<ConnectedDemux<ConnectedBidi>>()
        .await;

    let peers = vote_to_participant_port.keys.clone();
    println!("peers: {:?}", peers);
    let vote_to_participant_sink = vote_to_participant_port.into_sink();

    let vote_from_participant_source = ports
        .port("vote_from_participant")
        .connect::<ConnectedBidi>()
        .await
        .into_source();

    let instruct_to_participant_sink = ports
        .port("instruct_to_participant")
        .connect::<ConnectedDemux<ConnectedBidi>>()
        .await
        .into_sink();

    let ack_from_participant_source = ports
        .port("ack_from_participant")
        .connect::<ConnectedBidi>()
        .await
        .into_source();

    let mut df = datalog!(
        r#"
        .input clientIn `repeat_iter([("vote".to_string(),),])`
        .output clientOut `for_each(|(i,msg):(u32,String,)| println!("committed {:?}: {:?}", i, msg))`

        # EDBs
        .input startIndex `repeat_iter([(1u32,),])`
        .input participants `repeat_iter(peers.clone()) -> map(|p| (p,))`
        .input success `repeat_iter([(true,),])`
        .input reject `repeat_iter([(false,),])`
        .input commitInstruct `repeat_iter([(true,),])`
        .input rollbackInstruct `repeat_iter([(false,),])`

        .async voteToParticipant `map(|(node_id, v):(u32,(u32,String))| (node_id, serialize_to_bytes(v))) -> dest_sink(vote_to_participant_sink)` `null::<(u32,String,)>()`
        .async voteFromParticipant `null::<(u32,String,bool,u32,)>()` `source_stream(vote_from_participant_source) -> map(|v| deserialize_from_bytes::<(u32,String,bool,u32,)>(v.unwrap()).unwrap())`
        .async instructToParticipant `map(|(node_id, v):(u32,(u32,String,bool))| (node_id, serialize_to_bytes(v))) -> dest_sink(instruct_to_participant_sink)` `null::<(u32,String,bool,)>()`
        .async ackFromParticipant `null::<(u32,String,u32,)>()` `source_stream(ack_from_participant_source) -> map(|v| deserialize_from_bytes::<(u32,String,u32,)>(v.unwrap()).unwrap())`

        # Persistence rules
        AllMsg(msg) :+ AllMsg(msg), !NextMsgToAssign(msg)
        AllVotes(i, msg, res, l_from) :+ AllVotes(i, msg, res, l_from)
        AllAcks(i, msg, l_from) :+ AllAcks(i, msg, l_from)
        indices(i) :+ indices(i)

        # Non-EDBs with initialized values
        indices(i) :- startIndex(i)

        # Phase 1a
        AllMsg(msg) :- clientIn(msg)
        NextMsgToAssign(choose(msg)) :- AllMsg(msg)
        MaxID(max(i)) :- indices(i)
        indices(i+1) :+ NextMsgToAssign(msg), MaxID(i)
        voteToParticipant@addr(i, msg) :~ participants(addr), NextMsgToAssign(msg), MaxID(i)

        // # Phase 1b, Phase 2a
        AllVotes(i, msg, res, l_from) :- voteFromParticipant(i, msg, res, l_from)
        unanimous(count(addr)) :- participants(addr)

        NumYesVotes(i, msg, count(l_from)) :- AllVotes(i, msg, res, l_from), success(res)
        msgCommitted(i, msg) :- NumYesVotes(i, msg, size), unanimous(size)
        instructToParticipant@addr(i, msg, type) :~ msgCommitted(i, msg), participants(addr), commitInstruct(type)

        msgAborted(i, msg) :- AllVotes(i, msg, res, l_from), reject(res)
        instructToParticipant@addr(i, msg, type) :~ msgAborted(i, msg), participants(addr),rollbackInstruct(type)

        # Phase 2b
        AllAcks(i, msg, l_from) :- ackFromParticipant(i, msg, l_from)
        NumAcks(i, msg, count(l_from)) :- AllAcks(i, msg, l_from)
        outputted(i) :+ NumAcks(i, msg, size), unanimous(size)
        clientOut(i, msg) :- NumAcks(i, msg, size), unanimous(size), !outputted(i)
    "#
    );

    df.run_async().await;
}
