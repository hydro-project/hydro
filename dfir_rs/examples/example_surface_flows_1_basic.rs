use dfir_rs::dfir_syntax;

pub fn main() {
    let mut flow = dfir_syntax! {
        source_iter(vec!["Hello", "world"])
            -> map(|x| x.to_uppercase()) -> for_each(|x| println!("{}", x));
    };
    flow.run_available_sync();
}
