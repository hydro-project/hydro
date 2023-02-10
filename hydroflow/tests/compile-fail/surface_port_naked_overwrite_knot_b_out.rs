use hydroflow::hydroflow_syntax;

fn main() {
    let mut df = hydroflow_syntax! {
        pivot = merge() -> tee();

        x_0 = pivot[0];
        x_1 = pivot[1];

        x_0 -> [0]x_0;
        x_1[0] -> [1]x_1; // Error: `pivot[1][0]`
    };
    df.run_available();
}
