//! `lossy_receiver`'s output is the stream received over a lossy channel, which may be missing
//! messages, so a sink that does not acknowledge that cannot be wired to it.

use hydro_lang::compile::embedded_runtime::{Acknowledgements, EmbeddedOutputChannel};
use hydro_test_embedded::lossy_channel::lossy_receiver;

fn main() {
    let _outputs = lossy_receiver::EmbeddedOutputs {
        output: EmbeddedOutputChannel::new(|_: String| {}, Acknowledgements::new()),
        hints_out: EmbeddedOutputChannel::new(|_: String| {}, Acknowledgements::new()),
    };
}
