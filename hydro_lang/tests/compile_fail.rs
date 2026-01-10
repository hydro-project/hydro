#[test]
fn test_all() {
    let t = trybuild::TestCases::new();
    #[cfg(nightly)]
    let path = "tests/compile-fail/*.rs";
    #[cfg(not(nightly))]
    let path = "tests/compile-fail-stable/*.rs";
    t.compile_fail(path);
}
