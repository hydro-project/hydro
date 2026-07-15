#![expect(unused_mut, unused_variables, reason = "example code")]

use dfir_rs::dfir_syntax;

fn main() {
    let mut flow = dfir_syntax! {
        // DFIR syntax goes here
    };
}

#[cfg(not(nightly))]
#[test]
fn test() {
    use example_test::run_current_example;

    let output = run_current_example!().read_to_end();
    hydro_build_utils::assert_snapshot!("example_syntax_empty", output);
}
