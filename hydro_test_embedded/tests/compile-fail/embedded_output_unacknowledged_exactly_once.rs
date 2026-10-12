//! `capitalize_unordered`'s output is `AtLeastOnce`, so a sink that does not acknowledge that an
//! element may arrive more than once cannot be wired to it.

use hydro_lang::compile::embedded_runtime::{Acknowledgements, OutputChannel};
use hydro_lang::prelude::nondet;
use hydro_test_embedded::unordered_output::capitalize_unordered;

fn main() {
    let _outputs = capitalize_unordered::EmbeddedOutputs {
        output: OutputChannel::new(
            |_: String| {},
            Acknowledgements::new().no_order(nondet!(/** order does not matter */)),
        ),
    };
}
