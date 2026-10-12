//! `bounded_echo` assumes its input is complete from the start; this host promises its order and
//! exactly-once delivery, but not that.

use dfir_rs::futures::stream;
use hydro_lang::compile::embedded_runtime::{InputChannel, Promises};
use hydro_lang::prelude::manual_proof;
use hydro_test_embedded::bounded_output::bounded_echo;

fn main() {
    let _inputs = bounded_echo::EmbeddedInputs {
        input: InputChannel::new(
            stream::iter(Vec::<String>::new()),
            Promises::new()
                .total_order(manual_proof!(/** an in-memory list keeps its order */))
                .exactly_once(manual_proof!(/** each element appears once in the list */)),
        ),
    };
}
