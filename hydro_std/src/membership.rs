use std::hash::Hash;

use hydro_lang::boundedness::Unbounded;
use hydro_lang::live_collections::keyed_singleton::KeyedSingleton;
use hydro_lang::live_collections::keyed_stream::KeyedStream;
use hydro_lang::location::{Location, MembershipEvent};
use stageleft::q;

pub fn track_membership<'a, K: Hash + Eq, L: Location<'a>>(
    membership: KeyedStream<K, MembershipEvent, L, Unbounded>,
) -> KeyedSingleton<K, (), L, Unbounded> {
    membership
        .fold(
            q!(|| false),
            q!(|present, event| {
                match event {
                    MembershipEvent::Joined => *present = true,
                    MembershipEvent::Left => *present = false,
                }
            }),
        )
        .filter_map(q!(|v| if v { Some(()) } else { None }))
}
