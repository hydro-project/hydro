#[test]
fn test_all() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile-fail/surface_*.rs");
}

#[test]
fn test_one() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile-fail/surface_dest_sink_badsink.rs");
}
