use dfir_rs::dfir_syntax;

pub fn main() {
    let mut flow = dfir_syntax! {
        source_iter(vec!["Hello", "world"])
            -> map(|x| x.to_uppercase()) -> for_each(|x| println!("{}", x));
    };
    flow.run_available_sync();
}

#[cfg(not(nightly))]
#[test]
fn test() {
    use example_test::run_current_example;

    let output = run_current_example!().read_to_end();
    hydro_build_utils::assert_snapshot!("example_surface_flows_1_basic", output);
}
