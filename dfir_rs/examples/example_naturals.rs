use dfir_rs::dfir_syntax;

pub fn main() {
    let mut _flow = dfir_syntax! {
        base = source_iter(vec![1]) -> cycle;
        cycle = union()
                -> map(|i| i + 1)
                -> inspect(|i| println!("{}", i))
                -> cycle;
    };

    // Let's not run this -- it will go forever!
    // flow.run_available_sync();
}
