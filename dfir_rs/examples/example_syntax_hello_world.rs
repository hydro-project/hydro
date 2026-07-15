use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        source_iter(["Hello", "World"])
            -> map(|s| s.to_uppercase())
            -> for_each(|s| println!("{}", s));
    };

    df.run_available_sync();
}

#[cfg(not(nightly))]
#[test]
fn test() {
    use example_test::run_current_example;

    let output = run_current_example!().read_to_end();
    hydro_build_utils::assert_snapshot!("example_syntax_hello_world", output);
}
