pub mod rustc_version {
    pub use rustc_version::*;
}

#[macro_export]
macro_rules! emit_nightly_configuration {
    () => {
        println!("cargo:rerun-if-env-changed=RUSTC_BOOTSTRAP");
        println!("cargo::rustc-check-cfg=cfg(nightly)");
        if matches!(
            $crate::rustc_version::version_meta().map(|meta| meta.channel),
            Ok($crate::rustc_version::Channel::Nightly)
        ) || option_env!("RUSTC_BOOTSTRAP") == Some("1")
        {
            println!("cargo:rustc-cfg=nightly");
        }
    };
}

pub mod insta {
    pub use ::insta::*;
}

#[macro_export]
macro_rules! nightly_wrapper {
    ($statement:stmt) => {
        $crate::insta::with_settings!({
            snapshot_path => if cfg!(nightly) { "snapshots-nightly" } else { "snapshots" }
        }, {
            $statement;
        });
    }
}

#[macro_export]
macro_rules! assert_snapshot {
    ($($arg:tt)*) => {
        $crate::nightly_wrapper!($crate::insta::assert_snapshot!($($arg)*));
    };
}

#[macro_export]
macro_rules! assert_debug_snapshot {
    ($($arg:tt)*) => {
        $crate::nightly_wrapper!($crate::insta::assert_debug_snapshot!($($arg)*));
    };
}
