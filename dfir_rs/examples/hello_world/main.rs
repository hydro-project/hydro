use dfir_rs::dfir_syntax_inline;

pub fn main() {
    let mut df = dfir_syntax_inline! {
        source_iter(["Hello World"])
            -> assert_eq(["Hello World"]);
    };
    df.run_available_sync();
}

#[test]
fn test() {
    main();
}
