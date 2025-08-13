use std::time::Duration;

use hydro_lang::keyed_stream::KeyedStream;
use hydro_lang::location::MembershipEvent;
use hydro_lang::*;
use hydro_std::membership::track_membership;

pub fn echo_server<'a, P>(
    in_stream: KeyedStream<u64, String, Process<'a, P>, Unbounded, TotalOrder>,
    membership: KeyedStream<u64, MembershipEvent, Process<'a, P>, Unbounded, TotalOrder>,
) -> KeyedStream<u64, String, Process<'a, P>, Unbounded, TotalOrder> {
    let current_connections = track_membership(membership);

    current_connections
        .key_count()
        .sample_every(q!(Duration::from_secs(1)), nondet!(/** logging */))
        .for_each(q!(|count| {
            println!("Current connections: {}", count);
        }));

    in_stream.inspect_with_key(q!(|(id, t)| println!(
        "...received request {} from client #{}, echoing back...",
        t, id
    )))
}
