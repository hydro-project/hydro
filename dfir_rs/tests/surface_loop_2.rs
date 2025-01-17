// use dfir_rs::util::iter_batches_stream;
use dfir_rs::{assert_graphvis_snapshots, dfir_syntax};
use multiplatform_test::multiplatform_test;

#[multiplatform_test(test, wasm, env_tracing)]
pub fn test_flo_repeat_n_nested() {
    let mut df = dfir_syntax! {
        usrs1 = source_iter(["alice", "bob"]);
        loop {
            usrs2 = usrs1 -> batch()
                -> inspect(|x| println!("{}: {:?}", line!(), x))
                -> flatten()
                -> enumerate();
            loop {
                usrs3 = usrs2 -> repeat_n(3)
                    -> inspect(|x| println!("{}: {:?}", line!(), x))
                    -> flatten()
                    -> enumerate();
                loop {
                    usrs3 -> repeat_n(3)
                        -> inspect(|x| println!("x {:?}", x))
                        -> null();
                        // assert_eq([
                        //     vec!["alice", "bob", "alice", "bob", "alice", "bob"],
                        //     vec!["alice", "bob", "alice", "bob", "alice", "bob"],
                        //     vec!["alice", "bob", "alice", "bob", "alice", "bob"],
                        // ]);
                }
            }
        }
    };
    assert_graphvis_snapshots!(df);
    df.run_available();
}
