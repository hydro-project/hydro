//! Harness that runs `cargo verus verify` over the fixtures in this crate: the
//! `accepted` module must verify, and each `reject_*` feature must fail verification
//! with the diagnostics captured in an insta snapshot.
//!
//! Requires the Verus toolchain (`cargo-verus`, `verus` and its pinned rustc) to be
//! installed and on `PATH`; set `HYDRO_VERUS_TESTS=1` to enable (CI does this). The
//! rejection cases are compiled one at a time because a failed proof aborts
//! verification of the whole crate.

use std::path::PathBuf;
use std::process::Command;

/// The Verus driver requires the `verus_builtin` prelude import in every crate it
/// verifies (this harness itself contains no proofs, but the crate opts in via
/// `[package.metadata.verus]`). Erased under normal compilation.
#[cfg(verus_keep_ghost)]
#[allow(unused_imports, reason = "required by the Verus driver")]
use vstd::prelude::*;

const REJECT_FEATURES: &[&str] = &[
    "reject_overwrite_fold",
    "reject_halving_fold",
    "reject_overflowing_fold",
    "reject_overwrite_map_capture",
    "reject_running_total_map",
    "reject_rate_limit_filter",
];

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn cargo_verus_verify(features: Option<&str>) -> std::process::Output {
    // Touch the fixture source so cargo re-verifies it even when cached (a fully
    // cached run would produce no verification output at all).
    let lib_rs = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    let file = std::fs::OpenOptions::new()
        .append(true)
        .open(&lib_rs)
        .unwrap();
    file.set_modified(std::time::SystemTime::now()).unwrap();

    let mut command = Command::new("cargo-verus");
    command
        .arg("verify")
        .args(["-p", "hydro_verus_tests"])
        .current_dir(workspace_root())
        // Colored diagnostics (e.g. from `CARGO_TERM_COLOR: always` in CI) would defeat
        // the noise filtering and destabilize the snapshots.
        .env("CARGO_TERM_COLOR", "never")
        .env("NO_COLOR", "1");
    if let Some(features) = features {
        command.args(["--features", features]);
    }
    command
        .output()
        .expect("failed to spawn cargo-verus; is the Verus toolchain installed?")
}

/// Strips ANSI escape sequences (e.g. from colored cargo/rustc output), so filtering
/// and snapshots are stable regardless of the environment's color settings.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                // consume until the final byte of the CSI sequence (an ASCII letter)
                for e in chars.by_ref() {
                    if e.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Extracts the Verus diagnostics from the stderr of a `cargo verus verify` run,
/// dropping cargo's progress chatter so the output is stable for snapshotting.
fn verus_diagnostics(stderr: &str) -> String {
    let stderr = strip_ansi(stderr);
    let noise_prefixes = [
        "Checking",
        "Compiling",
        "Finished",
        "Downloading",
        "Downloaded",
        "Blocking",
        "Adding",
        "Updating",
        "Locking",
        "Removing",
        "warning: failed to write cache",
    ];

    stderr
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.is_empty() && !noise_prefixes.iter().any(|p| trimmed.starts_with(p))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The final `verification results::` line (from stdout) for the target crate;
/// earlier lines may belong to freshly-built dependencies like `vstd`.
fn verification_results(stdout: &str) -> String {
    strip_ansi(stdout)
        .lines()
        .rfind(|line| line.starts_with("verification results::"))
        .unwrap_or("")
        .to_owned()
}

fn enabled() -> bool {
    if std::env::var("HYDRO_VERUS_TESTS").is_ok_and(|v| v == "1") {
        true
    } else {
        eprintln!("skipping Verus verification tests; set HYDRO_VERUS_TESTS=1 to enable");
        false
    }
}

#[test]
fn accepted_proofs_verify() {
    if !enabled() {
        return;
    }

    let out = cargo_verus_verify(None);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "expected verification to succeed:\n{stderr}"
    );

    hydro_build_utils::assert_snapshot!(verification_results(&stdout));
}

#[test]
fn rejected_proofs_fail_verification() {
    if !enabled() {
        return;
    }

    for reject in REJECT_FEATURES {
        let out = cargo_verus_verify(Some(reject));
        let stderr = String::from_utf8_lossy(&out.stderr);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            !out.status.success(),
            "expected verification to FAIL for feature `{reject}`, but it succeeded"
        );

        hydro_build_utils::insta::with_settings!({
            snapshot_suffix => reject.to_string()
        }, {
            hydro_build_utils::assert_snapshot!(format!(
                "{}\n{}",
                verification_results(&stdout),
                verus_diagnostics(&stderr)
            ));
        });
    }
}
