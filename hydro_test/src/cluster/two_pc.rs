use hydro_lang::*;
use hydro_std::quorum::collect_quorum;

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
        .for_each(q!(|t| println!(
            "receive transaction {}, ready to broadcast",
            t
        )));

    // broadcast prepare message to participants.
    let p_receive_prepare = c_receive_client_transactions.broadcast_bincode(&participants);

    // participant 1 aborts transaction 1
    let p_ready_to_commit = p_receive_prepare.map(q!(move |t| (
        t,
        if t == 1 && CLUSTER_SELF_ID.raw_id == 1 {
            "abort".to_string()
        } else {
            "commit".to_string()
        }
    )));
    let c_received_reply = p_ready_to_commit.send_bincode(&coordinator);
    // c_received_reply.clone().inspect(q!(|(id, (t, reply))| println!("participant {id} said {reply} for transaction {t}")));

    // collect votes from participant.
    let coordinator_tick = coordinator.tick();
    let (c_all_commit, c_participant_voted_abort) = collect_quorum(
        c_received_reply
            .map(q!(|(id, (t, reply))| (
                t,
                if reply == "commit" { Ok(()) } else { Err(id) }
            )))
            .atomic(&coordinator_tick),
        num_participants as usize,
        num_participants as usize,
    );

    let p_receive_abort = c_participant_voted_abort
        // TODO(shadaj): if multiple participants vote abort we should deduplicate
        .inspect(q!(|(t, id)| println!(
            "{} vote abort for transaction {}",
            id, t
        )))
        .broadcast_bincode(&participants);
    let c_receive_ack = p_receive_abort.send_bincode(&coordinator);
    c_receive_ack.for_each(q!(|(id, (t, _))| println!(
        "Coordinator receive participant {} abort for transaction {}",
        id, t
    )));

    // broadcast commit transactions to participants.
    let p_receive_commit = c_all_commit.broadcast_bincode(&participants);
    // p_receive_commit.clone().for_each(q!(|t| println!("commit for transaction {}", t)));

    let c_receive_ack = p_receive_commit.send_bincode(&coordinator);
    c_receive_ack.for_each(q!(|(id, t)| println!(
        "receive participant {} commit for transaction {}",
        id, t
    )));
    (coordinator, participants, client)
}
