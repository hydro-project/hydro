//! `messages` is declared with no anomalies and carries an exactly-once stream, so it
//! needs total order and exactly-once delivery; this host promises only total order.

use dfir_rs::futures::stream;
use hydro_lang::compile::embedded_runtime::{
    ChannelConfiguration, EmbeddedNetworkInputChannel, Promises,
};
use hydro_lang::networking::EMBEDDED;
use hydro_lang::prelude::manual_proof;
use hydro_test_embedded::echo_network_embedded::echo_receiver;

fn main() {
    let _net_in = echo_receiver::EmbeddedNetworkIn {
        messages: EmbeddedNetworkInputChannel::new(
            stream::iter(Vec::<String>::new()),
            ChannelConfiguration::new(EMBEDDED.never_reorders().never_duplicates().never_drops()),
            Promises::new().total_order(manual_proof!(/** an in-memory list keeps its order */)),
        ),
    };
}
