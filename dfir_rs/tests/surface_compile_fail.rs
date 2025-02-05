#[test]
fn test_all() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile-fail/surface_*.rs");
}
