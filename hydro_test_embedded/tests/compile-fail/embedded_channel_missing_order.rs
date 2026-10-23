//! `messages` is declared `EMBEDDED.fail_stop()` and carries an exactly-once stream, so its
//! receiving end needs total order and exactly-once delivery; this host promises only
//! exactly-once delivery.

use dfir_rs::futures::stream;
use hydro_lang::compile::embedded_runtime::{InputChannel, Promises};
use hydro_lang::prelude::manual_proof;
use hydro_test_embedded::echo_network_embedded::echo_receiver;

fn main() {
    let _net_in = echo_receiver::EmbeddedNetworkIn {
        messages: InputChannel::new(
            stream::iter(Vec::<String>::new()),
            Promises::new()
                .exactly_once(manual_proof!(/** each message appears once in the list */)),
        ),
    };
}
