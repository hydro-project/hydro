use dfir_rs::dfir_syntax_inline;

pub fn main() {
    let mut flow = dfir_syntax_inline! {
        source_iter(0..10)
            -> map(|n| n * n)
            -> filter(|n| *n > 10)
            -> map(|n| n..=n+1)
            -> flatten()
            -> for_each(|n| println!("Howdy {}", n));
    };

    flow.run_available_sync();
}
