use dfir_rs::dfir_syntax;
use multiplatform_test::multiplatform_test;

#[multiplatform_test]
pub fn test_basic_null_src() {
    let mut df = dfir_syntax! {
        null() -> for_each(drop::<String>);
    };
    df.run_available_sync();
}

#[multiplatform_test]
pub fn test_basic_null_dest() {
    let mut df = dfir_syntax! {
        source_iter([1, 2, 3, 4]) -> null();
    };
    df.run_available_sync();
}

#[multiplatform_test]
pub fn test_basic_null_both() {
    let mut df = dfir_syntax! {
        source_iter([1, 2, 3, 4]) -> null() -> for_each(drop::<String>);
    };
    df.run_available_sync();
}
