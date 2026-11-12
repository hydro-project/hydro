//! The dataflow sends an unordered stream on `messages`, but this host acknowledges nothing about
//! it, so its sending end does not compile: like an output, the stream sent on a channel must
//! have every way it may be weaker than ordered and exactly-once acknowledged.

use hydro_lang::compile::embedded_runtime::{
    Acknowledgements, ChannelConfiguration, EmbeddedNetworkOutputChannel,
};
use hydro_lang::networking::EMBEDDED;
use hydro_test_embedded::weak_stream_channel::weak_stream_sender;

fn main() {
    let _net_out = weak_stream_sender::EmbeddedNetworkOut {
        messages: EmbeddedNetworkOutputChannel::new(
            |_: String| {},
            ChannelConfiguration::new(EMBEDDED.never_reorders().never_duplicates().never_drops()),
            Acknowledgements::new().at_least_once(),
        ),
    };
}
