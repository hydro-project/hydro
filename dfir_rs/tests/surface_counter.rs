use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use dfir_rs::dfir_syntax_inline;
use dfir_rs::util::iter_batches_stream;
use multiplatform_test::multiplatform_test;
use web_time::Duration;

fn fib(n: u64) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => fib(n - 1) + fib(n - 2),
    }
}

#[multiplatform_test(dfir)]
pub async fn test_fib() {
    let output = Rc::new(RefCell::new(Vec::new()));
    let output_ref = output.clone();

    let mut df = dfir_syntax_inline! {
        source_stream(iter_batches_stream(0..=40, 1))
            -> map(fib)
            -> _counter("_counter(nums)", Duration::from_millis(50))
            -> for_each(|x| output_ref.borrow_mut().push(x));
    };

    df.run_available().await;

    let result = output.borrow();
    assert_eq!(result.len(), 41);
    assert_eq!(result[0], 0); // fib(0)
    assert_eq!(result[1], 1); // fib(1)
    assert_eq!(result[10], 55); // fib(10)
}

#[multiplatform_test(dfir)]
pub async fn test_stream() {
    let count = Rc::new(RefCell::new(0u64));
    let count_ref = count.clone();

    let mut df = dfir_syntax_inline! {
        source_stream(iter_batches_stream(0..=100_000, 1))
            -> _counter("_counter(nums)", Duration::from_millis(100))
            -> for_each(|_| *count_ref.borrow_mut() += 1);
    };

    df.run_available().await;

    assert_eq!(*count.borrow(), 100_001);
}

#[multiplatform_test(dfir)]
pub async fn test_pull() {
    let output = Rc::new(RefCell::new(Vec::new()));
    let output_ref = output.clone();

    let mut df = dfir_syntax_inline! {
        source_iter(0..10)
            -> _counter("_counter(pull_test)", Duration::from_millis(50))
            -> for_each(|x| output_ref.borrow_mut().push(x));
    };

    df.run_available().await;

    assert_eq!(*output.borrow(), (0..10).collect::<Vec<_>>());
}

/// Verify that the counter's background task is actually spawned and runs.
/// This would fail if `spawn_local` were replaced with a no-op.
#[multiplatform_test(dfir)]
pub async fn test_counter_task_spawns() {
    let flag = Arc::new(AtomicBool::new(false));
    let flag_clone = flag.clone();

    // Spawn a sentinel task to verify spawn_local works in this context.
    tokio::task::spawn_local(async move {
        flag_clone.store(true, Ordering::Relaxed);
    });

    let mut df = dfir_syntax_inline! {
        source_iter(0..5)
            -> _counter("_counter(spawn_test)", Duration::from_millis(10));
    };

    df.run_available().await;

    // Yield to let spawned tasks run.
    tokio::task::yield_now().await;

    assert!(
        flag.load(Ordering::Relaxed),
        "spawn_local tasks must actually execute; if this fails, the counter's background task is also broken"
    );
}
