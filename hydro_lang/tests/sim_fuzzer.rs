#![cfg(feature = "sim")]

use std::env::join_paths;

#[test]
#[cfg_attr(target_os = "windows", ignore)] // `cargo-sim` script is currently Unix only
fn fuzz_with_cargo_sim() {
    let command = std::process::Command::new("cargo")
        .args([
            "sim",
            "-p",
            "hydro_lang",
            "--features",
            "sim",
            "--",
            "sim::tests::sim_crash_with_fuzzed_batching",
        ])
        .env(
            "PATH",
            join_paths(
                std::env::split_paths(&std::env::var("PATH").unwrap())
                    .collect::<Vec<_>>()
                    .into_iter()
                    .chain([std::env::current_dir()
                        .unwrap()
                        .parent()
                        .unwrap()
                        .to_path_buf()]),
            )
            .unwrap(),
        )
        .env("NO_COLOR", "1")
        .env("HYDRO_NO_FAILURE_OUTPUT", "1")
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    let out = command.wait_with_output().unwrap();
    let stderr_text = String::from_utf8(out.stderr).unwrap();

    eprintln!("stderr:\n{}", stderr_text);

    assert!(stderr_text.contains("test failed; shrinking input..."));
    assert!(stderr_text.contains("boom\nstack backtrace:"));

    assert!(stderr_text.contains("releasing items: [100, 23]"));
}

#[test]
#[cfg_attr(target_os = "windows", ignore)] // `cargo-sim` script is currently Unix only
fn fuzz_with_cargo_sim_assume() {
    let command = std::process::Command::new("cargo")
        .args([
            "sim",
            "-p",
            "hydro_lang",
            "--features",
            "sim",
            "--",
            "sim::tests::sim_crash_behind_assume",
        ])
        .env(
            "PATH",
            join_paths(
                std::env::split_paths(&std::env::var("PATH").unwrap())
                    .collect::<Vec<_>>()
                    .into_iter()
                    .chain([std::env::current_dir()
                        .unwrap()
                        .parent()
                        .unwrap()
                        .to_path_buf()]),
            )
            .unwrap(),
        )
        .env("NO_COLOR", "1")
        .env("HYDRO_NO_FAILURE_OUTPUT", "1")
        // enable sim logging so failed assumptions are logged
        .env("HYDRO_SIM_LOG", "1")
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    let out = command.wait_with_output().unwrap();
    let stderr_text = String::from_utf8(out.stderr).unwrap();

    eprintln!("stderr:\n{}", stderr_text);

    // Instances that fail the `assume!` must be discarded (logged, not treated as failures),
    // while the fuzzer continues and still finds the real crash behind the assumption.
    assert!(stderr_text.contains("test failed; shrinking input..."));
    assert!(stderr_text.contains("boom\nstack backtrace:"));

    // Snapshot the assumption-failure log block (header + location + source line + caret) to
    // verify the `assume!` call site is properly reported.
    let lines: Vec<&str> = stderr_text.lines().collect();
    let block_start = lines
        .iter()
        .position(|line| line.starts_with("Assumption failed (discarding simulation instance):"))
        .expect("no assumption failure was logged");
    hydro_build_utils::assert_snapshot!(lines[block_start..block_start + 4].join("\n"));
}
