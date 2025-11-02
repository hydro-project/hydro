use std::hash::Hash;

use hydro_lang::location::{Location, MembershipEvent};
use hydro_lang::prelude::*;
use stageleft::q;

pub fn track_membership<'a, K: Hash + Eq, L: Location<'a>>(
    membership: KeyedStream<K, MembershipEvent, L, Unbounded>,
) -> KeyedSingleton<K, bool, L, Unbounded> {
    membership.fold(
        q!(|| false),
        q!(|present, event| {
            match event {
                MembershipEvent::Joined => *present = true,
                MembershipEvent::Left => *present = false,
            }
        }),
    )
}
