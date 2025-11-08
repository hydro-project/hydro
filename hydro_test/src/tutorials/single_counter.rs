use hydro_lang::prelude::*;

pub struct CounterServer;
pub fn single_counter_service<'a>(
    increment_requests: KeyedStream<u32, (), Process<'a, CounterServer>>,
    get_requests: KeyedStream<u32, (), Process<'a, CounterServer>>,
) -> (
    KeyedStream<u32, (), Process<'a, CounterServer>>,
    KeyedStream<u32, usize, Process<'a, CounterServer>>,
) {
    let atomic_tick = increment_requests.location().tick();
    let increment_request_processing = increment_requests.atomic(&atomic_tick);
    let current_count = increment_request_processing.clone().values().count();
    let increment_ack = increment_request_processing.end_atomic();

    let get_response = sliced! {
        let request_batch = use(get_requests, nondet!(/** we never observe batch boundaries */));
        let count_snapshot = use::atomic(current_count, nondet!(/** atomicity guarantees consistency wrt increments */));

        request_batch.cross_singleton(count_snapshot).map(q!(|(_, count)| count))
    };

    (increment_ack, get_response)
}

// buggy version which does not guarantee consistent reads after increment acks
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
        let count_snapshot = use(current_count, nondet!(/** atomicity guarantees consistency wrt increments */));

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
        let external = flow.external::<()>();

        let (inc_in_port, inc_requests) = process.source_external_bincode(&external);
        let inc_requests = inc_requests.into_keyed();

        let (get_in_port, get_requests) = process.source_external_bincode(&external);
        let get_requests = get_requests.into_keyed();

        let (inc_acks, get_responses) = single_counter_service(inc_requests, get_requests);

        let inc_out_port = inc_acks.entries().send_bincode_external(&external);
        let get_out_port = get_responses.entries().send_bincode_external(&external);

        flow.sim().exhaustive(async |mut instance| {
            let inc_in_port = instance.connect(&inc_in_port);
            let get_in_port = instance.connect(&get_in_port);
            let mut inc_out_port = instance.connect(&inc_out_port);
            let get_out_port = instance.connect(&get_out_port);

            instance.launch();

            inc_in_port.send((1, ()));
            inc_out_port.assert_yields_unordered([(1, ())]).await;
            get_in_port.send((1, ()));
            get_out_port.assert_yields_only_unordered([(1, 1)]).await;
        });
    }

    #[test]
    #[should_panic]
    fn test_buggy_counter_read_after_write() {
        let flow = FlowBuilder::new();
        let process = flow.process();
        let external = flow.external::<()>();

        let (inc_in_port, inc_requests) = process.source_external_bincode(&external);
        let inc_requests = inc_requests.into_keyed();

        let (get_in_port, get_requests) = process.source_external_bincode(&external);
        let get_requests = get_requests.into_keyed();

        let (inc_acks, get_responses) = single_counter_service_buggy(inc_requests, get_requests);

        let inc_out_port = inc_acks.entries().send_bincode_external(&external);
        let get_out_port = get_responses.entries().send_bincode_external(&external);

        flow.sim().exhaustive(async |mut instance| {
            let inc_in_port = instance.connect(&inc_in_port);
            let get_in_port = instance.connect(&get_in_port);
            let mut inc_out_port = instance.connect(&inc_out_port);
            let get_out_port = instance.connect(&get_out_port);

            instance.launch();

            inc_in_port.send((1, ()));
            inc_out_port.assert_yields_unordered([(1, ())]).await;
            get_in_port.send((1, ()));
            get_out_port.assert_yields_only_unordered([(1, 1)]).await;
        });
    }
}
