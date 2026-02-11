use std::collections::{BTreeMap, BTreeSet};

use dfir_rs::dfir_syntax;
use dfir_rs::util::collect_ready;

fn run_unique_keyed(input: Vec<(String, i32)>) -> Vec<(String, i32)> {
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<(String, i32)>();

    let mut df = dfir_syntax! {
        source_iter(input)
            -> unique_keyed()
            -> for_each(|kv| result_send.send(kv).unwrap());
    };
    df.run_available_sync();

    collect_ready::<Vec<_>, _>(&mut result_recv)
}

fn distinct_values_per_key(tuples: &[(String, i32)]) -> BTreeMap<String, BTreeSet<i32>> {
    let mut map = BTreeMap::new();
    for (k, v) in tuples {
        map.entry(k.clone()).or_insert_with(BTreeSet::new).insert(*v);
    }
    map
}

fn input(pairs: &[(&str, i32)]) -> Vec<(String, i32)> {
    pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
}

#[test]
fn test_unique_keyed_no_duplicates() {
    let output = run_unique_keyed(input(&[
        ("a", 1),
        ("b", 2),
        ("a", 1),
        ("b", 3),
        ("a", 2),
        ("a", 1),
    ]));

    let mut seen: BTreeMap<String, BTreeSet<i32>> = BTreeMap::new();
    for (k, v) in &output {
        let was_new = seen.entry(k.clone()).or_default().insert(*v);
        assert!(was_new, "Duplicate value {} for key {}", v, k);
    }
}

#[test]
fn test_unique_keyed_preserves_distinct_set() {
    let inp = input(&[("a", 1), ("b", 2), ("a", 1), ("b", 3), ("a", 2)]);
    let output = run_unique_keyed(inp.clone());

    assert_eq!(
        distinct_values_per_key(&inp),
        distinct_values_per_key(&output)
    );
}

#[test]
fn test_unique_keyed_empty() {
    let output = run_unique_keyed(vec![]);
    assert!(output.is_empty());
}

#[test]
fn test_unique_keyed_all_duplicates() {
    let output = run_unique_keyed(input(&[("a", 1), ("a", 1), ("a", 1)]));
    assert_eq!(output.len(), 1);
    assert_eq!(output[0].1, 1);
}
