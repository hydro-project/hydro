use hydro_lang::live_collections::stream::{NoOrder, Ordering};
use hydro_lang::location::{Location, NoTick};
use hydro_lang::prelude::*;

pub struct CounterServer;

pub fn keyed_counter_service_buggy<'a, L: Location<'a> + NoTick, O: Ordering>(
    increment_requests: KeyedStream<u32, String, L, Unbounded, O>,
    get_requests: KeyedStream<u32, String, L, Unbounded, O>,
) -> (
    KeyedStream<u32, String, L, Unbounded, O>,
    KeyedStream<u32, (String, usize), L, Unbounded, NoOrder>,
) {
    let current_count = increment_requests
        .clone()
        .entries()
        .map(q!(|(_, key)| (key, ())))
        .into_keyed()
        .value_counts();
    let increment_ack = increment_requests;

    let get_response = sliced! {
        let request_batch = use(get_requests, nondet!(/** we never observe batch boundaries */));
        let count_snapshot = use(current_count, nondet!(/** atomicity guarantees consistency wrt increments */));

        count_snapshot.get_many_if_present(request_batch.entries().map(q!(|(cid, key)| (key, cid))).into_keyed())
            .entries()
            .map(q!(|(key, (count, client))| (client, (key, count))))
    };

    (increment_ack, get_response.into_keyed())
}

#[cfg(test)]
mod tests {
    use hydro_lang::prelude::*;

    use super::*;

    #[test]
    #[should_panic]
    fn test_counter_read_after_write_buggy() {
        let flow = FlowBuilder::new();
        let process = flow.process::<CounterServer>();
        let external = flow.external::<()>();

        let (inc_in_port, inc_requests) = process.source_external_bincode(&external);
        let inc_requests = inc_requests.into_keyed();

        let (get_in_port, get_requests) = process.source_external_bincode(&external);
        let get_requests = get_requests.into_keyed();

        let (inc_acks, get_responses) = keyed_counter_service_buggy(inc_requests, get_requests);

        let inc_out_port = inc_acks.entries().send_bincode_external(&external);
        let get_out_port = get_responses.entries().send_bincode_external(&external);

        flow.sim().exhaustive(async |mut instance| {
            let inc_in_port = instance.connect(&inc_in_port);
            let get_in_port = instance.connect(&get_in_port);
            let mut inc_out_port = instance.connect(&inc_out_port);
            let get_out_port = instance.connect(&get_out_port);

            instance.launch();

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

    #[test]
    fn test_counter_read_after_write_instances() {
        let flow = FlowBuilder::new();
        let process = flow.process::<CounterServer>();
        let external = flow.external::<()>();

        let (inc_in_port, inc_requests) = process.source_external_bincode(&external);
        let inc_requests = inc_requests.into_keyed();

        let (get_in_port, get_requests) = process.source_external_bincode(&external);
        let get_requests = get_requests.into_keyed();

        let (inc_acks, get_responses) = keyed_counter_service_buggy(inc_requests, get_requests);

        let inc_out_port = inc_acks.entries().send_bincode_external(&external);
        let get_out_port = get_responses.entries().send_bincode_external(&external);

        let instances = flow.sim().exhaustive(async |mut instance| {
            let inc_in_port = instance.connect(&inc_in_port);
            let get_in_port = instance.connect(&get_in_port);
            let mut inc_out_port = instance.connect(&inc_out_port);
            let get_out_port = instance.connect(&get_out_port);

            instance.launch();

            inc_in_port.send((1, "abc".to_string()));
            inc_out_port
                .assert_yields_unordered([(1, "abc".to_string())])
                .await;
            get_in_port.send((1, "abc".to_string()));
            let _ = get_out_port.collect_sorted::<Vec<_>>().await;
        });

        assert_eq!(instances, 3);
    }
}
