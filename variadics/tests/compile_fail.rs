#[test]
fn test_all() {
    let t = trybuild::TestCases::new();
    #[cfg(nightly)]
    let path = "tests/compile-fail-nightly/surface_*.rs";
    #[cfg(not(nightly))]
    let path = "tests/compile-fail/surface_*.rs";
    t.compile_fail(path);
}
