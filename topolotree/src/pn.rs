use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use std::rc::Rc;

use dfir_rs::dfir_syntax;
use dfir_rs::scheduled::ticks::TickInstant;
use dfir_rs::serde::{Deserialize, Serialize};
use dfir_rs::util::deploy::{
    ConnectedDemux, ConnectedDirect, ConnectedSink, ConnectedSource, ConnectedTagged,
};
use dfir_rs::util::{deserialize_from_bytes, serialize_to_bytes};

mod protocol;
use protocol::*;

type NextStateType = (u64, Rc<RefCell<(Vec<u64>, Vec<u64>)>>);

#[derive(Serialize, Deserialize, Clone, Debug)]
enum GossipOrIncrement {
    Gossip(Vec<NextStateType>),
    Increment(u64, i64),
}

#[dfir_rs::main]
async fn main() {
    let ports = dfir_rs::util::deploy::init::<()>().await;

    let my_id: Vec<usize> = serde_json::from_str(&std::env::args().nth(1).unwrap()).unwrap();
    let my_id = my_id[0];
    let num_replicas: Vec<usize> = serde_json::from_str(&std::env::args().nth(2).unwrap()).unwrap();
    let num_replicas = num_replicas[0];

    let increment_requests = ports
        .port("increment_requests")
        .connect::<ConnectedDirect>()
        .into_source();

    let query_responses = ports
        .port("query_responses")
        .connect::<ConnectedDirect>()
        .into_sink();

    let to_peer = ports
        .port("to_peer")
        .connect::<ConnectedDemux<ConnectedDirect>>()
        .into_sink();

    let from_peer = ports
        .port("from_peer")
        .connect::<ConnectedTagged<ConnectedDirect>>()
        .into_source();

    let f1 = async move {
        #[cfg(target_os = "linux")]
        loop {
            let x = procinfo::pid::stat_self().unwrap();
            let bytes = x.rss * 1024 * 4;
            println!("memory,{}", bytes);
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    };

    let df = dfir_syntax! {
        next_state = union()
            -> fold::<'static>(|| (HashMap::<u64, Rc<RefCell<(Vec<u64>, Vec<u64>)>>>::new(), HashSet::new(), TickInstant::default()), |(cur_state, modified_tweets, last_tick): &mut (HashMap<_, _>, HashSet<_>, _), goi| {
                if context.current_tick() != *last_tick {
                    modified_tweets.clear();
                }

                match goi {
                    GossipOrIncrement::Gossip(gossip) => {
                        for (counter_id, gossip_rc) in gossip.iter() {
                            let gossip_borrowed = gossip_rc.as_ref().borrow();
                            let (pos, neg) = gossip_borrowed.deref();
                            let cur_value = cur_state.entry(*counter_id).or_insert(Rc::new(RefCell::new((
                                vec![0; num_replicas], vec![0; num_replicas]
                            ))));
                            let mut cur_value = cur_value.as_ref().borrow_mut();

                            for i in 0..num_replicas {
                                if pos[i] > cur_value.0[i] {
                                    cur_value.0[i] = pos[i];
                                    modified_tweets.insert(*counter_id);
                                }

                                if neg[i] > cur_value.1[i] {
                                    cur_value.1[i] = neg[i];
                                    modified_tweets.insert(*counter_id);
                                }
                            }
                        }
                    }
                    GossipOrIncrement::Increment(counter_id, delta) => {
                        let cur_value = cur_state.entry(counter_id).or_insert(Rc::new(RefCell::new((
                            vec![0; num_replicas], vec![0; num_replicas]
                        ))));
                        let mut cur_value = cur_value.as_ref().borrow_mut();

                        if delta > 0 {
                            cur_value.0[my_id] += delta as u64;
                        } else {
                            cur_value.1[my_id] += (-delta) as u64;
                        }

                        modified_tweets.insert(counter_id);
                    }
                }

                *last_tick = context.current_tick();
            })
            -> filter(|(_, _, tick)| *tick == context.current_tick())
            -> filter(|(_, modified_tweets, _)| !modified_tweets.is_empty())
            -> map(|(state, modified_tweets, _)| modified_tweets.iter().map(|t| (*t, state.get(t).unwrap().clone())).collect::<Vec<_>>())
            -> tee();

        source_stream(from_peer)
            -> map(|x| deserialize_from_bytes::<GossipOrIncrement>(&x.unwrap().1).unwrap())
            -> next_state;

        source_stream(increment_requests)
            -> map(|x| deserialize_from_bytes::<OperationPayload>(&x.unwrap()).unwrap())
            -> map(|t| GossipOrIncrement::Increment(t.key, t.change))
            -> next_state;

        all_peers = source_iter(0..num_replicas)
            -> filter(|x| *x != my_id);

        all_peers -> [0] broadcaster;
        next_state -> [1] broadcaster;
        broadcaster = cross_join::<'static, 'tick>()
            -> map(|(peer, state)| {
                (peer as u32, serialize_to_bytes(GossipOrIncrement::Gossip(state)))
            })
            -> dest_sink(to_peer);

        next_state
            -> flat_map(|a: Vec<NextStateType>| {
                a.into_iter().map(|(k, rc_array)| {
                    let rc_borrowed = rc_array.as_ref().borrow();
                    let (pos, neg) = rc_borrowed.deref();
                    QueryResponse {
                        key: k,
                        value: pos.iter().sum::<u64>() as i64 - neg.iter().sum::<u64>() as i64
                    }
                }).collect::<Vec<_>>()
            })
            -> map(serialize_to_bytes::<QueryResponse>)
            -> dest_sink(query_responses);
    };

    // initial memory
    #[cfg(target_os = "linux")]
    {
        let x = procinfo::pid::stat_self().unwrap();
        let bytes = x.rss * 1024 * 4;
        println!("memory,{}", bytes);
    }

    let f1_handle = tokio::spawn(f1);
    dfir_rs::util::deploy::launch_flow(df).await;
    f1_handle.abort();
}
