#[test]
fn test_all() {
    hydro_build_utils::trybuild_compile_fail!("*.rs");
}
