use hydro_lang::prelude::*;

pub struct Sender {}
pub struct Receiver {}

/// Like [`super::echo_network::echo_network`], but uses `.embedded()` serialization so that the
/// generated network channel exposes the raw `String` payload (rather than serialized bytes) to
/// the developer, who is then responsible for serializing it outside of Hydro.
pub fn echo_network_embedded<'a>(
    receiver: &Process<'a, Receiver>,
    input: Stream<String, Process<'a, Sender>>,
) -> Stream<String, Process<'a, Receiver>> {
    input
        .send(receiver, TCP.fail_stop().embedded().name("messages"))
        .map(q!(|s| s.to_uppercase()))
}
