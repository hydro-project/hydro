use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        pivot = union() -> tee();

        x_0 = pivot[0];
        x_1 = pivot[1];

        x_0 -> [0]x_0;
        x_1[0] -> [1]x_1; // Error: `pivot[1][0]`
    };
    df.run_available_sync();
}
