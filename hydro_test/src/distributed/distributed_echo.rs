use hydro_lang::live_collections::stream::TotalOrder;
use hydro_lang::location::NetworkHint;
use hydro_lang::location::external_process::{ExternalBytesPort, Many};
use hydro_lang::prelude::*;
use tokio_util::codec::LengthDelimitedCodec;

pub struct P1 {}
pub struct C2 {}
pub struct C3 {}
pub struct P4 {}

pub fn distributed_echo<'a>(
    external: &External<'a, ()>,
    p1: &Process<'a, P1>,
    c2: &Cluster<'a, C2>,
    c3: &Cluster<'a, C3>,
    p4: &Process<'a, P4>,
) -> ExternalBytesPort<Many> {
    let (bidi_port, raw_input, _external_membership, response_sink) = p1
        .bidi_external_many_bytes::<_, bytes::Bytes, LengthDelimitedCodec>(
            external,
            NetworkHint::Auto,
        );

    // Parse incoming JSON bytes to u32
    let input = raw_input.filter_map(q!(|data: bytes::BytesMut| {
        let json_str = String::from_utf8_lossy(&data);
        match serde_json::from_str::<u32>(&json_str) {
            Ok(n) => Some(n),
            Err(e) => {
                println!("[P1] Failed to parse JSON input '{}': {}", json_str, e);
                None
            }
        }
    }));

    let echo_result = input
        .inspect_with_key(q!(|(client_id, n)| println!(
            "[P1] received from external client {client_id}: {n}"
        )))
        // Convert keyed stream to stream of tuples, incrementing the value
        .entries()
        .map(q!(|(client_id, n)| (client_id, n + 1)))
        .assume_ordering::<TotalOrder>(nondet!(/** external input order */))
        .round_robin(c2, TCP.fail_stop().bincode(), nondet!(/** test */))
        .inspect(q!(|(client_id, n)| println!(
            "[C2] received from client {client_id}: {n}"
        )))
        .map(q!(|(client_id, n)| (client_id, n + 1)))
        .round_robin(c3, TCP.fail_stop().bincode(), nondet!(/** test */))
        .inspect(q!(|(client_id, n)| println!(
            "[C3] received from client {client_id}: {n}"
        )))
        .map(q!(|(client_id, n)| (client_id, n + 1)))
        .values()
        .send(p4, TCP.fail_stop().bincode())
        .inspect(q!(|(client_id, n)| println!(
            "[P4] received from client {client_id}: {n}"
        )))
        .map(q!(|(client_id, n)| (client_id, n + 1)))
        .values()
        .send(p1, TCP.fail_stop().bincode());

    let tick = p1.tick();

    let all_responses = echo_result
        .batch(&tick, nondet!(/** test */))
        .map(q!(|(client_id, n)| {
            let json = serde_json::to_string(&n).unwrap();
            (client_id, bytes::Bytes::from(json))
        }))
        .into_keyed()
        .all_ticks();

    response_sink.complete(all_responses);

    bidi_port
}
