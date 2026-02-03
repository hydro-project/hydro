use std::net::SocketAddr;

use dfir_rs::dfir_syntax;
use dfir_rs::scheduled::graph::Dfir;
use dfir_rs::util::{UdpSink, UdpStream};

use crate::helpers::parse_out;
use crate::protocol::Msg;
use crate::{Addresses, Opts};

pub(crate) async fn run_coordinator(outbound: UdpSink, inbound: UdpStream, opts: Opts) {
    println!("Coordinator live!");

    let path = opts.path();
    let mut df: Dfir = dfir_syntax! {
        // fetch subordinates from file, convert ip:port to a SocketAddr, and tee
        subords = source_json(path)
            -> flat_map(|json: Addresses| json.subordinates)
            -> map(|s| s.parse::<SocketAddr>().unwrap())
            -> tee();

        // phase_map tells us what phase each transaction is in
        // There are only 3 phases per xid:
        //   1. coordinator sends PREPARE, subordinates vote COMMIT/ABORT
        //   2. coordinator send final decision, subordinates ACK
        //   3. coordinate sends END, subordinates respond with ENDED
        // After phase 3 we delete the xid from the phase_map
        phase_map = union() -> persist_mut_keyed::<'mutable>();

        // set up channels
        outbound_chan = tee();
        outbound_chan[0] -> dest_sink_serde(outbound);
        inbound_chan = source_stream_serde(inbound) -> map(Result::unwrap) -> map(|(m, _a)| m) -> tee();
        msgs = inbound_chan[0] -> demux_enum::<Msg>();

        msgs[Prepare] -> errs;
        msgs[End] -> errs;
        errs = union() -> for_each(|m| println!("Received unexpected message type: {:?}", m));

        msgs[Ended]
            -> map(|(xid,)| dfir_rs::util::PersistenceKeyed::Delete(xid))
            -> defer_tick()
            -> phase_map;

        // we log all messages (in this prototype we just print)
        inbound_chan[1] -> for_each(|m| println!("Received {:?}", m));
        outbound_chan[1] -> for_each(|(m, a)| println!("Sending {:?} to {:?}", m, a));

        // setup broadcast channel to all subords
        broadcast_join = cross_join::<'tick, 'static>() -> outbound_chan;
        broadcast = union() -> [0]broadcast_join;
        subords[1] -> [1]broadcast_join;
        subords[2] -> for_each(|s| println!("Subordinate: {:?}", s));


        // Phase 1 initiate:
        // Given a transaction commit request from stdio, broadcast a Prepare to subordinates
        initiate = source_stdin()
            -> filter_map(|l: Result<std::string::String, std::io::Error>| parse_out(l.unwrap()))
            -> tee();
        initiate
            -> flat_map(|xid: u16| [dfir_rs::util::PersistenceKeyed::Delete(xid), dfir_rs::util::PersistenceKeyed::Persist(xid, 1)])
            -> phase_map;
        initiate
            -> map(|xid: u16| Msg::Prepare(xid))
            -> [0]broadcast;

        // Phase 1 responses:
        // as soon as we get an abort message for P1, we start Phase 2 with Abort.
        // We'll respond to each abort message: this is redundant but correct (and monotone)
        abort_p1s = msgs[Abort] -> tee();
        abort_p1s
            -> flat_map(|(xid,)| [dfir_rs::util::PersistenceKeyed::Delete(xid), dfir_rs::util::PersistenceKeyed::Persist(xid, 2)])
            -> defer_tick()
            -> phase_map;
        abort_p1s
            -> map(|(xid,)| Msg::Abort(xid))
            -> [1]broadcast;

        // count commit votes
        // XXX This fold_keyed accumulates xids without bound.
        // Should be replaced with a persist_mut_keyed and logic to manage it.
        commit_votes = msgs[Commit]
            -> map(|(xid,)| (xid, 1))
            -> fold_keyed::<'static, u16, u32>(|| 0, |acc: &mut _, val| *acc += val);

        // count subordinates
        subord_total = subords[0] -> fold::<'static>(|| 0, |a: &mut _, _b| *a += 1);

        // If commit_votes for this xid is the same as all_votes, send a P2 Commit message
        commit_votes -> map(|(xid, c)| (c, xid)) -> [0]committed;
        subord_total -> map(|c| (c, ())) -> [1]committed;
        committed = join::<'tick,'tick>() -> map(|(_c, (xid, ()))| xid);

        // the committed join would succeed forever once a transaction is chosen for commit
        // so we filter to only send the P2 commit message and output to screen if this xid is still in Phase 1
        // We also transition this xid to Phase 2 to start the next tick
        committed -> map(|xid| (xid, ())) -> [0]check_committed;
        phase_map -> [1]check_committed;
        check_committed = join::<'tick, 'tick>()
            -> map(|(xid, (_, phase))| (xid, phase))
            -> filter(|(_xid, phase)| *phase == 1)
            -> map(|(xid, _phase)| xid)
            -> tee();
        // update the phase_map
        check_committed
            -> flat_map(|xid| [dfir_rs::util::PersistenceKeyed::Delete(xid), dfir_rs::util::PersistenceKeyed::Persist(xid, 2)])
            -> defer_tick()
            -> phase_map;
        // broadcast the P2 commit message
        check_committed
            -> map(Msg::Commit)
            -> [2]broadcast;

        // Handle p2 acknowledgments by sending an End message
        ack_p2s = msgs[AckP2] -> tee();
        ack_p2s
            -> flat_map(|(xid,)| [dfir_rs::util::PersistenceKeyed::Delete(xid), dfir_rs::util::PersistenceKeyed::Persist(xid, 3)])
            -> defer_tick()
            -> phase_map;
        ack_p2s
            -> map(|(xid,)| Msg::End(xid))
            -> [3]broadcast;

        // Handler for ended acknowledgments not necessary; we just print them
    };

    #[cfg(feature = "debugging")]
    if let Some(graph) = opts.graph {
        let serde_graph = df
            .meta_graph()
            .expect("No graph found, maybe failed to parse.");
        serde_graph.open_graph(graph, opts.write_config).unwrap();
    }

    let None = df.run().await;
}
