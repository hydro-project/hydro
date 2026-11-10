//! `capitalize_unordered`'s output is `NoOrder`, so a sink that does not acknowledge that its
//! elements may arrive in any order cannot be wired to it.

use hydro_lang::compile::embedded_runtime::{Acknowledgements, EmbeddedOutputChannel};
use hydro_test_embedded::unordered_output::capitalize_unordered;

fn main() {
    let _outputs = capitalize_unordered::EmbeddedOutputs {
        output: EmbeddedOutputChannel::new(|_: String| {}, Acknowledgements::new().at_least_once()),
    };
}
