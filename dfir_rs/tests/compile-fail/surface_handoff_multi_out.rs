use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        source_iter(0..10) -> my_handoff;
        my_handoff = handoff();
        my_handoff -> for_each(|x| println!("a: {x}"));
        my_handoff -> for_each(|x| println!("b: {x}"));
    };
    df.run_available_sync();
}
