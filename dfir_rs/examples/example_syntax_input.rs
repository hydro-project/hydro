use dfir_rs::dfir_syntax;

fn main() {
    let (input_send, input_recv) = dfir_rs::util::unbounded_channel::<&str>();
    let mut flow = dfir_syntax! {
        source_stream(input_recv) -> map(|x| x.to_uppercase())
            -> for_each(|x| println!("{}", x));
    };
    input_send.send("Hello").unwrap();
    input_send.send("World").unwrap();
    flow.run_available_sync();
}

#[cfg(not(nightly))]
#[test]
fn test() {
    use example_test::run_current_example;

    let output = run_current_example!().read_to_end();
    hydro_build_utils::assert_snapshot!("example_syntax_input", output);
}
