//! `capitalize_unordered`'s output is `NoOrder`, so a sink that does not acknowledge that its
//! elements may arrive in any order cannot be wired to it.

use hydro_lang::compile::embedded_runtime::{Acknowledgements, OutputChannel};
use hydro_lang::prelude::nondet;
use hydro_test_embedded::unordered_output::capitalize_unordered;

fn main() {
    let _outputs = capitalize_unordered::EmbeddedOutputs {
        output: OutputChannel::new(
            |_: String| {},
            Acknowledgements::new().at_least_once(nondet!(/** duplicates are harmless */)),
        ),
    };
}
