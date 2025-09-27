#[cfg(feature = "sim")]
extern crate bolero; // TODO(shadaj): fixes linkage issues because I think the libfuzzer stuff becomes lazy

#[test]
fn test_all() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile-fail/*.rs");
}
