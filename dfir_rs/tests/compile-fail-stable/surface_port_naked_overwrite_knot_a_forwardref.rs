use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        out_1 -> inn_1;
        out_0[0] -> inn_0; // Error: `pivot[0][0]`

        out_1 = pivot[1];
        out_0 = pivot[0];

        inn_1 = [1]pivot;
        inn_0 = [0]pivot;

        pivot = union() -> tee();
    };
    df.run_available_sync();
}
