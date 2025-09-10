use dfir_rs::dfir_syntax;

pub fn main() {
    let mut flow = dfir_syntax! {
        source_iter([()])
            -> for_each(|()| println!("Current tick: {}, stratum: {}", context.current_tick(), context.current_stratum()));
    };
    flow.run_available_sync();
}
