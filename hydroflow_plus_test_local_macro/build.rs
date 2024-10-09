use std::path::Path;

fn main() {
    stageleft_tool::gen_macro(
        Path::new("../hydroflow_plus_test_local"),
        "hydroflow_plus_test_local",
    );
}
