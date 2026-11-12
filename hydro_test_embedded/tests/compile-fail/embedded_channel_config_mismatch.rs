//! `messages` is sent with no anomalies declared, but this host acknowledges a channel that
//! may delay messages forever, so its receiving end does not compile: changing a channel's network
//! config fails the host's build until it acknowledges the change.

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
            ChannelConfiguration::new(
                EMBEDDED
                    .never_reorders()
                    .never_duplicates()
                    .may_delay_forever(),
            ),
            Promises::new()
                .total_order(manual_proof!(/** the list keeps its order */))
                .exactly_once(manual_proof!(/** each message appears once in the list */)),
        ),
    };
}
