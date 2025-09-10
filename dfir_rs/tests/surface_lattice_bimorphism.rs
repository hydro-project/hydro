use std::collections::{HashMap, HashSet};

use dfir_rs::util::collect_ready;
use dfir_rs::{assert_graphvis_snapshots, dfir_syntax};
use lattices::GhtType;
use lattices::ght::GeneralizedHashTrieNode;
use lattices::ght::lattice::{DeepJoinLatticeBimorphism, GhtBimorphism};
use lattices::map_union::{KeyedBimorphism, MapUnionHashMap, MapUnionSingletonMap};
use lattices::set_union::{CartesianProductBimorphism, SetUnionHashSet, SetUnionSingletonSet};
use multiplatform_test::multiplatform_test;
use variadics::CloneVariadic;
use variadics::variadic_collections::VariadicHashSet;

#[multiplatform_test]
pub fn test_cartesian_product() {
    let (out_send, out_recv) = dfir_rs::util::unbounded_channel::<_>();

    let mut df = dfir_syntax! {
        lhs = source_iter(0..3)
            -> map(SetUnionSingletonSet::new_from)
            -> state::<'static, SetUnionHashSet<u32>>();
        rhs = source_iter(3..5)
            -> map(SetUnionSingletonSet::new_from)
            -> state::<'static, SetUnionHashSet<u32>>();

        lhs -> [0]my_join;
        rhs -> [1]my_join;

        my_join = lattice_bimorphism(CartesianProductBimorphism::<HashSet<_>>::default(), #lhs, #rhs)
            -> for_each(|x| out_send.send(x).unwrap());
    };

    assert_graphvis_snapshots!(df);
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
        &*collect_ready::<Vec<_>, _>(out_recv)
    );
}

#[multiplatform_test(test, wasm, env_tracing)]
pub fn test_cartesian_product_1401() {
    let (out_send, out_recv) = dfir_rs::util::unbounded_channel::<_>();

    let mut df = dfir_syntax! {
        lhs = source_iter(0..1)
            -> map(SetUnionSingletonSet::new_from)
            -> state::<'static, SetUnionHashSet<u32>>();
        rhs = source_iter(1..2)
            -> map(SetUnionSingletonSet::new_from)
            -> state::<'static, SetUnionHashSet<u32>>();

        lhs -> [0]my_join;
        rhs -> [1]my_join;

        my_join = lattice_bimorphism(CartesianProductBimorphism::<HashSet<_>>::default(), #lhs, #rhs)
            -> for_each(|x| out_send.send(x).unwrap());
    };
    assert_graphvis_snapshots!(df);
    df.run_available_sync();

    assert_eq!(
        &[SetUnionHashSet::new(HashSet::from_iter([(0, 1)]))],
        &*collect_ready::<Vec<_>, _>(out_recv)
    );
}

#[multiplatform_test]
pub fn test_join() {
    let (out_send, out_recv) = dfir_rs::util::unbounded_channel::<_>();

    let mut df = dfir_syntax! {
        lhs = source_iter([(7, 1), (7, 2)])
            -> map(|(k, v)| MapUnionSingletonMap::new_from((k, SetUnionSingletonSet::new_from(v))))
            -> state::<'static, MapUnionHashMap<usize, SetUnionHashSet<usize>>>();
        rhs = source_iter([(7, 0), (7, 1), (7, 2)])
            -> map(|(k, v)| MapUnionSingletonMap::new_from((k, SetUnionSingletonSet::new_from(v))))
            -> state::<'static, MapUnionHashMap<usize, SetUnionHashSet<usize>>>();

        lhs -> [0]my_join;
        rhs -> [1]my_join;

        my_join = lattice_bimorphism(KeyedBimorphism::<HashMap<_, _>, _>::new(CartesianProductBimorphism::<HashSet<_>>::default()), #lhs, #rhs)
            -> for_each(|x| out_send.send(x).unwrap());
    };

    assert_graphvis_snapshots!(df);
    df.run_available_sync();

    assert_eq!(
        &[MapUnionHashMap::new(HashMap::from_iter([(
            7,
            SetUnionHashSet::new(HashSet::from_iter([
                (1, 0),
                (1, 1),
                (1, 2),
                (2, 0),
                (2, 1),
                (2, 2),
            ]))
        )]))],
        &*collect_ready::<Vec<_>, _>(out_recv)
    );
}

/// Test for https://github.com/hydro-project/hydro/issues/1298
#[multiplatform_test]
pub fn test_cartesian_product_tick_state() {
    let (lhs_send, lhs_recv) = dfir_rs::util::unbounded_channel::<u32>();
    let (rhs_send, rhs_recv) = dfir_rs::util::unbounded_channel::<u32>();
    let (out_send, mut out_recv) = dfir_rs::util::unbounded_channel::<_>();

    let mut df = dfir_syntax! {
        lhs = source_stream(lhs_recv)
            -> map(SetUnionSingletonSet::new_from)
            -> state::<'tick, SetUnionHashSet<u32>>();
        rhs = source_stream(rhs_recv)
            -> map(SetUnionSingletonSet::new_from)
            -> state::<'tick, SetUnionHashSet<u32>>();

        lhs[items] -> [0]my_join;
        rhs[items] -> [1]my_join;

        my_join = lattice_bimorphism(CartesianProductBimorphism::<HashSet<_>>::default(), #lhs, #rhs)
            -> inspect(|x| println!("{:?}: {:?}", context.current_tick(), x))
            -> for_each(|x| out_send.send(x).unwrap());
    };
    assert_graphvis_snapshots!(df);

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

    for x in 3..5 {
        lhs_send.send(x).unwrap();
    }
    df.run_available_sync();
    assert_eq!(
        &[SetUnionHashSet::default()],
        &*collect_ready::<Vec<_>, _>(&mut out_recv)
    );
}

#[multiplatform_test]
fn test_ght_join_bimorphism() {
    type MyGhtATrie = GhtType!(u32, u64, u16 => &'static str: VariadicHashSet);
    type MyGhtBTrie = GhtType!(u32, u64, u16 => &'static str: VariadicHashSet);

    type JoinSchema = variadics::var_type!(u32, u64, u16, &'static str, &'static str);

    type MyNodeBim = <(MyGhtATrie, MyGhtBTrie) as DeepJoinLatticeBimorphism<
        VariadicHashSet<JoinSchema>,
    >>::DeepJoinLatticeBimorphism;
    type MyBim = GhtBimorphism<MyNodeBim>;

    let mut hf = dfir_syntax! {
        lhs = source_iter([
                var_expr!(123, 2, 5, "hello"),
                var_expr!(50, 1, 1, "hi"),
                var_expr!(5, 1, 7, "hi"),
                var_expr!(5, 1, 7, "bye"),
            ])
            -> map(|row| MyGhtATrie::new_from([row]))
            -> state::<'tick, MyGhtATrie>();
        rhs = source_iter([
                var_expr!(5, 1, 8, "hi"),
                var_expr!(5, 1, 7, "world"),
                var_expr!(5, 1, 7, "folks"),
                var_expr!(10, 1, 2, "hi"),
                var_expr!(12, 10, 98, "bye"),
            ])
            -> map(|row| MyGhtBTrie::new_from([row]))
            -> state::<'tick, MyGhtBTrie>();

        lhs[items] -> [0]my_join;
        rhs[items] -> [1]my_join;


        my_join = lattice_bimorphism(MyBim::default(), #lhs, #rhs)
            -> enumerate()
            -> inspect(|x| println!("{:?} {:#?}", context.current_tick(), x))
            -> flat_map(|(_num, ght)| ght.recursive_iter().map(<JoinSchema as CloneVariadic>::clone_ref_var).collect::<Vec<_>>())
            -> null();
    };
    hf.run_available_sync();
}
