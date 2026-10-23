//! An `InputChannel` built with no promises meets none of this channel's requirements: total
//! order from `EMBEDDED.fail_stop()`, and exactly-once delivery for the stream sent.

use dfir_rs::futures::stream;
use hydro_lang::compile::embedded_runtime::{InputChannel, Promises};
use hydro_test_embedded::echo_network_embedded::echo_receiver;

fn main() {
    let _net_in = echo_receiver::EmbeddedNetworkIn {
        messages: InputChannel::new(stream::iter(Vec::<String>::new()), Promises::new()),
    };
}
