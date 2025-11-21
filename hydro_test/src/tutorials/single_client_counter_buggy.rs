use hydro_lang::prelude::*;

pub struct CounterServer;

#[expect(clippy::type_complexity, reason = "multiple outputs")]
pub fn single_client_counter_service_buggy<'a>(
    increment_requests: Stream<(), Process<'a, CounterServer>>,
    get_requests: Stream<(), Process<'a, CounterServer>>,
) -> (
    Stream<(), Process<'a, CounterServer>>, // increment acknowledgments
    Stream<usize, Process<'a, CounterServer>>, // get responses
) {
    let current_count = increment_requests.clone().count();
    let increment_ack = increment_requests;

    let get_response = sliced! {
        let request_batch = use(get_requests, nondet!(/** we never observe batch boundaries */));
        let count_snapshot = use(current_count, nondet!(/** intentional, based on when the request came in */));

        request_batch.cross_singleton(count_snapshot).map(q!(|(_, count)| count))
    };

    (increment_ack, get_response)
}
