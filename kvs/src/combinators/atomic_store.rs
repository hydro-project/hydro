//! Single-location read-after-write scaffolding for stateful services.

use hydro_lang::live_collections::stream::NoOrder;
use hydro_lang::location::tick::{Atomic, Tick};
use hydro_lang::location::{Location, TopLevel};
use hydro_lang::prelude::*;

/// Builds a store on a single location that guarantees read-after-write: a
/// write's acknowledgement is released only after the write has been applied to
/// local state, and reads observe a snapshot consistent with all acknowledged
/// writes.
///
/// This factors out the atomic write/read *scaffolding* (`.atomic()` → fold into
/// state → `.end_atomic()` ack + `use::atomic` snapshot read) from the policy of
/// *what the state is*, *how it merges*, and *how reads are served* — all of
/// which the caller supplies as two dataflow-building closures. Keeping the `q!`
/// closures at the call site (inside `build_store`/`serve`) is what lets this be
/// generic over the state and message types.
///
/// The store is deliberately **not** key-value aware: `State` is any single
/// value. A keyed store is just one instantiation where `State` is a map (see
/// `kvs`, which folds into a `HashMap` and looks keys up in the snapshot); a
/// counter, an LWW register, or a lattice work equally well.
///
/// - `writes`: each item is `(ack, write)` — `ack` is an opaque token returned
///   once the write lands (e.g. a request id), `write` is whatever the store
///   folds into its state.
/// - `reads`: each item is an opaque `ReadReq`, echoed to `serve` so it can build
///   a response.
/// - `build_store`: folds the atomic stream of writes into a single `State`.
/// - `serve`: given a bounded batch of read requests and a bounded, atomically
///   consistent snapshot of the state, produces the read responses.
///
/// Returns `(acks, read_responses)`. The guarantee is **single-location only** —
/// it does not span cluster members; cross-machine consistency needs a
/// quorum/replication protocol layered on top.
pub fn atomic_store<'a, L, Ack, Write, State, ReadReq, ReadResp, BuildFn, ServeFn>(
    writes: Stream<(Ack, Write), L, Unbounded, NoOrder>,
    reads: Stream<ReadReq, L, Unbounded, NoOrder>,
    build_store: BuildFn,
    serve: ServeFn,
) -> (
    Stream<Ack, L, Unbounded, NoOrder>,
    Stream<ReadResp, L, Unbounded, NoOrder>,
)
where
    L: Location<'a> + TopLevel<'a>,
    Ack: Clone,
    Write: Clone,
    State: Clone,
    ReadReq: Clone,
    ReadResp: Clone,
    BuildFn: FnOnce(
        Stream<Write, Atomic<L>, Unbounded, NoOrder>,
    ) -> Singleton<State, Atomic<L>, Unbounded>,
    ServeFn: FnOnce(
        Stream<ReadReq, Tick<L::DropConsistency>, Bounded, NoOrder>,
        Singleton<State, Tick<L::DropConsistency>, Bounded>,
    ) -> Stream<ReadResp, Tick<L::DropConsistency>, Bounded, NoOrder>,
{
    // Enter the atomic context: acks released via `end_atomic` won't be visible
    // until the corresponding state update has been applied, and any
    // `use::atomic` snapshot observed after an ack reflects that write.
    let atomic_writes = writes.atomic();

    // Fold the writes into state (caller decides what the state is and how it
    // merges).
    let store = build_store(atomic_writes.clone().map(q!(|(_ack, write)| write)));

    // Release an ack for each write once it has been applied in this context.
    let acks = atomic_writes.map(q!(|(ack, _write)| ack)).end_atomic();

    // Serve reads from a snapshot consistent with acknowledged writes.
    let read_responses = sliced! {
        let request_batch = use(reads, nondet!(
            /// Batch boundaries are not observable: each read is answered
            /// independently against the same snapshot.
        ));
        let store_snapshot = use::atomic(store, nondet!(
            /// Atomicity guarantees this snapshot reflects every acknowledged write.
        ));

        serve(request_batch, store_snapshot)
    };

    // The slice output lands at `L::DropConsistency`; recover `L`, since reads
    // are served consistently with the acknowledged writes in this context.
    let read_responses = read_responses.assert_has_consistency_of::<L>(manual_proof!(
        /// Reads are served from a `use::atomic` snapshot in the same atomic
        /// context the write acks are released from, so they are consistent
        /// with those acknowledged writes.
    ));

    (acks, read_responses)
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use hydro_lang::live_collections::stream::{ExactlyOnce, NoOrder, TotalOrder};
    use hydro_lang::prelude::*;

    use super::atomic_store;

    /// Instantiates the combinator as a keyed grow-only-set store — the state is
    /// a single `HashMap<String, HashSet<String>>`, showing the combinator itself
    /// is not key-value aware (keyedness lives entirely in the closures).
    /// Exercises read-after-write.
    #[test]
    fn read_after_write_map_state() {
        let mut flow = FlowBuilder::new();
        let node = flow.process::<()>();

        // Writes: (req_id, (key, value)). Reads: (req_id, key).
        let (write_send, writes) =
            node.sim_input::<(u64, (String, String)), TotalOrder, ExactlyOnce>();
        let (read_send, reads) = node.sim_input::<(u64, String), TotalOrder, ExactlyOnce>();

        let (acks, reads_out) = atomic_store(
            writes.weaken_ordering::<NoOrder>(),
            reads.weaken_ordering::<NoOrder>(),
            // build_store: fold (key, value) writes into a HashMap<key, set>.
            |writes| {
                writes.fold(
                    q!(|| HashMap::<String, HashSet<String>>::new()),
                    q!(
                        |map, (key, value)| {
                            map.entry(key).or_default().insert(value);
                        },
                        commutative = manual_proof!(/** set insert is commutative */)
                    ),
                )
            },
            // serve: look each read's key up in the map snapshot.
            |requests, snapshot| {
                let map_ref = snapshot.by_ref();
                requests.map(q!(move |(req_id, key)| {
                    let values = map_ref.get(&key).cloned().unwrap_or_default();
                    (req_id, values)
                }))
            },
        );

        let ack_out = acks
            .assume_ordering::<TotalOrder>(nondet!(/** single client, sequential */))
            .sim_output();
        let read_out = reads_out
            .assume_ordering::<TotalOrder>(nondet!(/** single client, sequential */))
            .sim_output();

        flow.sim().fuzz(async || {
            // Write k=a, wait for the ack.
            write_send.send((1, ("k".to_owned(), "a".to_owned())));
            match ack_out.next().await {
                Some(1) => {}
                other => panic!("expected ack 1, got {other:?}"),
            }

            // Read k — must observe the acknowledged write.
            read_send.send((2, "k".to_owned()));
            match read_out.next().await {
                Some((2, values)) => assert_eq!(values, HashSet::from(["a".to_owned()])),
                other => panic!("expected read 2, got {other:?}"),
            }

            // Write a second value under the same key, then read again: the set
            // grows to include both.
            write_send.send((3, ("k".to_owned(), "b".to_owned())));
            match ack_out.next().await {
                Some(3) => {}
                other => panic!("expected ack 3, got {other:?}"),
            }
            read_send.send((4, "k".to_owned()));
            match read_out.next().await {
                Some((4, values)) => {
                    assert_eq!(values, HashSet::from(["a".to_owned(), "b".to_owned()]))
                }
                other => panic!("expected read 4, got {other:?}"),
            }
        });
    }

    /// Instantiates the combinator with a scalar counter state (no keys at all),
    /// demonstrating the store is fully general. Writes increment; reads return
    /// the current count, and must reflect all acknowledged increments.
    #[test]
    fn read_after_write_counter_state() {
        let mut flow = FlowBuilder::new();
        let node = flow.process::<()>();

        // Writes: (req_id, amount). Reads: req_id.
        let (write_send, writes) = node.sim_input::<(u64, u64), TotalOrder, ExactlyOnce>();
        let (read_send, reads) = node.sim_input::<u64, TotalOrder, ExactlyOnce>();

        let (acks, reads_out) = atomic_store(
            writes.weaken_ordering::<NoOrder>(),
            reads.weaken_ordering::<NoOrder>(),
            // build_store: sum the increments into a single u64.
            |amounts| {
                amounts.fold(
                    q!(|| 0u64),
                    q!(
                        |sum, amount| {
                            *sum += amount;
                        },
                        commutative = manual_proof!(/** addition is commutative */)
                    ),
                )
            },
            // serve: pair each read with the current count snapshot.
            |requests, snapshot| {
                let count_ref = snapshot.by_ref();
                requests.map(q!(move |req_id| (req_id, *count_ref)))
            },
        );

        let ack_out = acks
            .assume_ordering::<TotalOrder>(nondet!(/** single client, sequential */))
            .sim_output();
        let read_out = reads_out
            .assume_ordering::<TotalOrder>(nondet!(/** single client, sequential */))
            .sim_output();

        flow.sim().fuzz(async || {
            // Increment by 5, wait for the ack.
            write_send.send((1, 5));
            match ack_out.next().await {
                Some(1) => {}
                other => panic!("expected ack 1, got {other:?}"),
            }

            // Read — must observe the acknowledged increment.
            read_send.send(2);
            match read_out.next().await {
                Some((2, count)) => assert_eq!(count, 5),
                other => panic!("expected read 2 = 5, got {other:?}"),
            }

            // Increment by 3 more, then read: count reflects both.
            write_send.send((3, 3));
            match ack_out.next().await {
                Some(3) => {}
                other => panic!("expected ack 3, got {other:?}"),
            }
            read_send.send(4);
            match read_out.next().await {
                Some((4, count)) => assert_eq!(count, 8),
                other => panic!("expected read 4 = 8, got {other:?}"),
            }
        });
    }
}
