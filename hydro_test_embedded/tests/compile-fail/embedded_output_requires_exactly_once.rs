//! `capitalize_unordered`'s output is `AtLeastOnce`, so a sink that requires each element
//! exactly once cannot be wired to it.

use hydro_lang::compile::embedded_runtime::OutputChannel;
use hydro_test_embedded::unordered_output::capitalize_unordered;

fn main() {
    let _outputs = capitalize_unordered::EmbeddedOutputs {
        output: OutputChannel::new(|_: String| {}).require_exactly_once(),
    };
}
