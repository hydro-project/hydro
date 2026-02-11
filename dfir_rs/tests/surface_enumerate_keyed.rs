use std::collections::BTreeMap;

use dfir_rs::dfir_syntax;
use dfir_rs::util::collect_ready;

fn run_enumerate_keyed(input: Vec<(String, String)>) -> Vec<(String, (usize, String))> {
    let (result_send, mut result_recv) =
        dfir_rs::util::unbounded_channel::<(String, (usize, String))>();

    let mut df = dfir_syntax! {
        source_iter(input)
            -> enumerate_keyed()
            -> for_each(|kv| result_send.send(kv).unwrap());
    };
    df.run_available_sync();

    collect_ready::<Vec<_>, _>(&mut result_recv)
}

fn input(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[test]
fn test_enumerate_keyed_sequential_indices_and_values() {
    let output = run_enumerate_keyed(input(&[
        ("a", "x"),
        ("b", "y"),
        ("a", "z"),
        ("b", "w"),
        ("a", "q"),
    ]));

    let mut per_key: BTreeMap<String, Vec<(usize, String)>> = BTreeMap::new();
    for (k, (i, v)) in &output {
        per_key.entry(k.clone()).or_default().push((*i, v.clone()));
    }

    assert_eq!(
        per_key["a"],
        vec![(0, "x".into()), (1, "z".into()), (2, "q".into())]
    );
    assert_eq!(per_key["b"], vec![(0, "y".into()), (1, "w".into())]);
}

#[test]
fn test_enumerate_keyed_count_and_values_per_key() {
    let inp = input(&[("a", "x"), ("b", "y"), ("a", "z")]);
    let output = run_enumerate_keyed(inp);

    let mut per_key: BTreeMap<String, Vec<(usize, String)>> = BTreeMap::new();
    for (k, (i, v)) in &output {
        per_key.entry(k.clone()).or_default().push((*i, v.clone()));
    }

    assert_eq!(per_key["a"], vec![(0, "x".into()), (1, "z".into())]);
    assert_eq!(per_key["b"], vec![(0, "y".into())]);
}

#[test]
fn test_enumerate_keyed_empty() {
    let output = run_enumerate_keyed(vec![]);
    assert!(output.is_empty());
}

#[test]
fn test_enumerate_keyed_single_key() {
    let output = run_enumerate_keyed(input(&[("a", "x"), ("a", "y"), ("a", "z")]));
    let indexed: Vec<(usize, String)> = output.into_iter().map(|(_, iv)| iv).collect();
    assert_eq!(
        indexed,
        vec![(0, "x".into()), (1, "y".into()), (2, "z".into())]
    );
}
