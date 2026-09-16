use std::path::Path;
use std::process::Command;

#[expect(dead_code, reason = "main is the standalone wrapper entrypoint")]
#[path = "coverage_wrapper.rs"]
mod wrapper;

pub(super) fn requested() -> bool {
    if std::env::var_os("LLVM_PROFILE_FILE").is_some() {
        return true;
    }
    // Match Cargo's environment precedence, including an explicitly empty encoded
    // value. Do not interpret substrings in paths, cfg values, or linker arguments.
    if let Some(flags) = std::env::var_os("CARGO_ENCODED_RUSTFLAGS") {
        flags
            .to_str()
            .is_some_and(|flags| wrapper::coverage_setting(flags.split('\x1f')) == Some(true))
    } else if let Some(flags) = std::env::var_os("RUSTFLAGS") {
        flags
            .to_str()
            .is_some_and(|flags| wrapper::coverage_setting(flags.split_whitespace()) == Some(true))
    } else {
        false
    }
}

/// Install an additive wrapper, without replacing any of Cargo's rustflag sources.
/// Keep the returned directory alive until Cargo and all its children have exited.
pub(super) fn configure(command: &mut Command, project_dir: &Path) -> tempfile::TempDir {
    // Only use the config reader for program paths and build.target. Cargo itself
    // must resolve rustflags, including target cfgs, env precedence, and host units.
    let mut config = cargo_config2::Config::load_with_options(
        project_dir,
        cargo_config2::ResolveOptions::default()
            .env(std::env::vars_os().filter(|(key, _)| key == "CARGO_BUILD_TARGET")),
    )
    .expect("failed to read Cargo compiler configuration");
    // Program paths are OsStrings, not rustflag text. Keep non-Unicode paths intact.
    // Do not ask the config reader to decode unrelated environment settings: Cargo
    // remains responsible for interpreting those, including encoded rustflags.
    if let Some(rustc) = std::env::var_os("RUSTC").or_else(|| std::env::var_os("CARGO_BUILD_RUSTC"))
    {
        config.build.rustc = Some(rustc.into());
    }
    config.build.rustc_wrapper = None;
    config.build.rustc_workspace_wrapper = None;
    let compiler = &config.rustc().path;
    let directory = tempfile::tempdir().expect("failed to create coverage wrapper directory");
    let source = directory.path().join("coverage_wrapper.rs");
    let executable = directory
        .path()
        .join(format!("coverage-wrapper{}", std::env::consts::EXE_SUFFIX));
    std::fs::write(&source, include_str!("coverage_wrapper.rs")).unwrap();
    let status = Command::new(compiler)
        .current_dir(project_dir)
        .arg(&source)
        .args([
            "--crate-name",
            "hydro_coverage_wrapper",
            "--edition=2024",
            "-o",
        ])
        .arg(&executable)
        .status()
        .expect("failed to compile coverage wrapper");
    assert!(status.success(), "failed to compile coverage wrapper");

    // Cargo keeps both of its wrappers outside this shim, in their original order.
    // Their additions are visible when the shim checks for explicit coverage flags.
    command.env("RUSTC", executable);
    command.env("HYDRO_COVERAGE_RUSTC", compiler);
    command.env_remove("HYDRO_COVERAGE_TARGET_ONLY");
    if config
        .build
        .target
        .is_some_and(|targets| !targets.is_empty())
    {
        command.env("HYDRO_COVERAGE_TARGET_ONLY", "1");
    }
    directory
}
