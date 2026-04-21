use dfir_rs::dfir_syntax_inline;

fn main() {
    let mut df = dfir_syntax_inline! {
        source_iter(["Hello", "World"])
            -> map(|s| s.to_uppercase())
            -> for_each(|s| println!("{}", s));
    };

    df.run_available_sync();
}
