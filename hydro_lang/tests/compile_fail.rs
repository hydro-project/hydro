// `hydro_lang` is processed by the Verus driver (see `[package.metadata.verus]`), which
// requires the `vstd` prelude in every crate it verifies, including test targets.
// Erased under normal compilation.
#[cfg(all(verus_keep_ghost, feature = "verus"))]
#[allow(unused_imports, reason = "required by the Verus driver")]
use vstd::prelude::*;

#[test]
fn test_all() {
    hydro_build_utils::trybuild_compile_fail!("*.rs");
}
