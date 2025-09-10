use std::cell::Cell;
use std::rc::Rc;

use dfir_macro::dfir_test;
use dfir_rs::{assert_graphvis_snapshots, dfir_syntax};
use multiplatform_test::multiplatform_test;
use web_time::Duration;

#[multiplatform_test]
pub fn test_context_ref() {
    let mut df = dfir_syntax! {
        source_iter([()])
            -> for_each(|()| println!("Current tick: {}, stratum: {}", context.current_tick(), context.current_stratum()));
    };
    assert_graphvis_snapshots!(df);
    df.run_available_sync();
}

#[multiplatform_test]
pub fn test_context_mut() {
    // TODO(mingwei): Currently cannot have conflicting (mut) references to `context` in the same
    // subgraph - bit of a leak of the subgraphs abstraction. `next_stratum()` here so it runs.
    let mut df = dfir_syntax! {
        source_iter(0..10)
            -> map(|n| context.add_state(n))
            -> next_stratum()
            -> for_each(|handle| println!("{:?}: {}", handle, context.state_ref(handle)));
    };
    assert_graphvis_snapshots!(df);
    df.run_available_sync();
}

#[multiplatform_test(dfir)]
pub async fn test_context_current_tick_start_does_not_count_time_between_ticks_async() {
    let time = Rc::new(Cell::new(None));

    let mut df = {
        let time = time.clone();
        dfir_syntax! {
            source_iter([()])
                -> persist::<'static>()
                -> for_each(|_| time.set(Some(context.current_tick_start().elapsed().unwrap())));
        }
    };
    assert_graphvis_snapshots!(df);
    tokio::time::sleep(Duration::from_millis(100)).await;
    df.run_tick().await;
    assert!(time.take().unwrap() < Duration::from_millis(50));

    tokio::time::sleep(Duration::from_millis(100)).await;
    df.run_available().await;
    assert!(time.take().unwrap() < Duration::from_millis(50));
}

#[dfir_test]
pub async fn test_defered_tick_and_no_io_with_run_async() {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    let mut df = dfir_syntax! {
        source_iter([()])
            -> defer_tick()
            -> for_each(|_| tx.send(()).unwrap());
    };

    tokio::select! {
        _ = df.run() => {},
        _ = rx.recv() => {},
    }
}
