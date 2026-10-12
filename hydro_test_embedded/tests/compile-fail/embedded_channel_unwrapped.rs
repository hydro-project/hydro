//! A network field takes an `InputChannel`, not a bare stream: the host must state its promises.

use dfir_rs::futures::stream;
use hydro_test_embedded::echo_network_embedded::echo_receiver;

fn main() {
    let _net_in = echo_receiver::EmbeddedNetworkIn {
        messages: stream::iter(Vec::<String>::new()),
    };
}
