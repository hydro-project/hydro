use dfir_rs::dfir_syntax;

pub fn main() {
    let mut flow = dfir_syntax! {
        source_iter(0..10)
            -> map(|n| n * n)
            -> filter(|n| *n > 10)
            -> map(|n| n..=n+1)
            -> flatten()
            -> for_each(|n| println!("Howdy {}", n));
    };

    flow.run_available_sync();
}

#[cfg(not(nightly))]
#[test]
fn test() {
    use example_test::run_current_example;

    let output = run_current_example!().read_to_end();
    hydro_build_utils::assert_snapshot!("example_2_simple_1", output);
}
