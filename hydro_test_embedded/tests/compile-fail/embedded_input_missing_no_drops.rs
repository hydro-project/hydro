//! `capitalize`'s input is declared `NoDrop`: the dataflow relies on every element arriving,
//! so a host that does not promise that cannot be wired to it.

use dfir_rs::futures::stream;
use hydro_lang::compile::embedded_runtime::{EmbeddedInputChannel, Promises};
use hydro_lang::prelude::manual_proof;
use hydro_test_embedded::embedded::capitalize;

fn main() {
    let _inputs = capitalize::EmbeddedInputs {
        input: EmbeddedInputChannel::new(
            stream::iter(Vec::<String>::new()),
            Promises::new()
                .total_order(manual_proof!(/** the list keeps its order */))
                .exactly_once(manual_proof!(/** the list holds each element once */)),
        ),
    };
}
