#![cfg(feature = "sim")]

/// Compiles and runs `sim::tests::sim_generated_code_sees_root_rustflags` with a probe cfg
/// injected via `--config build.rustflags`. Unlike the `RUSTFLAGS` environment variable, config
/// rustflags never appear in the test process's environment, so the generated sim program only
/// sees them if `hydro_lang` forwards the `CARGO_ENCODED_RUSTFLAGS` it was itself compiled with.
#[test]
fn config_rustflags_reach_generated_program() {
    // Fold any inherited `RUSTFLAGS` into the config value: the environment variable takes
    // precedence over `build.rustflags`, so leaving it set would silently drop the probe cfg.
    let mut rustflags: Vec<String> = std::env::var("RUSTFLAGS")
        .map(|flags| flags.split_whitespace().map(str::to_owned).collect())
        .unwrap_or_default();
    rustflags.extend(["--cfg".to_owned(), "hydro_rustflags_probe".to_owned()]);
    let config = format!(
        "build.rustflags=[{}]",
        rustflags
            .iter()
            .map(|flag| format!("{flag:?}"))
            .collect::<Vec<_>>()
            .join(",")
    );

    let out = std::process::Command::new("cargo")
        .args([
            "test",
            "-p",
            "hydro_lang",
            "--lib",
            "--features",
            "sim",
            "--config",
            &config,
            "--",
            "--exact",
            "--ignored",
            "sim::tests::sim_generated_code_sees_root_rustflags",
        ])
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env("NO_COLOR", "1")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .unwrap();

    let stdout_text = String::from_utf8(out.stdout).unwrap();
    let stderr_text = String::from_utf8(out.stderr).unwrap();
    eprintln!("stdout:\n{stdout_text}\nstderr:\n{stderr_text}");

    assert!(out.status.success(), "inner `cargo test` failed");
    assert!(stdout_text.contains("test result: ok. 1 passed"));
}
