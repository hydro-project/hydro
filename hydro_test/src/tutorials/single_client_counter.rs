use hydro_lang::prelude::*;

pub struct CounterServer;

pub fn single_client_counter_service<'a>(
    increment_requests: Stream<(), Process<'a, CounterServer>>,
    get_requests: Stream<(), Process<'a, CounterServer>>,
) -> (
    Stream<(), Process<'a, CounterServer>>, // increment acknowledgments
    Stream<usize, Process<'a, CounterServer>>, // get responses
) {
    let atomic_tick = increment_requests.location().tick();
    let increment_request_processing = increment_requests.atomic(&atomic_tick);
    let current_count = increment_request_processing.clone().count();
    let increment_ack = increment_request_processing.end_atomic();

    let get_response = sliced! {
        let request_batch = use(get_requests, nondet!(/** we never observe batch boundaries */));
        let count_snapshot = use::atomic(current_count, nondet!(/** atomicity guarantees consistency wrt increments */));

        request_batch.cross_singleton(count_snapshot).map(q!(|(_, count)| count))
    };

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

        let (inc_in_port, inc_requests) = process.sim_input();
        let (get_in_port, get_requests) = process.sim_input();

        let (inc_acks, get_responses) = single_client_counter_service(inc_requests, get_requests);

        let inc_out_port = inc_acks.sim_output();
        let get_out_port = get_responses.sim_output();

        flow.sim().exhaustive(async || {
            inc_in_port.send(());
            inc_out_port.assert_yields([()]).await;
            get_in_port.send(());
            get_out_port.assert_yields_only([1]).await;
        });
    }
}
