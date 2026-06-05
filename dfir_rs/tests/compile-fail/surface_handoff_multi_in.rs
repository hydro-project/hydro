use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        source_iter(0..5) -> my_handoff;
        source_iter(5..10) -> my_handoff;
        my_handoff = handoff() -> for_each(std::mem::drop);
    };
    df.run_available_sync();
}
