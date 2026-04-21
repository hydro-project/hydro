use dfir_rs::dfir_syntax_inline;

pub fn main() {
    let mut flow = dfir_syntax_inline! {
        source_iter([()])
            -> for_each(|()| println!("Current tick: {}", context.current_tick()));
    };
    flow.run_available_sync();
}
