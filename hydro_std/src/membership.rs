use std::hash::Hash;

use hydro_lang::location::{Location, MembershipEvent};
use hydro_lang::prelude::*;
use stageleft::q;

pub fn track_membership<'a, K: Hash + Eq, L: Location<'a>>(
    membership: KeyedStream<K, MembershipEvent, L, Unbounded>,
) -> KeyedSingleton<K, bool, L, MonotonicKeys> {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that a join sets the member present.
    #[test]
    fn track_membership_join_sets_present() {
        let mut flow = FlowBuilder::new();
        let process = flow.process::<()>();

        let (send, events) = process.sim_input();
        let observed = sliced! {
            let snap = use::snapshot(track_membership(events.into_keyed()), nondet!(
                /** test */
            ));
            snap.entries().sort().collect_vec().into_stream()
        }
        .sim_output();

        flow.sim().exhaustive(async || {
            send.send((1, MembershipEvent::Joined));

            let snaps: Vec<Vec<(u32, bool)>> = observed.collect().await;
            for snap in &snaps {
                for &(key, is_present) in snap {
                    assert_eq!(key, 1);
                    assert!(
                        is_present,
                        "a join must set the member present, saw {snaps:?}"
                    );
                }
            }
            assert_eq!(
                snaps.last().expect("membership snapshot").as_slice(),
                [(1, true)].as_slice()
            );
        });
    }

    /// Test that a join followed by a leave sets the member absent.
    #[test]
    fn track_membership_join_then_leave_sets_absent() {
        let mut flow = FlowBuilder::new();
        let process = flow.process::<()>();

        let (send, events) = process.sim_input();
        let observed = sliced! {
            let snap = use::snapshot(track_membership(events.into_keyed()), nondet!(
                /** test */
            ));
            snap.entries().sort().collect_vec().into_stream()
        }
        .sim_output();

        flow.sim().exhaustive(async || {
            send.send_many([(1, MembershipEvent::Joined), (1, MembershipEvent::Left)]);

            let snaps: Vec<Vec<(u32, bool)>> = observed.collect().await;
            let mut seen_absent = false;
            for snap in &snaps {
                for &(key, is_present) in snap {
                    assert_eq!(key, 1);
                    if is_present {
                        assert!(
                            !seen_absent,
                            "member was present after a leave, saw {snaps:?}"
                        );
                    } else {
                        seen_absent = true;
                    }
                }
            }
            assert_eq!(
                snaps.last().expect("membership snapshot").as_slice(),
                [(1, false)].as_slice()
            );
        });
    }

    /// Test that two keys update independently.
    #[test]
    fn track_membership_keys_update_independently() {
        let mut flow = FlowBuilder::new();
        let process = flow.process::<()>();

        let (send, events) = process.sim_input();
        let observed = sliced! {
            let snap = use::snapshot(track_membership(events.into_keyed()), nondet!(
                /** test */
            ));
            snap.entries().sort().collect_vec().into_stream()
        }
        .sim_output();

        flow.sim().exhaustive(async || {
            send.send_many([
                (1, MembershipEvent::Joined),
                (2, MembershipEvent::Joined),
                (2, MembershipEvent::Left),
            ]);

            let snaps: Vec<Vec<(u32, bool)>> = observed.collect().await;
            let mut key_two_absent = false;
            for snap in &snaps {
                for &(key, is_present) in snap {
                    match key {
                        1 => assert!(
                            is_present,
                            "key 1 must stay present while key 2 changes, saw {snaps:?}"
                        ),
                        2 => {
                            if is_present {
                                assert!(
                                    !key_two_absent,
                                    "key 2 was present after a leave, saw {snaps:?}"
                                );
                            } else {
                                key_two_absent = true;
                            }
                        }
                        _ => panic!("unexpected key {key} in {snaps:?}"),
                    }
                }
            }
            assert_eq!(
                snaps.last().expect("membership snapshot").as_slice(),
                [(1, true), (2, false)].as_slice()
            );
        });
    }
}
