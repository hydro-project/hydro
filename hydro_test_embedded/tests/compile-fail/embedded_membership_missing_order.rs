//! A cluster membership stream must promise each member's events in the order they happened;
//! this host promises only that none is duplicated.

use dfir_rs::futures::stream;
use hydro_lang::compile::embedded_runtime::{InputChannel, Promises};
use hydro_lang::location::MembershipEvent;
use hydro_lang::location::member_id::TaglessMemberId;
use hydro_lang::prelude::manual_proof;
use hydro_test_embedded::o2m_broadcast::o2m_sender;

fn main() {
    let events: Vec<(TaglessMemberId, MembershipEvent)> = Vec::new();
    let _membership = o2m_sender::EmbeddedMembershipStreams {
        o2m_receiver: InputChannel::new(
            stream::iter(events),
            Promises::new().exactly_once(manual_proof!(/** each event appears once in the list */)),
        ),
    };
}
