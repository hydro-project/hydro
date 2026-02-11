use std::collections::BTreeMap;

use dfir_rs::dfir_syntax;
use dfir_rs::util::collect_ready;

fn run_sort_keyed(input: Vec<(String, i32)>) -> Vec<(String, i32)> {
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<(String, i32)>();

    let mut df = dfir_syntax! {
        source_iter(input)
            -> sort_keyed()
            -> for_each(|kv| result_send.send(kv).unwrap());
    };
    df.run_available_sync();

    collect_ready::<Vec<_>, _>(&mut result_recv)
}

fn values_per_key(tuples: &[(String, i32)]) -> BTreeMap<String, Vec<i32>> {
    let mut map = BTreeMap::new();
    for (k, v) in tuples {
        map.entry(k.clone()).or_insert_with(Vec::new).push(*v);
    }
    map
}

fn input(pairs: &[(&str, i32)]) -> Vec<(String, i32)> {
    pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
}

#[test]
fn test_sort_keyed_sorted_per_key() {
    let output = run_sort_keyed(input(&[
        ("a", 3),
        ("b", 1),
        ("a", 1),
        ("b", 2),
        ("c", 5),
        ("a", 2),
    ]));
    let per_key = values_per_key(&output);

    assert_eq!(per_key["a"], vec![1, 2, 3]);
    assert_eq!(per_key["b"], vec![1, 2]);
    assert_eq!(per_key["c"], vec![5]);
}

#[test]
fn test_sort_keyed_preserves_multiset() {
    let inp = input(&[("x", 10), ("y", 20), ("x", 10), ("y", 30), ("x", 5)]);
    let output = run_sort_keyed(inp.clone());

    let mut input_per_key = values_per_key(&inp);
    let mut output_per_key = values_per_key(&output);

    for vs in input_per_key.values_mut() {
        vs.sort();
    }
    for vs in output_per_key.values_mut() {
        vs.sort();
    }

    assert_eq!(input_per_key, output_per_key);
}

#[test]
fn test_sort_keyed_empty() {
    let output = run_sort_keyed(vec![]);
    assert!(output.is_empty());
}

#[test]
fn test_sort_keyed_single_key() {
    let output = run_sort_keyed(input(&[("a", 3), ("a", 1), ("a", 2)]));
    let vals: Vec<i32> = output.into_iter().map(|(_, v)| v).collect();
    assert_eq!(vals, vec![1, 2, 3]);
}
