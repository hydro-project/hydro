use hydro_lang::prelude::*;

pub struct CounterServer;

// buggy version which does not guarantee consistent reads after increment acks
#[expect(clippy::type_complexity, reason = "multiple outputs")]
pub fn single_counter_service_buggy<'a>(
    increment_requests: KeyedStream<u32, (), Process<'a, CounterServer>>,
    get_requests: KeyedStream<u32, (), Process<'a, CounterServer>>,
) -> (
    KeyedStream<u32, (), Process<'a, CounterServer>>,
    KeyedStream<u32, usize, Process<'a, CounterServer>>,
) {
    let current_count = increment_requests.clone().values().count();
    let increment_ack = increment_requests;

    let get_response = sliced! {
        let request_batch = use(get_requests, nondet!(/** we never observe batch boundaries */));
        let count_snapshot = use(current_count, nondet!(/** intentional, based on when the request came in */));

        request_batch.cross_singleton(count_snapshot).map(q!(|(_, count)| count))
    };

    (increment_ack, get_response)
}

#[cfg(test)]
mod tests {
    use hydro_lang::prelude::*;

    use super::*;

    #[test]
    #[should_panic]
    fn test_buggy_counter_read_after_write() {
        let flow = FlowBuilder::new();
        let process = flow.process();

        let (inc_in_port, inc_requests) = process.sim_input();
        let inc_requests = inc_requests.into_keyed();

        let (get_in_port, get_requests) = process.sim_input();
        let get_requests = get_requests.into_keyed();

        let (inc_acks, get_responses) = single_counter_service_buggy(inc_requests, get_requests);

        let inc_out_port = inc_acks.entries().sim_output();
        let get_out_port = get_responses.entries().sim_output();

        flow.sim().exhaustive(async || {
            inc_in_port.send((1, ()));
            inc_out_port.assert_yields_unordered([(1, ())]).await;
            get_in_port.send((1, ()));
            get_out_port.assert_yields_only_unordered([(1, 1)]).await;
        });
    }
}
