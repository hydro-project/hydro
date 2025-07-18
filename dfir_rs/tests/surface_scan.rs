use std::collections::HashMap;

use dfir_rs::assert_graphvis_snapshots;
use dfir_rs::scheduled::ticks::TickInstant;
use dfir_rs::util::collect_ready;
use multiplatform_test::multiplatform_test;

// Basic scan operator tests

#[multiplatform_test]
pub fn test_scan_tick() {
    // Test scan with 'tick' persistence
    let (items_send, items_recv) = dfir_rs::util::unbounded_channel::<u32>();
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<u32>();

    let mut df = dfir_rs::dfir_syntax! {
        source_stream(items_recv)
            -> scan::<'tick>(|| 0, |acc: &mut u32, x: u32| {
                *acc += x;
                Some(*acc)
            })
            -> for_each(|v| result_send.send(v).unwrap());
    };
    assert_graphvis_snapshots!(df);

    assert_eq!(
        (TickInstant::new(0), 0),
        (df.current_tick(), df.current_stratum())
    );

    // First tick with values 1, 2
    items_send.send(1).unwrap();
    items_send.send(2).unwrap();
    df.run_tick();

    assert_eq!(
        (TickInstant::new(1), 0),
        (df.current_tick(), df.current_stratum())
    );

    // Should receive running sums: 1, 3
    assert_eq!(&[1, 3], &*collect_ready::<Vec<_>, _>(&mut result_recv));

    // Second tick with values 3, 4
    items_send.send(3).unwrap();
    items_send.send(4).unwrap();
    df.run_tick();

    assert_eq!(
        (TickInstant::new(2), 0),
        (df.current_tick(), df.current_stratum())
    );

    // With 'tick' persistence, accumulator resets each tick
    // So we should get: 3, 7 (not 6, 10)
    assert_eq!(&[3, 7], &*collect_ready::<Vec<_>, _>(&mut result_recv));

    df.run_available(); // Should return quickly and not hang
}

#[multiplatform_test]
pub fn test_scan_static() {
    // Test scan with 'static' persistence
    let (items_send, items_recv) = dfir_rs::util::unbounded_channel::<u32>();
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<u32>();

    let mut df = dfir_rs::dfir_syntax! {
        source_stream(items_recv)
            -> scan::<'static>(|| 0, |acc: &mut u32, x: u32| {
                *acc += x;
                Some(*acc)
            })
            -> for_each(|v| result_send.send(v).unwrap());
    };
    assert_graphvis_snapshots!(df);

    assert_eq!(
        (TickInstant::new(0), 0),
        (df.current_tick(), df.current_stratum())
    );

    // First tick with values 1, 2
    items_send.send(1).unwrap();
    items_send.send(2).unwrap();
    df.run_tick();

    assert_eq!(
        (TickInstant::new(1), 0),
        (df.current_tick(), df.current_stratum())
    );

    // Should receive running sums: 1, 3
    assert_eq!(&[1, 3], &*collect_ready::<Vec<_>, _>(&mut result_recv));

    // Second tick with values 3, 4
    items_send.send(3).unwrap();
    items_send.send(4).unwrap();
    df.run_tick();

    assert_eq!(
        (TickInstant::new(2), 0),
        (df.current_tick(), df.current_stratum())
    );

    // With 'static' persistence, accumulator persists across ticks
    // So we should get: 6, 10 (continuing from previous sum of 3)
    assert_eq!(&[6, 10], &*collect_ready::<Vec<_>, _>(&mut result_recv));

    df.run_available(); // Should return quickly and not hang
}

// Edge case tests

#[multiplatform_test]
pub fn test_scan_empty_input() {
    // Test scan with empty input
    let (_items_send, items_recv) = dfir_rs::util::unbounded_channel::<u32>();
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<u32>();

    let mut df = dfir_rs::dfir_syntax! {
        source_stream(items_recv)
            -> scan::<'tick>(|| 0, |acc: &mut u32, x: u32| {
                *acc += x;
                Some(*acc)
            })
            -> for_each(|v| result_send.send(v).unwrap());
    };

    // Run a tick without sending any items
    df.run_tick();

    // Should receive no results since there were no inputs
    assert_eq!(
        &[] as &[u32],
        &*collect_ready::<Vec<_>, _>(&mut result_recv)
    );

    df.run_available(); // Should return quickly and not hang
}

#[multiplatform_test]
pub fn test_scan_different_types() {
    // Test scan with different accumulator and output types
    let (items_send, items_recv) = dfir_rs::util::unbounded_channel::<String>();
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<(usize, String)>();

    let mut df = dfir_rs::dfir_syntax! {
        source_stream(items_recv)
            -> scan::<'tick>(|| (0, String::new()), |acc: &mut (usize, String), x: String| {
                // Accumulator is a tuple of (count, concatenated_string)
                acc.0 += 1;
                if !acc.1.is_empty() {
                    acc.1.push_str(", ");
                }
                acc.1.push_str(&x);
                // Clone the accumulator for the output since it doesn't implement Copy
                Some((acc.0, acc.1.clone()))
            })
            -> for_each(|v| result_send.send(v).unwrap());
    };

    // Send some strings
    items_send.send("hello".to_string()).unwrap();
    items_send.send("world".to_string()).unwrap();
    df.run_tick();

    // Should receive tuples with count and concatenated strings
    let results = collect_ready::<Vec<_>, _>(&mut result_recv);
    assert_eq!(2, results.len());
    assert_eq!((1, "hello".to_string()), results[0]);
    assert_eq!((2, "hello, world".to_string()), results[1]);

    df.run_available(); // Should return quickly and not hang
}

#[multiplatform_test]
pub fn test_scan_early_termination() {
    // Test scan with early termination (returning None)
    let (items_send, items_recv) = dfir_rs::util::unbounded_channel::<u32>();
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<u32>();

    let mut df = dfir_rs::dfir_syntax! {
        source_stream(items_recv)
            -> scan::<'tick>(|| 0, |acc: &mut u32, x: u32| {
                *acc += x;
                // Terminate when accumulator exceeds 5
                if *acc > 5 {
                    None
                } else {
                    Some(*acc)
                }
            })
            -> for_each(|v| result_send.send(v).unwrap());
    };

    // Send values that will cause the accumulator to exceed 5
    items_send.send(1).unwrap(); // acc = 1
    items_send.send(2).unwrap(); // acc = 3
    items_send.send(3).unwrap(); // acc = 6 > 5, should terminate
    items_send.send(4).unwrap(); // This should be ignored due to termination
    df.run_tick();

    // Should only receive values before termination: 1, 3
    assert_eq!(&[1, 3], &*collect_ready::<Vec<_>, _>(&mut result_recv));

    // Send more values in a new tick
    items_send.send(1).unwrap();
    items_send.send(2).unwrap();
    df.run_tick();

    // With 'tick' persistence, accumulator resets each tick
    // So we should get new values: 1, 3
    assert_eq!(&[1, 3], &*collect_ready::<Vec<_>, _>(&mut result_recv));

    df.run_available(); // Should return quickly and not hang
}

#[multiplatform_test]
pub fn test_scan_static_early_termination() {
    // Test scan with 'static' persistence and early termination
    let (items_send, items_recv) = dfir_rs::util::unbounded_channel::<u32>();
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<u32>();

    let mut df = dfir_rs::dfir_syntax! {
        source_stream(items_recv)
            -> scan::<'static>(|| 0, |acc: &mut u32, x: u32| {
                *acc += x;
                // Terminate when accumulator exceeds 5
                if *acc > 5 {
                    None
                } else {
                    Some(*acc)
                }
            })
            -> for_each(|v| result_send.send(v).unwrap());
    };

    // First tick: Send values that will cause the accumulator to exceed 5
    items_send.send(1).unwrap(); // acc = 1
    items_send.send(2).unwrap(); // acc = 3
    items_send.send(3).unwrap(); // acc = 6 > 5, should terminate
    items_send.send(4).unwrap(); // This should be ignored due to termination
    df.run_tick();

    // Should only receive values before termination: 1, 3
    assert_eq!(&[1, 3], &*collect_ready::<Vec<_>, _>(&mut result_recv));

    // Second tick: Send more values
    items_send.send(1).unwrap();
    items_send.send(2).unwrap();
    df.run_tick();

    // With 'static' persistence, termination state is preserved across ticks
    // So we should get no values in the second tick
    assert_eq!(
        &[] as &[u32],
        &*collect_ready::<Vec<_>, _>(&mut result_recv)
    );

    df.run_available(); // Should return quickly and not hang
}

#[multiplatform_test]
pub fn test_scan_complex_accumulator() {
    // Test scan with a more complex accumulator (HashMap)
    let (items_send, items_recv) = dfir_rs::util::unbounded_channel::<(String, u32)>();
    let (result_send, mut result_recv) = dfir_rs::util::unbounded_channel::<HashMap<String, u32>>();

    let mut df = dfir_rs::dfir_syntax! {
        source_stream(items_recv)
            -> scan::<'tick>(HashMap::<String, u32>::new, |acc: &mut HashMap<String, u32>, item: (String, u32)| {
                // Update frequency count for each key
                let entry = acc.entry(item.0).or_insert(0);
                *entry += item.1;
                // Return a clone of the current state
                Some(acc.clone())
            })
            -> for_each(|v| result_send.send(v).unwrap());
    };

    // Send some key-value pairs
    items_send.send(("apple".to_string(), 1)).unwrap();
    items_send.send(("banana".to_string(), 2)).unwrap();
    items_send.send(("apple".to_string(), 3)).unwrap();
    df.run_tick();

    // Should receive HashMaps with accumulated counts
    let results = collect_ready::<Vec<_>, _>(&mut result_recv);
    assert_eq!(3, results.len());

    // First result: {"apple": 1}
    let mut expected1 = HashMap::new();
    expected1.insert("apple".to_string(), 1);
    assert_eq!(expected1, results[0]);

    // Second result: {"apple": 1, "banana": 2}
    let mut expected2 = HashMap::new();
    expected2.insert("apple".to_string(), 1);
    expected2.insert("banana".to_string(), 2);
    assert_eq!(expected2, results[1]);

    // Third result: {"apple": 4, "banana": 2}
    let mut expected3 = HashMap::new();
    expected3.insert("apple".to_string(), 4);
    expected3.insert("banana".to_string(), 2);
    assert_eq!(expected3, results[2]);

    df.run_available(); // Should return quickly and not hang
}
