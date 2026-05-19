#[cfg(feature = "insta")]
pub use insta;
pub use rustc_version;
#[cfg(feature = "trybuild")]
pub use trybuild;

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

#[cfg(feature = "insta")]
#[macro_export]
macro_rules! nightly_wrapper {
    ($statement:stmt) => {
        $crate::insta::with_settings!({
            prepend_module_to_snapshot => option_env!("CARGO_TARGET_TMPDIR").is_some(), // Only for integration tests.
            snapshot_path => if cfg!(nightly) { "snapshots-nightly" } else { "snapshots" },
        }, {
            $statement;
        });
    }
}

#[cfg(feature = "insta")]
#[macro_export]
macro_rules! assert_snapshot {
    ($($arg:tt)*) => {
        $crate::nightly_wrapper!($crate::insta::assert_snapshot!($($arg)*));
    };
}

#[cfg(feature = "insta")]
#[macro_export]
macro_rules! assert_debug_snapshot {
    ($($arg:tt)*) => {
        $crate::nightly_wrapper!($crate::insta::assert_debug_snapshot!($($arg)*));
    };
}

#[cfg(feature = "trybuild")]
#[macro_export]
macro_rules! trybuild_compile_fail {
    ($glob:expr) => {{
        let source_dir = std::path::Path::new("tests/compile-fail");
        let stderr_dir = source_dir.join(if cfg!(nightly) { "nightly" } else { "stable" });

        // Remove any existing .rs files/symlinks in the channel dir first.
        // Stale broken symlinks (e.g. from rebasing) break trybuild.
        for entry in std::fs::read_dir(&stderr_dir).unwrap().flatten() {
            if entry.path().extension().and_then(|e| e.to_str()) == Some("rs") {
                let _ = std::fs::remove_file(entry.path());
            }
        }
        // Symlink all .rs files from the source directory into the channel-specific
        // stderr directory so trybuild can find them next to the .stderr files.
        for entry in std::fs::read_dir(source_dir).unwrap().flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                let dest = stderr_dir.join(entry.file_name());
                let original = std::path::Path::new("..").join(entry.file_name());
                #[cfg(unix)]
                std::os::unix::fs::symlink(&original, &dest).unwrap();
                #[cfg(windows)]
                std::os::windows::fs::symlink_file(&original, &dest).unwrap();
            }
        }

        let pattern = stderr_dir.join($glob);
        let t = $crate::trybuild::TestCases::new();
        t.compile_fail(pattern.to_str().unwrap());
    }};
}
