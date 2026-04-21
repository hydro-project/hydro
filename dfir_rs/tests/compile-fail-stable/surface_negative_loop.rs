use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        diff = difference();
        source_iter([1]) -> [pos]diff;
        diff -> [neg]diff;
    };
    df.run_available_sync();
}
