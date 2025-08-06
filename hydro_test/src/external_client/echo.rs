use std::time::Duration;

use hydro_lang::keyed_stream::KeyedStream;
use hydro_lang::location::MembershipEvent;
use hydro_lang::*;

pub fn echo_server<'a, P>(
    in_stream: KeyedStream<u64, String, Process<'a, P>, Unbounded, TotalOrder>,
    membership: KeyedStream<u64, MembershipEvent, Process<'a, P>, Unbounded, TotalOrder>,
) -> KeyedStream<u64, String, Process<'a, P>, Unbounded, TotalOrder> {
    let current_connections = membership
        .values()
        .map(q!(|event| {
            match event {
                MembershipEvent::Joined => 1,
                MembershipEvent::Left => -1,
            }
        }))
        .reduce_commutative(q!(|count, delta| *count += delta));

    unsafe {
        current_connections
            .sample_every(q!(Duration::from_secs(1)))
            .for_each(q!(|count| {
                println!("Current connections: {}", count);
            }))
    }

    in_stream.inspect_with_key(q!(|(id, t)| println!(
        "...received request {} from client #{}, echoing back...",
        t, id
    )))
}
