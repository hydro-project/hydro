use std::collections::HashSet;

use dfir_rs::scheduled::graph::Dfir;
use dfir_rs::util::collect_ready;
use dfir_rs::{assert_graphvis_snapshots, dfir_syntax};
use lattices::set_union::{CartesianProductBimorphism, SetUnionHashSet, SetUnionSingletonSet};
use multiplatform_test::multiplatform_test;
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::wrappers::UnboundedReceiverStream;

/// Check that the following tests all behave the same.
fn check_cartesian_product_multi_tick(
    mut df: Dfir,
    lhs_send: UnboundedSender<u32>,
    rhs_send: UnboundedSender<u32>,
    mut out_recv: futures::stream::Fuse<UnboundedReceiverStream<SetUnionHashSet<(u32, u32)>>>,
) {
    df.run_available_sync();
    assert_eq!(0, collect_ready::<Vec<_>, _>(&mut out_recv).len());

    for x in 0..3 {
        lhs_send.send(x).unwrap();
    }
    for x in 3..5 {
        rhs_send.send(x).unwrap();
    }
    df.run_available_sync();
    assert_eq!(
        &[SetUnionHashSet::new(HashSet::from_iter([
            (0, 3),
            (0, 4),
            (1, 3),
            (1, 4),
            (2, 3),
            (2, 4),
        ]))],
        &*collect_ready::<Vec<_>, _>(&mut out_recv)
    );

    df.run_available_sync();
    assert_eq!(0, collect_ready::<Vec<_>, _>(&mut out_recv).len());
}

#[multiplatform_test]
pub fn test_cartesian_product_multi_tick() {
    let (lhs_send, lhs_recv) = dfir_rs::util::unbounded_channel::<_>();
    let (rhs_send, rhs_recv) = dfir_rs::util::unbounded_channel::<_>();
    let (out_send, out_recv) = dfir_rs::util::unbounded_channel::<_>();

    let df = dfir_syntax! {
        lhs = source_stream(lhs_recv)
            -> map(SetUnionSingletonSet::new_from)
            -> state::<'static, SetUnionHashSet<u32>>();
        rhs = source_stream(rhs_recv)
            -> map(SetUnionSingletonSet::new_from)
            -> state::<'static, SetUnionHashSet<u32>>();

        lhs -> [0]my_join;
        rhs -> [1]my_join;

        my_join = lattice_bimorphism(CartesianProductBimorphism::<HashSet<_>>::default(), #lhs, #rhs)
            -> lattice_reduce()
            -> for_each(|x| out_send.send(x).unwrap());
    };

    assert_graphvis_snapshots!(df);
    check_cartesian_product_multi_tick(df, lhs_send, rhs_send, out_recv);
}

#[multiplatform_test]
pub fn test_cartesian_product_multi_tick_tee() {
    let (lhs_send, lhs_recv) = dfir_rs::util::unbounded_channel::<_>();
    let (rhs_send, rhs_recv) = dfir_rs::util::unbounded_channel::<_>();
    let (out_send, out_recv) = dfir_rs::util::unbounded_channel::<_>();

    let df = dfir_syntax! {
        lhs = source_stream(lhs_recv)
            -> map(SetUnionSingletonSet::new_from)
            -> state::<'static, SetUnionHashSet<u32>>();
        rhs = source_stream(rhs_recv)
            -> map(SetUnionSingletonSet::new_from)
            -> state::<'static, SetUnionHashSet<u32>>();
        rhs_tee = rhs -> tee();
        rhs_tee -> for_each(|x| println!("tee: {:?}", x));

        lhs -> [0]my_join;
        rhs_tee -> [1]my_join;

        my_join = lattice_bimorphism(CartesianProductBimorphism::<HashSet<_>>::default(), #lhs, #rhs)
            -> lattice_reduce()
            -> for_each(|x| out_send.send(x).unwrap());
    };

    assert_graphvis_snapshots!(df);
    check_cartesian_product_multi_tick(df, lhs_send, rhs_send, out_recv);
}

#[multiplatform_test]
pub fn test_cartesian_product_multi_tick_identity() {
    let (lhs_send, lhs_recv) = dfir_rs::util::unbounded_channel::<_>();
    let (rhs_send, rhs_recv) = dfir_rs::util::unbounded_channel::<_>();
    let (out_send, out_recv) = dfir_rs::util::unbounded_channel::<_>();

    let df = dfir_syntax! {
        lhs = source_stream(lhs_recv)
            -> map(SetUnionSingletonSet::new_from)
            -> state::<'static, SetUnionHashSet<u32>>();
        rhs = source_stream(rhs_recv)
            -> map(SetUnionSingletonSet::new_from)
            -> state::<'static, SetUnionHashSet<u32>>();
        rhs_id = rhs -> identity();

        lhs -> [0]my_join;
        rhs_id -> [1]my_join;

        my_join = lattice_bimorphism(CartesianProductBimorphism::<HashSet<_>>::default(), #lhs, #rhs)
            -> lattice_reduce()
            -> for_each(|x| out_send.send(x).unwrap());
    };

    assert_graphvis_snapshots!(df);
    check_cartesian_product_multi_tick(df, lhs_send, rhs_send, out_recv);
}
