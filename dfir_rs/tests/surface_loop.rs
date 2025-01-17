use dfir_rs::util::iter_batches_stream;
use dfir_rs::{assert_graphvis_snapshots, dfir_syntax};
use multiplatform_test::multiplatform_test;

#[multiplatform_test(test, wasm, env_tracing)]
pub fn test_flo_syntax() {
    let mut df = dfir_syntax! {
        users = source_iter(["alice", "bob"]);
        messages = source_stream(iter_batches_stream(0..12, 3));
        loop {
            // TODO(mingwei): cross_join type negotion should allow us to eliminate `flatten()`.
            users -> batch() -> flatten() -> [0]cp;
            messages -> batch() -> flatten() -> [1]cp;
            cp = cross_join::<'static, 'tick>() -> for_each(|(user, message)| println!("{}: notify {} of {}", context.current_tick(), user, message));
        }
    };
    assert_graphvis_snapshots!(df);
    df.run_available();
}

#[multiplatform_test(test, wasm, env_tracing)]
pub fn test_flo_nested() {
    let mut df = dfir_syntax! {
        users = source_iter(["alice", "bob"]);
        messages = source_stream(iter_batches_stream(0..12, 3));
        loop {
            // TODO(mingwei): cross_join type negotion should allow us to eliminate `flatten()`.
            users -> batch() -> flatten() -> [0]cp;
            messages -> batch() -> flatten() -> [1]cp;
            cp = cross_join::<'static, 'tick>();
            loop {
                cp -> all_once() -> for_each(|all| println!("{}: {:?}", context.current_tick(), all));
            }
        }
    };
    assert_graphvis_snapshots!(df);
    df.run_available();
}
