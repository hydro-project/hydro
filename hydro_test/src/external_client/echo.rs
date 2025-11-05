use std::time::Duration;

use hydro_lang::live_collections::stream::TotalOrder;
use hydro_lang::location::MembershipEvent;
use hydro_lang::prelude::*;
use hydro_std::membership::track_membership;

pub fn echo_server<'a, P>(
    in_stream: KeyedStream<u64, String, Process<'a, P>, Unbounded, TotalOrder>,
    membership: KeyedStream<u64, MembershipEvent, Process<'a, P>, Unbounded, TotalOrder>,
) -> KeyedStream<u64, String, Process<'a, P>, Unbounded, TotalOrder> {
    sliced!(|
        let conns = use(track_membership(membership), nondet!(/** logging */));
    | {
        conns.filter(q!(|b| *b)).key_count()
    })
    .sample_every(q!(Duration::from_secs(1)), nondet!(/** logging */))
    .assume_retries(nondet!(/** extra logs due to duplicate samples are okay */))
    .for_each(q!(|count| {
        println!("Current connections: {}", count);
    }));

    in_stream.inspect_with_key(q!(|(id, t)| println!(
        "...received request {} from client #{}, echoing back...",
        t, id
    )))
}
