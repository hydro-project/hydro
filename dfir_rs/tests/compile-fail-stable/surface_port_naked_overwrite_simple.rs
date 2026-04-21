use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        id = identity();
        inn = [0]id;
        out = id[0];
        out[0] -> [0]inn; // Error, equivalent to: `[0][0]id[0][0]`
    };
    df.run_available_sync();
}
