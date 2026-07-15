//[use]//
use dfir_rs::dfir_syntax;
//[/use]//

//[macro_call]//
pub fn main() {
    let mut flow = dfir_syntax! {
        source_iter(0..10) -> for_each(|n| println!("Hello {}", n));
    };
    //[/macro_call]//

    //[run]//
    flow.run_available_sync();
    //[/run]//
}

#[cfg(not(nightly))]
#[test]
fn test() {
    use example_test::run_current_example;

    let output = run_current_example!().read_to_end();
    hydro_build_utils::assert_snapshot!("example_1_simplest", output);
}
