//! An embedded input needs total order and exactly-once delivery; this host promises only
//! exactly-once delivery.

use dfir_rs::futures::stream;
use hydro_lang::compile::embedded_runtime::{InputChannel, Promises};
use hydro_lang::prelude::manual_proof;
use hydro_test_embedded::echo_network_embedded::echo_sender;

fn main() {
    let _inputs = echo_sender::EmbeddedInputs {
        input: InputChannel::new(
            stream::iter(Vec::<String>::new()),
            Promises::new().exactly_once(manual_proof!(/** each element appears once in the list */)),
        ),
    };
}
