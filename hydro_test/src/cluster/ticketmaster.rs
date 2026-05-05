use std::collections::HashSet;

use hydro_lang::prelude::*;
use serde::{Deserialize, Serialize};

pub struct TicketServer;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BookResult {
    Ok(u32),
    SeatNotFound,
    AlreadyBooked,
}

/// A ticketmaster service that handles seat bookings without double-booking.
///
/// Uses atomic processing to ensure that concurrent booking requests for the
/// same seat are serialized, eliminating the race conditions present in the
/// naive replicated approach.
///
/// # Why the original architecture cannot compile in Hydro
///
/// The original ticketmaster used independent replicas that each processed
/// bookings from the network. In Hydro, network messages arrive as `NoOrder`
/// streams. Accumulating over a `NoOrder` stream with `fold` requires proving
/// commutativity — but booking logic (check-then-reserve) is fundamentally
/// non-commutative. Hydro rejects this at compile time:
///
/// ```compile_fail
/// use std::collections::HashSet;
/// use hydro_lang::live_collections::stream::NoOrder;
/// use hydro_lang::prelude::*;
///
/// struct Replica {}
///
/// fn buggy_replica_booking<'a>(replicas: &Cluster<'a, Replica>) {
///     // Network messages arrive in NoOrder (non-deterministic interleaving)
///     let bookings: Stream<(u32, u32), Cluster<'a, Replica>, Unbounded, NoOrder> = replicas
///         .source_iter(q!(vec![(1u32, 5u32)]))
///         .weaken_ordering::<NoOrder>();
///
///     // ERROR: fold on NoOrder stream requires commutativity proof.
///     // Booking logic is NOT commutative — who gets the seat depends on order.
///     let _booked = bookings.fold(
///         q!(|| HashSet::<u32>::new()),
///         q!(|booked, (_client, seat)| {
///             booked.insert(seat);
///         }),
///     );
/// }
/// ```
#[expect(clippy::type_complexity, reason = "output types with orderings")]
pub fn ticketmaster_service<'a>(
    total_seats: u32,
    book_requests: KeyedStream<u32, u32, Process<'a, TicketServer>>,
    get_requests: KeyedStream<u32, (), Process<'a, TicketServer>>,
) -> (
    KeyedStream<u32, BookResult, Process<'a, TicketServer>>,
    KeyedStream<u32, Vec<u32>, Process<'a, TicketServer>>,
) {
    let book_processing = book_requests.atomic();

    // Use entries_partially_ordered to get a TotalOrder stream of (client_id, seat_id),
    // then scan with global state to prevent double-booking.
    let book_response = book_processing
        .clone()
        .entries_partially_ordered(nondet!(/** client interleaving is non-deterministic */))
        .scan(
            q!(move || HashSet::<u32>::new()),
            q!(move |booked, (client_id, seat_id)| {
                let result = if seat_id == 0 || seat_id > total_seats {
                    BookResult::SeatNotFound
                } else if booked.contains(&seat_id) {
                    BookResult::AlreadyBooked
                } else {
                    booked.insert(seat_id);
                    BookResult::Ok(seat_id)
                };
                Some((client_id, result))
            }),
        )
        .into_keyed()
        .end_atomic();

    let booked_seats = book_processing
        .values()
        .filter(q!(move |seat_id| *seat_id > 0 && *seat_id <= total_seats))
        .fold(
            q!(|| HashSet::<u32>::new()),
            q!(
                |booked, seat_id| {
                    booked.insert(seat_id);
                },
                commutative = manual_proof!(/** set insert is commutative */)
            ),
        );

    let get_response = sliced! {
        let request_batch = use(get_requests, nondet!(/** we never observe batch boundaries */));
        let seats_snapshot = use::atomic(booked_seats, nondet!(/** atomicity guarantees consistency wrt bookings */));

        request_batch.cross_singleton(seats_snapshot).map(q!(move |(_, booked)| {
            (1..=total_seats).filter(|id| !booked.contains(id)).collect::<Vec<_>>()
        }))
    };

    (book_response, get_response)
}

#[cfg(test)]
mod tests {
    use hydro_lang::prelude::*;

    use super::*;

    #[test]
    fn test_book_seat_success() {
        let mut flow = FlowBuilder::new();
        let process = flow.process();

        let (book_in, book_requests) = process.sim_input();
        let book_requests = book_requests.into_keyed();
        let (_get_in, get_requests) = process.sim_input();
        let get_requests = get_requests.into_keyed();

        let (book_acks, _get_responses) = ticketmaster_service(10, book_requests, get_requests);
        let book_out = book_acks.entries().sim_output();

        flow.sim().exhaustive(async || {
            book_in.send((1, 3));
            book_out
                .assert_yields_unordered([(1, BookResult::Ok(3))])
                .await;
        });
    }

    #[test]
    fn test_double_book_rejected() {
        let mut flow = FlowBuilder::new();
        let process = flow.process();

        let (book_in, book_requests) = process.sim_input();
        let book_requests = book_requests.into_keyed();
        let (_get_in, get_requests) = process.sim_input();
        let get_requests = get_requests.into_keyed();

        let (book_acks, _get_responses) = ticketmaster_service(10, book_requests, get_requests);
        let book_out = book_acks.entries().sim_output();

        flow.sim().exhaustive(async || {
            book_in.send((1, 5));
            book_out
                .assert_yields_unordered([(1, BookResult::Ok(5))])
                .await;

            book_in.send((2, 5));
            book_out
                .assert_yields_unordered([(2, BookResult::AlreadyBooked)])
                .await;
        });
    }

    #[test]
    fn test_invalid_seat() {
        let mut flow = FlowBuilder::new();
        let process = flow.process();

        let (book_in, book_requests) = process.sim_input();
        let book_requests = book_requests.into_keyed();
        let (_get_in, get_requests) = process.sim_input();
        let get_requests = get_requests.into_keyed();

        let (book_acks, _get_responses) = ticketmaster_service(10, book_requests, get_requests);
        let book_out = book_acks.entries().sim_output();

        flow.sim().exhaustive(async || {
            book_in.send((1, 0));
            book_out
                .assert_yields_unordered([(1, BookResult::SeatNotFound)])
                .await;

            book_in.send((2, 11));
            book_out
                .assert_yields_unordered([(2, BookResult::SeatNotFound)])
                .await;
        });
    }

    #[test]
    fn test_get_available_seats() {
        let mut flow = FlowBuilder::new();
        let process = flow.process();

        let (book_in, book_requests) = process.sim_input();
        let book_requests = book_requests.into_keyed();
        let (get_in, get_requests) = process.sim_input();
        let get_requests = get_requests.into_keyed();

        let (book_acks, get_responses) = ticketmaster_service(5, book_requests, get_requests);
        let book_out = book_acks.entries().sim_output();
        let get_out = get_responses.entries().sim_output();

        flow.sim().exhaustive(async || {
            book_in.send((1, 2));
            book_out
                .assert_yields_unordered([(1, BookResult::Ok(2))])
                .await;
            book_in.send((2, 4));
            book_out
                .assert_yields_unordered([(2, BookResult::Ok(4))])
                .await;

            get_in.send((3, ()));
            get_out
                .assert_yields_only_unordered([(3, vec![1, 3, 5])])
                .await;
        });
    }
}
