//! `unordered_echo`'s output is `Unbounded`, so a sink that requires a bounded stream cannot be
//! wired to it.

use hydro_lang::compile::embedded_runtime::OutputChannel;
use hydro_test_embedded::unordered_output::unordered_echo;

fn main() {
    let _outputs = unordered_echo::EmbeddedOutputs {
        output: OutputChannel::new(|_: String| {}).require_bounded(),
    };
}
