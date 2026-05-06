//! Cleaned-up expansion of the persist_mut push-side borrow conflict.
//! This file reproduces the borrow error without the macro.
//!
//! The issue: `sg_1v1_node_1v1_iter` is mutably borrowed by `op_1v1` (via pull::iter),
//! which flows into `op_2v1` (persist_mut pull-side). After the pivot `.await`,
//! the codegen emits `Iterator::for_each(&mut sg_1v1_node_1v1_iter, drop)` to drain
//! remaining items. But Rust can't prove the first borrow is dead because `op_2v1`
//! is an opaque `impl Pull` type.

#![expect(
    unused,
    reason = "expanded macro reproduction, not all bindings are used"
)]

use dfir_rs::dfir_pipes;
use dfir_rs::util::Persistence::{self, *};
use dfir_rs::util::sparse_vec::SparseVec;

async fn repro() {
    // Prologue (outer scope, persists across ticks)
    let mut sg_1v1_node_1v1_iter = {
        [
            Persist(1usize),
            Persist(2),
            Persist(3),
            Persist(4),
            Delete(2),
        ]
        .into_iter()
    };
    let mut sg_1v1_node_2v1_persistdata: SparseVec<usize> = SparseVec::default();
    let mut sg_1v1_node_6v1_persistdata: SparseVec<usize> = SparseVec::default();

    let (pull_tx, _) = dfir_rs::util::unbounded_channel::<usize>();
    let (push_tx, _) = dfir_rs::util::unbounded_channel::<usize>();

    // Tick closure (runs each tick)
    // --- Subgraph 1 ---
    {
        // Scoped block for pull chain — borrow of iter ends when block ends
        {
            // Pull side: source_iter -> persist_mut
            let op_1v1 = dfir_pipes::pull::iter(&mut sg_1v1_node_1v1_iter);

            // Type guard wrapper (this is what the codegen generates to check types)
            fn type_guard<Item, Input>(
                input: Input,
            ) -> impl dfir_pipes::pull::Pull<
                Item = Item,
                Meta = (),
                CanPend = Input::CanPend,
                CanEnd = Input::CanEnd,
            >
            where
                Input: dfir_pipes::pull::Pull<Item = Item, Meta = ()>,
            {
                input
            }
            let op_1v1 = type_guard(op_1v1);

            let op_2v1 = {
                // persist_mut consumes op_1v1, accumulates, then emits from persistdata
                let fut = dfir_pipes::pull::Pull::for_each(op_1v1, |item| match item {
                    Persist(v) => sg_1v1_node_2v1_persistdata.push(v),
                    Delete(v) => sg_1v1_node_2v1_persistdata.delete(&v),
                });
                fut.await;
                dfir_pipes::pull::iter(sg_1v1_node_2v1_persistdata.iter().cloned())
            };
            let op_2v1 = type_guard(op_2v1);

            // Push side: for_each (push_tx), flat_map -> persist_mut -> for_each (pull_tx)
            let op_7v1 = dfir_pipes::push::for_each(|v: usize| push_tx.send(v).unwrap());
            let op_6v1 = dfir_pipes::push::for_each(|item: Persistence<usize>| match item {
                Persist(v) => sg_1v1_node_6v1_persistdata.push(v),
                Delete(v) => sg_1v1_node_6v1_persistdata.delete(&v),
            });
            let op_5v1 = dfir_pipes::push::flat_map(
                |x: usize| {
                    if x == 3 {
                        vec![Persist(x), Delete(x)]
                    } else {
                        vec![Persist(x)]
                    }
                },
                op_6v1,
            );
            let op_4v1 = dfir_pipes::push::for_each(|v: usize| pull_tx.send(v).unwrap());
            let op_3v1 = dfir_pipes::push::fanout(op_4v1, op_5v1); // tee

            // Pivot: pull -> push
            dfir_pipes::pull::Pull::send_push(op_2v1, op_3v1).await;
        }
        // After the block, the &mut borrow of sg_1v1_node_1v1_iter is provably dead.

        // Cleanup: drain remaining items from source_iter
        (&mut sg_1v1_node_1v1_iter).for_each(drop);
    }
}
