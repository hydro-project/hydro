use hydro_lang::prelude::*;

pub struct Sender {}
pub struct Receiver {}

pub fn echo_network<'a>(
    receiver: &Process<'a, Receiver>,
    input: Stream<String, Process<'a, Sender>>,
) -> Stream<String, Process<'a, Receiver>> {
    input
        .send(receiver, TCP.fail_stop().bincode().name("messages"))
        .map(q!(|s| s.to_uppercase()))
}
