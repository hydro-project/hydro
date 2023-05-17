use hydroflow::util::cli::{ConnectedBidi, ConnectedDemux, ConnectedSink, ConnectedSource};
use hydroflow::util::{deserialize_from_bytes, serialize_to_bytes};
use hydroflow_datalog::datalog;

#[tokio::main]
async fn main() {
    let mut ports = hydroflow::util::cli::init().await;
    let vote_to_participant_source = ports
        .port("vote_to_participant")
        .connect::<ConnectedBidi>()
        .await
        .into_source();

    let vote_from_participant_port = ports
        .port("vote_from_participant")
        .connect::<ConnectedDemux<ConnectedBidi>>()
        .await;

    let peers = vote_from_participant_port.keys.clone();
    let vote_from_participant_sink = vote_from_participant_port.into_sink();

    let instruct_to_participant_source = ports
        .port("instruct_to_participant")
        .connect::<ConnectedBidi>()
        .await
        .into_source();

    let ack_from_participant_sink = ports
        .port("ack_from_participant")
        .connect::<ConnectedDemux<ConnectedBidi>>()
        .await
        .into_sink();

    let my_id: Vec<u32> = serde_json::from_str(&std::env::args().nth(1).unwrap()).unwrap();
    println!("my_id: {:?}", my_id);
    println!("coordinator: {:?}", peers);

    let mut df = datalog!(
        r#"
        .input myID `repeat_iter(my_id.clone()) -> map(|p| (p,))`
        .input coordinator `repeat_iter(peers.clone()) -> map(|p| (p,))`
        .input verdict `repeat_iter([(true,),])`
        // .output voteOut `for_each(|(i,myID):(u32,u32,)| println!("participant {:?}: message {:?}", myID, i))`
        
        .async voteToParticipant `null::<(u32,String,)>()` `source_stream(vote_to_participant_source) -> map(|x| deserialize_from_bytes::<(u32,String,)>(x.unwrap()).unwrap())`
        .async voteFromParticipant `map(|(node_id, v)| (node_id, serialize_to_bytes(v))) -> dest_sink(vote_from_participant_sink)` `null::<(u32,String,)>()`
        .async instructToParticipant `null::<(u32,String,bool,)>()` `source_stream(instruct_to_participant_source) -> map(|x| deserialize_from_bytes::<(u32,String,bool,)>(x.unwrap()).unwrap())`
        .async ackFromParticipant `map(|(node_id, v)| (node_id, serialize_to_bytes(v))) -> dest_sink(ack_from_participant_sink)` `null::<(u32,String,u32,)>()`
    
        # .output verdictRequest    
        # .output log
        
        # verdictRequest(i, msg) :- voteToParticipant(i, msg)
        voteFromParticipant@addr(i, msg, res, l_from) :~ voteToParticipant(i, msg), coordinator(addr), myID(l_from), verdict(res)
        ackFromParticipant@addr(i, msg, l_from) :~ instructToParticipant(i, msg, b), coordinator(addr), myID(l_from)
        // voteOut(i, l) :- voteToParticipant(i, msg), myID(l)
        
        # log(i, msg, type) :- instructToParticipant(i, msg, type) # the log channel will sort everything out
        "#
    );

    df.run_async().await;
}
