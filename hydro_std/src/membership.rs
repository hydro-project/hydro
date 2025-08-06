use hydro_lang::keyed_optional::KeyedOptional;
use hydro_lang::keyed_stream::KeyedStream;
use hydro_lang::location::MembershipEvent;
use hydro_lang::{Location, Unbounded};
use stageleft::q;

pub fn track_membership<'a, L: Location<'a>>(
    membership: KeyedStream<u64, MembershipEvent, L, Unbounded>,
) -> KeyedOptional<u64, (), L, Unbounded> {
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
