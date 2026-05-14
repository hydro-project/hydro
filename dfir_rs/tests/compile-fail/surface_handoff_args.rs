use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        source_iter(0..10) -> handoff(42) -> for_each(std::mem::drop);
    };
    df.run_available_sync();
}
