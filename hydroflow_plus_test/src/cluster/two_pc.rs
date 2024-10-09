use hydroflow_plus::*;
use stageleft::*;

// if the variable start with p, that means current work is at the participant side. if start with c, at coordinator side.
//

pub struct Participants {}

pub struct Coordinator {}

pub struct Client {}

pub fn two_pc<'a>(
    flow: &FlowBuilder<'a>,
    num_participants: u32,
) -> (
    Process<'a, Coordinator>,
    Cluster<'a, Participants>,
    Process<'a, Client>,
) {
    // Assume single client.
    let client = flow.process::<Client>();

    // Assume single coordinator.
    let coordinator = flow.process::<Coordinator>();

    // Assume 3 participants.
    let participants = flow.cluster::<Participants>();

    // assume 3 transactions are generated from 0 to 3
    let client_transaction = client.source_iter(q!(0..3));

    let c_receive_client_transactions = client_transaction.send_bincode(&coordinator);
    c_receive_client_transactions
        .clone()
        .inspect(q!(|t| println!(
            "receive transaction {}, ready to broadcast",
            t
        )));

    // broadcast prepare message to participants.
    let p_receive_prepare = c_receive_client_transactions.broadcast_bincode(&participants);

    // assume all participants reply commit
    let p_ready_to_commit = p_receive_prepare.map(q!(|t| (t, String::from("commit"))));
    let c_received_reply = p_ready_to_commit.send_bincode(&coordinator);
    // c_received_reply.clone().inspect(q!(|(id, (t, reply))| println!("participant {id} said {reply} for transaction {t}")));

    // collect votes from participant.
    // aborted transactions.
    let c_participant_voted_abort = c_received_reply
        .clone()
        .filter(q!(|(_id, (_t, reply))| reply == "abort"))
        .map(q!(|(id, (t, _reply))| (t, id)));
    let p_receive_abort = c_participant_voted_abort.broadcast_bincode(&participants);
    p_receive_abort.clone().inspect(q!(|(t, id)| println!(
        "{} vote abort for transaction {}",
        id, t
    )));
    let c_receive_ack = p_receive_abort.send_bincode(&coordinator);
    c_receive_ack.for_each(q!(|(id, (t, _))| println!(
        "Coordinator receive participant {} abort for transaction {}",
        id, t
    )));

    // committed transactions
    let c_participant_voted_commit = c_received_reply
    .filter(q!(|(_id, (_t, reply))| reply == "commit"))
    .map(q!(|(id, (t, _reply))| (t, id)))
    // fold_keyed: 1 input stream of type (K, V1), 1 output stream of type (K, V2). 
    // The output will have one tuple for each distinct K, with an accumulated value of type V2.
    .tick_batch().fold_keyed(q!(|| 0), q!(|old: &mut u32, _| *old += 1)).filter_map(q!(move |(t, count)| {
        // here I set the participant to 3. If want more or less participant, fix line 26 of examples/broadcast.rs
        if count == num_participants {
            Some(t)
        } else {
            None
        }
    }));

    // broadcast commit transactions to participants.
    let p_receive_commit = c_participant_voted_commit
        .all_ticks()
        .broadcast_bincode(&participants);
    // p_receive_commit.clone().for_each(q!(|t| println!("commit for transaction {}", t)));

    let c_receive_ack = p_receive_commit.send_bincode(&coordinator);
    c_receive_ack.for_each(q!(|(id, t)| println!(
        "receive participant {} commit for transaction {}",
        id, t
    )));
    (coordinator, participants, client)
}
