use hydro_lang::prelude::*;

pub struct Sender {}
pub struct Receiver {}

/// Like [`super::echo_network::echo_network`], but uses an `EMBEDDED` channel, so that the
/// generated network channel exposes the raw `String` payload (rather than serialized bytes) to
/// the host, which then moves (and, if it needs to, serializes) it outside of Hydro.
pub fn echo_network_embedded<'a>(
    receiver: &Process<'a, Receiver>,
    input: Stream<String, Process<'a, Sender>>,
) -> Stream<String, Process<'a, Receiver>> {
    input
        .send(receiver, EMBEDDED.fail_stop().name("messages"))
        .map(q!(|s| s.to_uppercase()))
}
