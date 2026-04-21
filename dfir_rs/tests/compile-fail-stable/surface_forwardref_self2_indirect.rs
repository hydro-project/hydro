use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        g = f -> map(|x: usize| x) -> f;
        f = g;
    };
    df.run_available_sync();
}
