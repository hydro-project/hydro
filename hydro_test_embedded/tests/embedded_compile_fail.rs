//! Compile-fail snapshots of the errors a host gets when it does not promise the guarantees
//! the generated embedded structs require.

#![cfg(feature = "test_embedded")]

#[test]
fn test_all() {
    hydro_build_utils::trybuild_compile_fail!("embedded_*.rs");
}
