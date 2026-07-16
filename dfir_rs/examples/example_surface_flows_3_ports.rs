use dfir_rs::dfir_syntax;

pub fn main() {
    let mut flow = dfir_syntax! {
        my_tee = source_iter(vec!["Hello", "world"]) -> tee();
        my_tee -> map(|x| x.to_uppercase()) -> [low_road]my_union;
        my_tee -> map(|x| x.to_lowercase()) -> [high_road]my_union;
        my_union = union() -> for_each(|x| println!("{}", x));
    };
    println!(
        "{}",
        flow.meta_graph()
            .expect("No graph found, maybe failed to parse.")
            .to_mermaid(&Default::default())
    );
    flow.run_available_sync();
}

#[cfg(not(nightly))]
#[test]
fn test() {
    use example_test::run_current_example;

    let output = run_current_example!().read_to_end();
    hydro_build_utils::assert_snapshot!("example_surface_flows_3_ports", output);
}
