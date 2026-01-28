//! Tests for the GHT code
#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    #[test]
    fn basic_test() {
        use variadics::var_expr;

        use crate::GhtType;
        use crate::ght::GeneralizedHashTrieNode;

        // Example usage
        type MyTrie1 = GhtType!(u32, u32 => &'static str: VariadicCountedHashSet);

        fn ght_type<T: GeneralizedHashTrieNode>() {}
        ght_type::<MyTrie1>();

        let htrie1 = MyTrie1::new_from(vec![var_expr!(42, 314, "hello")]);
        assert!(htrie1.contains(var_expr!(&42, &314, &"hello")));
        assert_eq!(htrie1.recursive_iter().count(), 1);

        type MyTrie2 = GhtType!(u32 => u32: VariadicCountedHashSet);
        let htrie2 = MyTrie2::new_from(vec![var_expr!(42, 314)]);
        assert!(htrie2.contains(var_expr!(&42, &314)));
        assert_eq!(htrie1.recursive_iter().count(), 1);

        type MyTrie3 = GhtType!(u32, u64, u16 => &'static str: VariadicCountedHashSet);
        let htrie3 = MyTrie3::new_from(vec![
            var_expr!(123, 2, 5, "hello"),
            var_expr!(50, 1, 1, "hi"),
            var_expr!(5, 1, 7, "hi"),
            var_expr!(5, 1, 7, "bye"),
        ]);
        assert!(htrie3.contains(var_expr!(&50, &1, &1, &"hi")));
        assert_eq!(htrie3.recursive_iter().count(), 4);
    }
    #[test]
    fn test_ght_node_type_macro() {
        use variadics::var_expr;

        use crate::GhtType;
        use crate::ght::GeneralizedHashTrieNode;

        // 0 => 1
        type LilTrie = GhtType!(() => u32: VariadicCountedHashSet);
        let _j = LilTrie::default();
        let _l = LilTrie::new_from(vec![var_expr!(1)]);

        // 0 => >1
        type LilTrie2 = GhtType!(() => u32, u64: VariadicCountedHashSet);
        let _l = LilTrie2::default();
        let _l = LilTrie2::new_from(vec![var_expr!(1, 1)]);

        // 1 => 0
        type KeyNoValTrie = GhtType!(u32 => (): VariadicCountedHashSet);
        let l = KeyNoValTrie::new_from(vec![var_expr!(1)]);
        let _: KeyNoValTrie = l;

        // 1 => 1
        type SmallTrie = GhtType!(u32 => &'static str: VariadicCountedHashSet);
        type SmallKeyedTrie = GhtType!(u32 => &'static str: VariadicCountedHashSet);
        let l = SmallTrie::new_from(vec![var_expr!(1, "hello")]);
        let _: SmallKeyedTrie = l;

        // 1 => >1
        type SmallKeyLongValTrie = GhtType!(u32 => u64, u16, &'static str: VariadicCountedHashSet);
        let _x = SmallKeyLongValTrie::new_from(vec![var_expr!(1, 999, 222, "hello")]);

        // >1 => 0
        type LongKeyNoValTrie = GhtType!(u32, u64 => (): VariadicCountedHashSet);
        let l = LongKeyNoValTrie::new_from(vec![var_expr!(1, 999)]);
        let _: LongKeyNoValTrie = l;

        // >1 => 1
        type LongKeySmallValTrie = GhtType!(u32, u16 => &'static str: VariadicCountedHashSet);
        type LongKeySmallValKeyedTrie = GhtType!(u32, u16 => &'static str: VariadicCountedHashSet);
        let x = LongKeySmallValTrie::new_from(vec![var_expr!(1, 314, "hello")]);
        let _: LongKeySmallValKeyedTrie = x;
        let _ = LongKeySmallValTrie::new_from(vec![var_expr!(1, 314, "hello")]);

        // >1 => >1
        type LongKeyLongValTrie = GhtType!(u32, u64 => u16, &'static str: VariadicCountedHashSet);
        let _x = LongKeyLongValTrie::new_from(vec![var_expr!(1, 999, 222, "hello")]);
    }

    #[test]
    fn test_insert() {
        use variadics::var_expr;

        use crate::GhtType;
        use crate::ght::GeneralizedHashTrieNode;

        type MyGht = GhtType!(u16, u32 => u64: VariadicCountedHashSet);
        let mut htrie = MyGht::default();
        htrie.insert(var_expr!(42, 314, 43770));
        assert_eq!(htrie.recursive_iter().count(), 1);
        assert_eq!(MyGht::HEIGHT, 2);
        htrie.insert(var_expr!(42, 315, 43770));
        assert_eq!(htrie.recursive_iter().count(), 2);
        htrie.insert(var_expr!(42, 314, 30619));
        assert_eq!(htrie.recursive_iter().count(), 3);
        htrie.insert(var_expr!(43, 10, 600));
        assert_eq!(htrie.recursive_iter().count(), 4);
        assert!(htrie.contains(var_expr!(&42, &314, &30619)));
        assert!(htrie.contains(var_expr!(&42, &315, &43770)));
        assert!(htrie.contains(var_expr!(&43, &10, &600)));

        type LongKeyLongValTrie = GhtType!(u32, u64 => u16, &'static str: VariadicCountedHashSet);
        let mut htrie = LongKeyLongValTrie::new_from(vec![var_expr!(1, 999, 222, "hello")]);
        htrie.insert(var_expr!(1, 999, 111, "bye"));
        htrie.insert(var_expr!(1, 1000, 123, "cya"));
        assert!(htrie.contains(var_expr!(&1, &999, &222, &"hello")));
        assert!(htrie.contains(var_expr!(&1, &999, &111, &"bye")));
        assert!(htrie.contains(var_expr!(&1, &1000, &123, &"cya")));
    }

    #[test]
    fn test_scale() {
        use variadics::var_expr;

        use crate::GhtType;
        use crate::ght::GeneralizedHashTrieNode;

        type MyGht = GhtType!(bool, usize, &'static str => i32: VariadicCountedHashSet);
        let mut htrie = MyGht::new_from(vec![var_expr!(true, 1, "hello", -5)]);
        assert_eq!(htrie.recursive_iter().count(), 1);
        for i in 1..1000000 {
            htrie.insert(var_expr!(true, 1, "hello", i));
        }
        assert_eq!(htrie.recursive_iter().count(), 1000000);
    }

    #[test]
    fn test_contains() {
        use variadics::{VariadicExt, var_expr};

        use crate::GhtType;
        use crate::ght::GeneralizedHashTrieNode;

        type MyGht = GhtType!(u16, u32 => u64: VariadicCountedHashSet);
        let htrie = MyGht::new_from(vec![var_expr!(42_u16, 314_u32, 43770_u64)]);
        let x = var_expr!(&42, &314, &43770);
        assert!(htrie.contains(x));
        assert!(htrie.contains(var_expr!(42, 314, 43770).as_ref_var()));
        assert!(htrie.contains(var_expr!(&42, &314, &43770)));
        assert!(!htrie.contains(var_expr!(42, 314, 30619).as_ref_var()));
        assert!(!htrie.contains(var_expr!(&42, &315, &43770)));
        assert!(!htrie.contains(var_expr!(&43, &314, &43770)));
    }

    #[test]
    fn test_get() {
        use variadics::{VariadicExt, var_expr};

        use crate::GhtType;
        use crate::ght::{GeneralizedHashTrieNode, GhtGet};

        type MyGht = GhtType!(u32, u32 => u32: VariadicCountedHashSet);
        let ht_root = MyGht::new_from(vec![var_expr!(42, 314, 43770)]);

        let inner = ht_root.get(&42).unwrap();
        let t = inner.recursive_iter().next().unwrap();
        assert_eq!(t, var_expr!(&42, &314, &43770));

        let leaf = inner.get(&314).unwrap();
        let t = leaf.recursive_iter().next().unwrap();
        assert_eq!(t, var_expr!(42, 314, 43770).as_ref_var());
    }

    #[test]
    fn test_iter() {
        use variadics::var_expr;

        use crate::GhtType;
        use crate::ght::{GeneralizedHashTrieNode, GhtGet};
        type MyGht = GhtType!(u32, u32 => u32: VariadicCountedHashSet);
        let ht_root = MyGht::new_from(vec![var_expr!(42, 314, 43770)]);
        let inner_key = ht_root.iter().next().unwrap();
        let inner = ht_root.get(&inner_key).unwrap();
        let t = inner.recursive_iter().next().unwrap();
        assert_eq!(t, var_expr!(&42, &314, &43770));

        let leaf_key = inner.iter().next().unwrap();
        let leaf = inner.get(&leaf_key).unwrap();
        // iter() on leaf should return None
        let t = leaf.iter().next();
        assert!(t.is_none());
    }

    #[test]
    fn test_recursive_iter() {
        use variadics::{VariadicExt, var_expr, var_type};

        use crate::GhtType;
        use crate::ght::GeneralizedHashTrieNode;

        type MyGht = GhtType!(u32, u32 => u32: VariadicCountedHashSet);
        type InputType = var_type!(u32, u32, u32);
        type ResultType<'a> = var_type!(&'a u32, &'a u32, &'a u32);
        let input: HashSet<InputType> = HashSet::from_iter(
            [
                (42, 314, 30619),
                (42, 314, 43770),
                (42, 315, 43770),
                (43, 10, 600),
            ]
            .iter()
            .map(|&(a, b, c)| var_expr!(a, b, c)),
        );
        let htrie = MyGht::new_from(input.clone());
        #[expect(
            clippy::disallowed_methods,
            reason = "nondeterministic iteration order, fine to collect into set"
        )]
        let result = input.iter().map(|v| v.as_ref_var()).collect();
        let v: HashSet<ResultType> = htrie.recursive_iter().collect();
        assert_eq!(v, result);
    }

    #[test]
    fn test_prefix_iter_leaf() {
        use variadics::variadic_collections::VariadicCountedHashSet;
        use variadics::{var_expr, var_type};

        use crate::ght::{GeneralizedHashTrieNode, GhtLeaf, GhtPrefixIter};

        type InputType = var_type!(u8, u16, u32);
        type ResultType<'a> = var_type!(&'a u8, &'a u16, &'a u32);

        let input: HashSet<InputType> = HashSet::from_iter(
            [
                (42, 314, 30619),
                (42, 314, 43770),
                (42, 315, 43770),
                (43, 10, 600),
            ]
            .iter()
            .map(|&(a, b, c)| var_expr!(a, b, c)),
        );
        let leaf =
            GhtLeaf::<InputType, var_type!(u16, u32), VariadicCountedHashSet<InputType>>::new_from(
                input.clone(),
            );
        // let key = var_expr!(42u8).as_ref_var();
        let key = (); // (var_expr!().as_ref_var();)
        let v: HashSet<ResultType> = leaf.prefix_iter(key).collect();
        #[expect(
            clippy::disallowed_methods,
            reason = "nondeterministic iteration order, fine to collect into set"
        )]
        let result = input
            .iter()
            // .filter(|t: &&InputType| t.0 == 42)
            .map(|t: &InputType| var_expr!(&t.0, &t.1 .0, &t.1 .1 .0))
            .collect();
        assert_eq!(v, result);
    }

    #[test]
    fn test_prefix_iter() {
        use variadics::{VariadicExt, var_expr, var_type};

        use crate::GhtType;
        use crate::ght::{GeneralizedHashTrieNode, GhtPrefixIter};

        type MyGht = GhtType!(u8, u16 => u32: VariadicCountedHashSet);
        type InputType = var_type!(u8, u16, u32);
        type ResultType<'a> = var_type!(&'a u8, &'a u16, &'a u32);
        let input: HashSet<InputType> = HashSet::from_iter(
            [
                (42, 314, 30619),
                (42, 314, 43770),
                (42, 315, 43770),
                (43, 10, 600),
            ]
            .iter()
            .map(|&(a, b, c)| var_expr!(a, b, c)),
        );
        let htrie = MyGht::new_from(input.clone());

        let v: HashSet<ResultType> = htrie.prefix_iter(var_expr!(42, 315).as_ref_var()).collect();
        let result = HashSet::from_iter([var_expr!(&42, &315, &43770)].iter().copied());
        assert_eq!(v, result);

        let v: HashSet<ResultType> = htrie.prefix_iter(var_expr!(42u8).as_ref_var()).collect();
        #[expect(
            clippy::disallowed_methods,
            reason = "nondeterministic iteration order, fine to collect into set"
        )]
        let result = input
            .iter()
            .filter(|t: &&InputType| t.0 == 42)
            .map(|t: &InputType| var_expr!(&t.0, &t.1.0, &t.1.1.0))
            .collect();
        assert_eq!(v, result);

        for row in htrie.prefix_iter(var_expr!(42, 315, 43770).as_ref_var()) {
            assert_eq!(row, var_expr!(&42, &315, &43770));
        }
    }

    #[test]
    fn test_prefix_iter_complex() {
        use variadics::{VariadicExt, var_expr, var_type};

        use crate::GhtType;
        use crate::ght::{GeneralizedHashTrieNode, GhtPrefixIter};

        type MyGht = GhtType!(bool, u32, &'static str => i32: VariadicCountedHashSet);
        type InputType = var_type!(bool, u32, &'static str, i32);
        type ResultType<'a> = var_type!(&'a bool, &'a u32, &'a &'static str, &'a i32);
        let input: HashSet<InputType> = HashSet::from_iter(
            [
                (true, 1, "hello", -5),
                (true, 1, "hi", -2),
                (true, 1, "hi", -3),
                (true, 1, "hi", -4),
                (true, 1, "hi", -5),
                (true, 2, "hello", 1),
                (false, 10, "bye", 5),
            ]
            .iter()
            .map(|&(a, b, c, d)| var_expr!(a, b, c, d)),
        );

        let htrie = MyGht::new_from(input.clone());

        let v: HashSet<ResultType> = htrie
            .prefix_iter(var_expr!(true, 1, "hi").as_ref_var())
            .collect();
        #[expect(
            clippy::disallowed_methods,
            reason = "nondeterministic iteration order, fine to collect into set"
        )]
        let result = input
            .iter()
            .filter(|t: &&InputType| t.0 && t.1.0 == 1 && t.1.1.0 == "hi")
            //.map(|t: &InputType| (&t.0, &t.1 .0, (&t.1 .1 .0, (&t.1 .1 .1 .0, ()))))
            .map(|t| t.as_ref_var())
            .collect();
        assert_eq!(v, result);

        let v: HashSet<ResultType> = htrie.prefix_iter(var_expr!(true).as_ref_var()).collect();
        #[expect(
            clippy::disallowed_methods,
            reason = "nondeterministic iteration order, fine to collect into set"
        )]
        let result = input
            .iter()
            .filter(|t: &&InputType| t.0)
            .map(|t: &InputType| t.as_ref_var())
            .collect();
        assert_eq!(v, result);
    }

    #[test]
    fn test_merge() {
        use variadics::{var_expr, var_type};

        use crate::ght::GeneralizedHashTrieNode;
        use crate::{GhtType, Merge};

        type MyGht = GhtType!(u32, u64 => u16, &'static str: VariadicHashSet);

        let mut test_ght1 = MyGht::new_from(vec![var_expr!(42, 314, 10, "hello")]);
        let test_ght2 = MyGht::new_from(vec![var_expr!(42, 314, 10, "hello")]);

        assert_eq!(
            test_ght1
                .recursive_iter()
                .collect::<Vec<var_type!(&u32, &u64, &u16, &&'static str)>>()
                .len(),
            1
        );
        test_ght1.merge(test_ght2.clone());
        // merge does not contain duplicate copy of the tuple
        assert_eq!(
            test_ght1
                .recursive_iter()
                .collect::<Vec<var_type!(&u32, &u64, &u16, &&'static str)>>()
                .len(),
            1
        );
        assert!(!test_ght1.merge(test_ght2.clone()));

        let mut test_ght1 = MyGht::new_from(vec![var_expr!(42, 314, 10, "hello")]);
        let mut test_ght2 = MyGht::new_from(vec![var_expr!(42, 314, 10, "hello")]);
        test_ght1.merge(test_ght2.clone());

        test_ght1.insert(var_expr!(42, 314, 20, "goodbye"));
        test_ght2.insert(var_expr!(42, 314, 20, "again"));

        // change on merge
        assert!(test_ght1.merge(test_ght2.clone()));
        for k in test_ght2.recursive_iter() {
            assert!(test_ght1.contains(k))
        }
    }

    #[test]
    fn test_node_lattice() {
        use variadics::var_expr;

        use crate::ght::GeneralizedHashTrieNode;
        use crate::{GhtType, NaiveLatticeOrd};

        type MyGht = GhtType!(u32, u64 => u16, &'static str: VariadicHashSet);
        type MyGhtNode = GhtType!(u32, u64 => u16, &'static str: VariadicHashSet);

        let mut test_vec: Vec<MyGhtNode> = Vec::new();

        let empty_ght = MyGht::new_from(vec![]);
        let test_ght1 = MyGht::new_from(vec![var_expr!(42, 314, 10, "hello")]);
        let mut test_ght2 = MyGht::new_from(vec![var_expr!(42, 314, 10, "hello")]);
        test_ght2.insert(var_expr!(42, 314, 20, "again"));
        let mut test_ght3 = test_ght2.clone();
        test_ght3.insert(var_expr!(42, 400, 1, "level 2"));
        let mut test_ght4 = test_ght3.clone();
        test_ght4.insert(var_expr!(43, 1, 1, "level 1"));

        let test_vec_wrap = [empty_ght, test_ght1, test_ght2, test_ght3, test_ght4];

        for ght in test_vec_wrap.iter().cloned() {
            ght.naive_cmp(&ght.clone());
            test_vec.push(ght);
        }
        crate::test::check_all(&test_vec);
        crate::test::check_all(&test_vec_wrap);
    }

    #[test]
    fn test_cartesian_bimorphism() {
        use variadics::var_expr;

        use crate::ght::GeneralizedHashTrieNode;
        use crate::ght::lattice::GhtCartesianProductBimorphism;
        use crate::{GhtType, LatticeBimorphism};

        type MyGhtA = GhtType!(u32, u64 => u16, &'static str: VariadicHashSet);
        type MyGhtB = GhtType!(u32, u64, u16 => &'static str: VariadicHashSet);

        let mut ght_a = MyGhtA::default();
        let mut ght_b = MyGhtB::default();

        ght_a.insert(var_expr!(123, 2, 5, "hello"));
        ght_a.insert(var_expr!(50, 1, 1, "hi"));
        ght_a.insert(var_expr!(5, 1, 7, "hi"));
        ght_b.insert(var_expr!(5, 1, 8, "hi"));
        ght_b.insert(var_expr!(10, 1, 2, "hi"));
        ght_b.insert(var_expr!(12, 10, 98, "bye"));

        type MyGhtAb = GhtType!(u32, u64, u16, &'static str, u32, u64 => u16, &'static str: VariadicCountedHashSet);

        let mut bim = GhtCartesianProductBimorphism::<MyGhtAb>::default();
        let ght_out = bim.call(&ght_a, &ght_b);
        assert_eq!(
            ght_out.recursive_iter().count(),
            ght_a.recursive_iter().count() * ght_b.recursive_iter().count()
        );
    }

    #[test]
    fn test_join_bimorphism() {
        use variadics::variadic_collections::{VariadicCountedHashSet, VariadicHashSet};
        use variadics::{var_expr, var_type};

        use crate::ght::lattice::{
            DeepJoinLatticeBimorphism, GhtNodeKeyedBimorphism, GhtValTypeProductBimorphism,
        };
        use crate::ght::{GeneralizedHashTrieNode, GhtInner, GhtLeaf};
        use crate::{GhtType, LatticeBimorphism};

        type ResultSchemaType = var_type!(u32, u64, u16, &'static str, &'static str);
        type ResultSchemaRefType<'a> = var_type!(
            &'a u32,
            &'a u64,
            &'a u16,
            &'a &'static str,
            &'a &'static str
        );
        type MyGhtATrie = GhtType!(u32, u64, u16 => &'static str: VariadicHashSet);
        type MyGhtBTrie = GhtType!(u32, u64, u16 => &'static str: VariadicHashSet);

        let mut ght_a = MyGhtATrie::default();
        let mut ght_b = MyGhtBTrie::default();

        ght_a.insert(var_expr!(123, 2, 5, "hello"));
        ght_a.insert(var_expr!(50, 1, 1, "hi"));
        ght_a.insert(var_expr!(5, 1, 7, "hi"));

        ght_b.insert(var_expr!(5, 1, 8, "hi"));
        ght_b.insert(var_expr!(5, 1, 7, "world"));
        ght_b.insert(var_expr!(10, 1, 2, "hi"));
        ght_b.insert(var_expr!(12, 10, 98, "bye"));

        let result: HashSet<ResultSchemaRefType> = [var_expr!(&5, &1, &7, &"hi", &"world")]
            .iter()
            .copied()
            .collect();
        {
            // here we manually construct the proper bimorphism stack.
            // note that the bottommost bimorphism is GhtValTypeProductBimorphism,
            // which ensures that the Schema of the resulting output GhtLeaf and GhtInner
            // nodes correctly includes the key columns, not just the cross-product of the values.
            type MyGhtOut = GhtInner<
                &'static str,
                GhtLeaf<
                    ResultSchemaType,
                    var_type!(&'static str),
                    VariadicCountedHashSet<ResultSchemaType>,
                >,
            >;
            // let mut bim = GhtNodeKeyedBimorphism::new(GhtNodeKeyedBimorphism::new(
            //     GhtNodeKeyedBimorphism::new(GhtValTypeProductBimorphism::<MyGhtOut>::default()),
            // ));
            let mut bim = GhtNodeKeyedBimorphism::new(GhtNodeKeyedBimorphism::new(
                GhtNodeKeyedBimorphism::new(GhtValTypeProductBimorphism::<MyGhtOut>::default()),
            ));
            let out = bim.call(&ght_a, &ght_b);
            let out: HashSet<ResultSchemaRefType> = out.recursive_iter().collect();
            assert_eq!(out, result);
        }
        {
            // Here we use DeepJoinLatticeBimorphism as a more compact representation of the
            // manual stack of bimorphisms above. This is the recommended approach.
            type MyNodeBim<'a> = <(MyGhtATrie, MyGhtBTrie) as DeepJoinLatticeBimorphism<
                VariadicHashSet<ResultSchemaType>,
            >>::DeepJoinLatticeBimorphism;
            let mut bim = <MyNodeBim as Default>::default();
            let out = bim.call(&ght_a, &ght_b);
            let out: HashSet<ResultSchemaRefType> = out.recursive_iter().collect();
            assert_eq!(out, result);
        }
    }

    #[test]
    fn test_ght_with_tuple_macro() {
        use variadics::{VariadicExt, var_expr};
        use variadics_macro::tuple;

        use crate::GhtType;
        use crate::ght::GeneralizedHashTrieNode;

        type MyRoot = GhtType!(u16, u32 => u64: VariadicCountedHashSet);

        let mut trie1 = MyRoot::default();
        assert_eq!(3, <<MyRoot as GeneralizedHashTrieNode>::Schema>::LEN);
        trie1.insert(var_expr!(1, 2, 3));
        let t = trie1.recursive_iter().next().unwrap();
        let tup = tuple!(t, 3);
        assert_eq!(tup, (&1, &2, &3));
    }

    #[test]
    fn test_triangle_generic_join() {
        use std::hash::{BuildHasherDefault, DefaultHasher};

        use variadics::var_expr;

        use crate::GhtType;
        use crate::ght::{GeneralizedHashTrieNode, GhtPrefixIter};

        const MATCHES: u32 = 1000;
        type MyGht = GhtType!(u32 => u32: VariadicCountedHashSet);

        let r_iter = (0..MATCHES)
            .map(|i| (0, i))
            .chain((1..MATCHES).map(|i| (i, 0)));

        let s_iter = (0..MATCHES)
            .map(|i| (0, i))
            .chain((1..MATCHES).map(|i| (i, 0)));

        let t_iter = (0..MATCHES)
            .map(|i| (0, i))
            .chain((1..MATCHES).map(|i| (i, 0)));

        let rx_ght = MyGht::new_from(r_iter.clone().map(|(x, y)| var_expr!(x, y)));
        let sb_ght = MyGht::new_from(s_iter.clone().map(|(y, b)| var_expr!(b, y)));
        let tx_ght = MyGht::new_from(t_iter.clone().map(|(z, x)| var_expr!(x, z)));

        let r_x = r_iter
            .clone()
            .map(|(x, _y)| x)
            .collect::<HashSet<_, BuildHasherDefault<DefaultHasher>>>();
        let t_x = s_iter
            .clone()
            .map(|(_z, x)| x)
            .collect::<HashSet<_, BuildHasherDefault<DefaultHasher>>>();
        let x_inter = r_x.intersection(&t_x);
        let len = x_inter.clone().count();
        if len > 1 {
            assert_eq!(1000, len);
        }

        let mut output: Vec<(u32, u32, u32)> = Vec::new();
        let mut x_iters = 0usize;
        let mut y_iters = 0usize;
        let mut z_iters = 0usize;
        for a in x_inter {
            x_iters += 1;
            let r = rx_ght
                .prefix_iter(var_expr!(a))
                .map(|(_x, (y, ()))| *y)
                .collect::<HashSet<_, BuildHasherDefault<DefaultHasher>>>();
            let s_y = s_iter
                .clone()
                .map(|(y, _z)| y)
                .collect::<HashSet<_, BuildHasherDefault<DefaultHasher>>>();
            let y_inter = r.intersection(&s_y);
            let len = y_inter.clone().count();
            if len > 1 {
                assert_eq!(1000, len);
            }
            for b in y_inter {
                y_iters += 1;
                let s = sb_ght
                    .prefix_iter(var_expr!(b))
                    .map(|(_b, (z, ()))| *z)
                    .collect::<HashSet<_, BuildHasherDefault<DefaultHasher>>>();
                let t = tx_ght
                    .prefix_iter(var_expr!(a))
                    .map(|(_x, (z, ()))| *z)
                    .collect::<HashSet<_, BuildHasherDefault<DefaultHasher>>>();
                let z_inter = s.intersection(&t);
                let len = z_inter.clone().count();
                if len > 1 {
                    assert_eq!(1000, len);
                }
                for c in z_inter {
                    z_iters += 1;
                    output.push((*a, *b, *c));
                }
            }
        }

        assert_eq!(1000, x_iters);
        assert_eq!(1999, y_iters);
        assert_eq!(2998, z_iters);
        assert_eq!(2998, output.len());
    }

    fn clover_setup(
        matches: usize,
    ) -> (
        impl Iterator<Item = (u32, u32)>,
        impl Iterator<Item = (u32, u32)>,
        impl Iterator<Item = (u32, u32)>,
    ) {
        let r_iter = (1..matches)
            .map(|i| (1u32, i as u32))
            .chain((1..matches).map(|i| (2, i as u32)))
            .chain([(0, 0)]);

        let s_iter = (1..matches)
            .map(|i| (2u32, i as u32))
            .chain((1..matches).map(|i| (3, i as u32)))
            .chain([(0, 0)]);

        let t_iter = (1..matches)
            .map(|i| (3u32, i as u32))
            .chain((1..matches).map(|i| (1, i as u32)))
            .chain([(0, 0)]);
        (r_iter, s_iter, t_iter)
    }

    #[test]
    fn clover_generic_join() {
        use variadics::var_expr;

        use crate::GhtType;
        use crate::ght::{GeneralizedHashTrieNode, GhtGet};

        const MATCHES: usize = 1000;
        let (r_iter, s_iter, t_iter) = clover_setup(MATCHES);

        type MyGht = GhtType!(u32 => u32: VariadicCountedHashSet);
        let rx_ght = MyGht::new_from(r_iter.map(|(x, a)| var_expr!(x, a)));
        let sx_ght = MyGht::new_from(s_iter.map(|(x, b)| var_expr!(x, b)));
        let tx_ght = MyGht::new_from(t_iter.map(|(x, c)| var_expr!(x, c)));
        for x in rx_ght.iter() {
            if let (Some(r), Some(s), Some(t)) = (rx_ght.get(&x), sx_ght.get(&x), tx_ght.get(&x)) {
                // All unwraps succeeded, use `r`, `s`, `t` here
                for a in r.iter() {
                    for b in s.iter() {
                        for c in t.iter() {
                            assert_eq!((x, a, b, c), (0, 0, 0, 0));
                        }
                    }
                }
            } else {
                // If any unwrap fails, continue to the next iteration
                continue;
            }
        }
    }

    #[test]
    fn clover_factorized_join() {
        use variadics::var_expr;

        use crate::GhtType;
        use crate::ght::{GeneralizedHashTrieNode, GhtGet};

        const MATCHES: usize = 1000;
        let (r_iter, s_iter, t_iter) = clover_setup(MATCHES);

        type Ght1 = GhtType!(() => u32, u32: VariadicCountedHashSet);
        type Ght2 = GhtType!(u32 => u32: VariadicCountedHashSet);
        let rx_ght = Ght1::new_from(r_iter.map(|(x, a)| var_expr!(x, a)));
        let sx_ght = Ght2::new_from(s_iter.map(|(x, b)| var_expr!(x, b)));
        let tx_ght = Ght2::new_from(t_iter.map(|(x, c)| var_expr!(x, c)));

        for t in rx_ght.recursive_iter() {
            let (x, (a, ())): (&u32, (&u32, _)) = t;
            if let (Some(s), Some(t)) = (sx_ght.get(x), tx_ght.get(x)) {
                // All unwraps succeeded, use `s`, `t` here
                for b in s.iter() {
                    for c in t.iter() {
                        assert_eq!((x, a, b, c), (&0, &0, 0, 0));
                    }
                }
            } else {
                // If any unwrap fails, continue to the next iteration
                continue;
            }
        }
    }

    #[test]
    fn test_force() {
        use variadics::var_expr;

        use crate::GhtType;
        use crate::ght::GeneralizedHashTrieNode;
        use crate::ght::colt::ColtForestNode;

        type LeafType = GhtType!(() => u16, u32, u64: VariadicCountedHashSet);
        let n = LeafType::new_from(vec![
            var_expr!(1, 1, 1),
            var_expr!(1, 2, 2),
            var_expr!(1, 3, 3),
            var_expr!(2, 4, 4),
        ]);
        let out = n.force().unwrap();
        assert_eq!(out.height(), 1);
    }

    #[test]
    fn test_forest_macro() {
        use crate::ColtType;

        type Forest4 = ColtType!(u8, u16, u32, u64);
        let _f4 = Forest4::default();

        type Forest3 = ColtType!(u8, u16, u32);
        let _f3 = Forest3::default();

        type Forest2 = ColtType!(u8, u16);
        let _f2 = Forest2::default();

        type Forest1 = ColtType!(u8);
        let _f2 = Forest1::default();

        type Forest01 = ColtType!(() => u16);
        let _f01 = Forest01::default();

        type Forest02 = ColtType!(() => u8, u16);
        let _f02 = Forest02::default();

        type Forest10 = ColtType!(u8 => ());
        let _f10 = Forest10::default();

        type Forest11 = ColtType!(u8 => u16);
        let _f11 = Forest11::default();

        type Forest12 = ColtType!(u8 => u16, u32);
        let _f12 = Forest12::default();

        type Forest20 = ColtType!(u8, u16 => ());
        let _f20 = Forest20::default();

        type Forest21 = ColtType!(u8, u16 => u32);
        let _f21 = Forest21::default();

        type Forest22 = ColtType!(u8, u16 => u32, u64);
        let _f22 = Forest22::default();
    }

    #[test]
    fn test_colt_little_get() {
        use variadics::variadic_collections::VariadicCollection;
        use variadics::{VariadicExt, var_expr};

        use crate::ColtType;
        use crate::ght::GeneralizedHashTrieNode;
        use crate::ght::colt::ColtGet;

        type MyForest = ColtType!(u8);

        let mut forest = MyForest::default();

        forest.0.insert(var_expr!(1));
        forest.0.insert(var_expr!(2));
        forest.0.insert(var_expr!(3));

        assert_eq!(2, forest.len());
        assert_eq!(3, forest.0.elements.len());

        let result = ColtGet::get(forest.as_mut_var(), &3);
        assert_eq!(1, result.len());
        assert_eq!(0, forest.0.elements.len());
        assert!(forest.0.forced);
    }

    #[test]
    fn test_colt_get() {
        use variadics::variadic_collections::VariadicCollection;
        use variadics::{VariadicExt, var_expr};

        use crate::ColtType;
        use crate::ght::colt::ColtGet;
        use crate::ght::{GeneralizedHashTrieNode, GhtGet};

        type MyForest = ColtType!(u8, u16, u32, u64);
        let mut forest = MyForest::default();
        forest.0.insert(var_expr!(1, 1, 1, 1));
        forest.0.insert(var_expr!(2, 2, 2, 2));
        forest.0.insert(var_expr!(3, 3, 3, 3));

        let len = forest.len();
        assert_eq!(5, len);
        {
            let get_result = ColtGet::get(forest.as_mut_var(), &1);
            assert_eq!(get_result.len(), len - 1);
            assert_eq!(get_result.0.height(), 0);
            let get_result2 = ColtGet::get(get_result, &1);
            assert_eq!(get_result2.len(), len - 2);
            let get_result3 = ColtGet::get(get_result2, &1);
            assert_eq!(get_result3.len(), len - 3);
            assert_eq!(
                get_result3.0.elements.iter().next(),
                Some(var_expr!(1, 1, 1, 1).as_ref_var())
            );
            assert_eq!(get_result3.1.0.children.len(), 0);
        }
        {
            let get_result = ColtGet::get(forest.as_mut_var(), &3);
            assert_eq!(get_result.len(), len - 1);
            let get_result2 = ColtGet::get(get_result, &3);
            assert_eq!(get_result2.len(), len - 2);
            assert_eq!(
                get_result2.0.elements.iter().next(),
                Some(var_expr!(3, 3, 3, 3).as_ref_var())
            );
            assert_eq!(get_result2.1.0.children.len(), 0);
        }
        assert!(forest.0.forced);
        assert_eq!(3, forest.1.0.children.len()); // keys 1, 2 and 3
        assert_eq!(0, forest.1.0.get(&1).unwrap().elements.len());
        assert_eq!(1, forest.1.0.get(&2).unwrap().elements.len());
        assert_eq!(0, forest.1.0.get(&3).unwrap().elements.len());
        assert_eq!(2, forest.1.1.0.children.len()); // keys 1 and 3
        assert_eq!(
            0,
            forest
                .1
                .1
                .0
                .get(&1)
                .unwrap()
                .get(&1)
                .unwrap()
                .elements
                .len()
        );
        assert!(forest.1.1.0.get(&2).is_none());
        assert_eq!(
            1,
            forest
                .1
                .1
                .0
                .get(&3)
                .unwrap()
                .get(&3)
                .unwrap()
                .elements
                .len()
        );
        assert_eq!(
            1,
            forest
                .1
                .1
                .1
                .0
                .get(&1)
                .unwrap()
                .get(&1)
                .unwrap()
                .get(&1)
                .unwrap()
                .elements
                .len()
        );
    }

    #[test]
    fn test_colt_scale() {
        use variadics::variadic_collections::VariadicCollection;
        use variadics::{VariadicExt, var_expr};

        use crate::ght::colt::ColtGet;
        use crate::ght::{GeneralizedHashTrieNode, GhtPrefixIter};

        type MyColt = crate::ColtType!(i32, bool, usize, &'static str);
        let mut forest = MyColt::default();
        for i in 1..100000 {
            forest.0.insert(var_expr!(i, true, 1, "hello"));
        }
        {
            let result = forest.as_mut_var().get(&3);
            assert_eq!(result.len(), 4);
        }
        // check: first Leaf trie is forced
        assert!(forest.0.forced);
        assert_eq!(forest.0.elements.len(), 0);
        {
            let result = forest.as_mut_var().get(&3);
            let result2 = result.get(&true);
            assert_eq!(result2.len(), 3);
        }
        {
            // check: leaf below 3 in first non-empty trie is forced
            let result = forest.as_mut_var().get(&3);
            assert!(result.0.forced);
            assert_eq!(result.0.elements.len(), 0);
        }
        // check: prefix (3, true) is now found in the third trie: forest.1.1.0
        assert!(
            forest
                .1
                .1
                .0
                .prefix_iter(var_expr!(3, true).as_ref_var())
                .next()
                .is_some()
        );
        {
            let result = forest.as_mut_var().get(&3);
            let result2 = result.get(&true);
            assert_eq!(result2.len(), 3);
            let result3 = result2.get(&1);
            assert_eq!(result3.len(), 2);
            let result4 = result3.get(&"hello");
            assert_eq!(result4.0.elements.len(), 1);
            assert_eq!(
                result4.0.elements.iter().next(),
                Some(var_expr!(3, true, 1, "hello").as_ref_var())
            );
        }
    }
}
