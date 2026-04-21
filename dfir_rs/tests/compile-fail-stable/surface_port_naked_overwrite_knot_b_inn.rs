use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        pivot = union() -> tee();

        x_0 = [0]pivot;
        x_1 = [1]pivot;

        x_0 -> [0]x_0;
        x_1[0] -> [1]x_1; // Error: `[1][1]pivot`
    };
    df.run_available_sync();
}
