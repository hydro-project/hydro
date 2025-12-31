#[test]
fn test_all() {
    let t = trybuild::TestCases::new();
    #[cfg(nightly)]
    let path = "tests/compile-fail-nightly/*.rs";
    #[cfg(not(nightly))]
    let path = "tests/compile-fail/*.rs";
    t.compile_fail(path);
}
