use std::time::Duration;

use hydro_lang::live_collections::stream::TotalOrder;
use hydro_lang::location::MembershipEvent;
use hydro_lang::prelude::*;
use hydro_std::membership::track_membership;

pub fn echo_server<'a, P>(
    in_stream: KeyedStream<u64, String, Process<'a, P>, Unbounded, TotalOrder>,
    membership: KeyedStream<u64, MembershipEvent, Process<'a, P>, Unbounded, TotalOrder>,
) -> KeyedStream<u64, String, Process<'a, P>, Unbounded, TotalOrder> {
    // The once-a-second clock drives the slice; each sample logs a snapshot of the connections.
    let samples = membership
        .location()
        .source_interval(q!(Duration::from_secs(1)));
    sliced! {
        let sample = use::batch(samples, nondet!(/** logging */));
        let conns = use::snapshot(track_membership(membership), nondet!(/** logging */));
        conns
            .filter(q!(|b| *b))
            .key_count()
            .filter_if(sample.first().is_some())
            .into_stream()
    }
    .for_each(q!(|count| {
        println!("Current connections: {}", count);
    }));

    in_stream.inspect_with_key(q!(|(id, t)| println!(
        "...received request {} from client #{}, echoing back...",
        t, id
    )))
}
