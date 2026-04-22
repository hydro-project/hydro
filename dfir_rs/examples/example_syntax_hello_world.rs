use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        source_iter(["Hello", "World"])
            -> map(|s| s.to_uppercase())
            -> for_each(|s| println!("{}", s));
    };

    df.run_available_sync();
}
