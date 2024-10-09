use hydroflow::hydroflow_syntax;

pub fn main() {
    let mut flow = hydroflow_syntax! {
        source_iter(vec!["Hello", "world"]) -> upper_print;
        upper_print = map(|x| x.to_uppercase()) -> for_each(|x| println!("{}", x));
    };
    flow.run_available();
}
