use hydro_lang::live_collections::stream::NoOrder;
use hydro_lang::location::{Location, NoTick};
use hydro_lang::prelude::*;

pub struct CounterServer;

#[expect(clippy::type_complexity, reason = "output types with orderings")]
pub fn keyed_counter_service<'a, L: Location<'a> + NoTick>(
    increment_requests: KeyedStream<u32, String, L, Unbounded>,
    get_requests: KeyedStream<u32, String, L, Unbounded>,
) -> (
    KeyedStream<u32, String, L, Unbounded>,
    KeyedStream<u32, (String, usize), L, Unbounded, NoOrder>,
) {
    let atomic_tick = increment_requests.location().tick();
    let increment_request_processing = increment_requests.atomic(&atomic_tick);
    let current_count = increment_request_processing
        .clone()
        .entries()
        .map(q!(|(_, key)| (key, ())))
        .into_keyed()
        .value_counts();
    let increment_ack = increment_request_processing.end_atomic();

    let requests_regrouped = get_requests
        .entries()
        .map(q!(|(cid, key)| (key, cid)))
        .into_keyed();

    let get_lookup = sliced! {
        let request_batch = use(requests_regrouped, nondet!(/** we never observe batch boundaries */));
        let count_snapshot = use::atomic(current_count, nondet!(/** atomicity guarantees consistency wrt increments */));

        request_batch.join_keyed_singleton(count_snapshot)
    };

    let get_response = get_lookup
        .entries()
        .map(q!(|(key, (client, count))| (client, (key, count))))
        .into_keyed();

    (increment_ack, get_response)
}

#[cfg(test)]
mod tests {
    use hydro_lang::prelude::*;

    use super::*;

    #[test]
    fn test_counter_read_after_write() {
        let mut flow = FlowBuilder::new();
        let process = flow.process::<CounterServer>();

        let (inc_in_port, inc_requests) = process.sim_input();
        let inc_requests = inc_requests.into_keyed();

        let (get_in_port, get_requests) = process.sim_input();
        let get_requests = get_requests.into_keyed();

        let (inc_acks, get_responses) = keyed_counter_service(inc_requests, get_requests);

        let inc_out_port = inc_acks.entries().sim_output();
        let get_out_port = get_responses.entries().sim_output();

        flow.sim().exhaustive(async || {
            inc_in_port.send((1, "abc".to_owned()));
            inc_out_port
                .assert_yields_unordered([(1, "abc".to_owned())])
                .await;
            get_in_port.send((1, "abc".to_owned()));
            get_out_port
                .assert_yields_only_unordered([(1, ("abc".to_owned(), 1))])
                .await;
        });
    }
}
