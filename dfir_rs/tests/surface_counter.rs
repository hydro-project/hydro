use std::time::Duration;

use dfir_rs::dfir_syntax;
use dfir_rs::scheduled::graph::Dfir;
use dfir_rs::util::iter_batches_stream;
use multiplatform_test::multiplatform_test;

fn fib(n: u64) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => fib(n - 1) + fib(n - 2),
    }
}

#[multiplatform_test(dfir)]
pub async fn test_fib() {
    let mut df: Dfir = dfir_syntax! {
        source_stream(iter_batches_stream(0..=40, 1))
            -> map(fib)
            -> _counter("nums", Duration::from_millis(50));
    };

    df.run_available().await;
    // _counter(nums): 1
    // _counter(nums): 34
    // _counter(nums): 36
    // _counter(nums): 38
    // _counter(nums): 39
    // _counter(nums): 40
}

#[multiplatform_test(dfir)]
pub async fn test_stream() {
    let mut df: Dfir = dfir_syntax! {
        source_stream(iter_batches_stream(0..=100_000, 1))
            -> _counter("nums", Duration::from_millis(100));
    };

    df.run_available().await;
    // _counter(nums): 1
    // _counter(nums): 6202
    // _counter(nums): 12540
    // _counter(nums): 18876
    // _counter(nums): 25218
    // _counter(nums): 31557
    // _counter(nums): 37893
    // _counter(nums): 44220
    // _counter(nums): 50576
    // _counter(nums): 56909
    // _counter(nums): 63181
    // _counter(nums): 69549
    // _counter(nums): 75914
    // _counter(nums): 82263
    // _counter(nums): 88638
    // _counter(nums): 94980
}
