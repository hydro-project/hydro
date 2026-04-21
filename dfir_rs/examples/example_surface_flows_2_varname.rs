use dfir_rs::dfir_syntax_inline;

pub fn main() {
    let mut flow = dfir_syntax_inline! {
        source_iter(vec!["Hello", "world"]) -> upper_print;
        upper_print = map(|x| x.to_uppercase()) -> for_each(|x| println!("{}", x));
    };
    flow.run_available_sync();
}
