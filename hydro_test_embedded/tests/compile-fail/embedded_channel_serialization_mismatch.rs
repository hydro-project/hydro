//! `messages` is sent with `TCP.fail_stop().bincode()`, but this host acknowledges the config
//! without its serialization, so its sending end does not compile.

use dfir_rs::bytes::Bytes;
use hydro_lang::compile::embedded_runtime::{Acknowledgements, ChannelConfiguration, SendChannel};
use hydro_lang::networking::TCP;
use hydro_test_embedded::echo_network::echo_sender;

fn main() {
    let _net_out = echo_sender::EmbeddedNetworkOut {
        messages: SendChannel::new(
            |_: Bytes| {},
            ChannelConfiguration::new(TCP.fail_stop()),
            Acknowledgements::new(),
        ),
    };
}
