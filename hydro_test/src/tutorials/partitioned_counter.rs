use std::hash::{DefaultHasher, Hash, Hasher};

use hydro_lang::live_collections::stream::NoOrder;
use hydro_lang::location::MemberId;
use hydro_lang::prelude::*;

pub struct CounterServer;
pub struct CounterShard;

#[expect(clippy::type_complexity, reason = "output types with orderings")]
pub fn sharded_counter_service<'a>(
    leader: &Process<'a, CounterServer>,
    shard_servers: &Cluster<'a, CounterShard>,
    increment_requests: KeyedStream<u32, String, Process<'a, CounterServer>>,
    get_requests: KeyedStream<u32, String, Process<'a, CounterServer>>,
) -> (
    KeyedStream<u32, String, Process<'a, CounterServer>, Unbounded, NoOrder>,
    KeyedStream<u32, (String, usize), Process<'a, CounterServer>, Unbounded, NoOrder>,
) {
    let sharded_increment_requests = increment_requests
        .prefix_key(q!(|(_client, key)| {
            let mut hasher = DefaultHasher::new();
            key.hash(&mut hasher);
            MemberId::from_raw_id(hasher.finish() as u32 % 5)
        }))
        .demux_bincode(shard_servers);

    let sharded_get_requests = get_requests
        .prefix_key(q!(|(_client, key)| {
            let mut hasher = DefaultHasher::new();
            key.hash(&mut hasher);
            MemberId::from_raw_id(hasher.finish() as u32 % 5)
        }))
        .demux_bincode(shard_servers);

    let (sharded_increment_ack, sharded_get_response) = super::keyed_counter::keyed_counter_service(
        sharded_increment_requests,
        sharded_get_requests,
    );

    let increment_ack = sharded_increment_ack.send_bincode(leader).drop_key_prefix();

    let get_response = sharded_get_response.send_bincode(leader).drop_key_prefix();

    (increment_ack, get_response)
}

#[cfg(test)]
mod tests {
    use hydro_lang::prelude::*;

    use super::*;

    #[test]
    fn test_counter_read_after_write() {
        let flow = FlowBuilder::new();
        let process = flow.process();
        let shards = flow.cluster();

        let (inc_in_port, inc_requests) = process.sim_input();
        let inc_requests = inc_requests.into_keyed();

        let (get_in_port, get_requests) = process.sim_input();
        let get_requests = get_requests.into_keyed();

        let (inc_acks, get_responses) =
            sharded_counter_service(&process, &shards, inc_requests, get_requests);

        let inc_out_port = inc_acks.entries().sim_output();
        let get_out_port = get_responses.entries().sim_output();

        flow.sim()
            .with_cluster_size(&shards, 5)
            .exhaustive(async || {
                inc_in_port.send((1, "abc".to_string()));
                inc_out_port
                    .assert_yields_unordered([(1, "abc".to_string())])
                    .await;
                get_in_port.send((1, "abc".to_string()));
                get_out_port
                    .assert_yields_only_unordered([(1, ("abc".to_string(), 1))])
                    .await;
            });
    }
}
