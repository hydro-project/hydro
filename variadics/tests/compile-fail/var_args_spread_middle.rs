use variadics::*;

fn main() {
    let var_args!(_a, ..._b, _c) = var_expr!(1, 2.0, "three", false);
}