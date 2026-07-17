//! Capture-and-replay of the final `rustc` invocation for generated example builds.
//!
//! The final per-test build compiles a single generated example crate against a fully
//! prebuilt dependency graph, so the `cargo` invocation wrapping it does no useful work
//! beyond computing the rustc command line — but costs a fixed ~300ms per invocation
//! (config/lockfile/fingerprint bookkeeping) plus per-job target dir population and
//! cross-process cargo locking. Since the command line is identical for every test modulo
//! a few substitutions (output dir, symbol metadata, crate name/source path, and env
//! vars), the first cargo build after each prebuild is run with `-v` and the rustc
//! invocation cargo prints is captured as a *template* with placeholders. All subsequent
//! builds replay the template with plain rustc.
//!
//! Templates are keyed to the prebuild stamp (`features_hash:timestamp`), which changes
//! whenever the prebuild reruns, so staleness tracking is exactly as strong as the
//! prebuild's own. Any capture or replay anomaly makes callers fall back to the cargo
//! path (deleting the template), so this is strictly a fast path.

use std::path::{Path, PathBuf};
use std::{fs, io};

/// Placeholder for the example's crate name (`-` replaced by `_`).
pub const TPL_CRATE_NAME: &str = "{{__hydro_crate_name__}}";
/// Placeholder for the example's source path, relative to the template's cwd.
pub const TPL_SOURCE_PATH: &str = "{{__hydro_source_path__}}";
/// Placeholder for the per-test output directory.
pub const TPL_OUT_DIR: &str = "{{__hydro_out_dir__}}";
/// Placeholder for the per-test `-C metadata` / `-C extra-filename` value.
pub const TPL_METADATA: &str = "{{__hydro_metadata__}}";

/// A captured rustc invocation for the final example build, with per-test
/// values replaced by placeholders.
pub struct RustcTemplate {
    /// The prebuild stamp this template was captured under.
    pub stamp: String,
    /// The working directory of the rustc invocation (the trybuild project dir).
    pub cwd: PathBuf,
    /// argv (program + args), with placeholders for per-test values.
    pub argv: Vec<String>,
}

/// The template file path associated with a prebuild lock file.
pub fn template_path(prebuild_lock_path: &Path) -> PathBuf {
    let mut path = prebuild_lock_path.as_os_str().to_owned();
    path.push(".rustc-cmd");
    PathBuf::from(path)
}

/// Load the template if it exists and matches the current prebuild stamp.
pub fn load(path: &Path, expected_stamp: &str) -> Option<RustcTemplate> {
    let contents = fs::read_to_string(path).ok()?;
    let mut lines = contents.lines();
    let stamp = lines.next()?;
    if stamp != expected_stamp {
        return None;
    }
    let cwd = PathBuf::from(lines.next()?);
    let argv: Vec<String> = lines.map(str::to_owned).collect();
    if argv.len() < 2 {
        return None;
    }
    Some(RustcTemplate {
        stamp: stamp.to_owned(),
        cwd,
        argv,
    })
}

/// Atomically persist the template (write to a temp file, then rename).
pub fn store(path: &Path, template: &RustcTemplate) -> io::Result<()> {
    let mut contents = String::new();
    contents.push_str(&template.stamp);
    contents.push('\n');
    contents.push_str(template.cwd.to_str().ok_or(io::ErrorKind::InvalidData)?);
    contents.push('\n');
    for arg in &template.argv {
        contents.push_str(arg);
        contents.push('\n');
    }
    let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
    fs::write(&tmp, contents)?;
    fs::rename(&tmp, path)
}

/// Extract the rustc invocation for `crate_name` from the stderr of a `cargo ... -v` build.
///
/// Cargo's verbose output contains lines like:
///
/// ```text
///      Running `/path/to/rustc --crate-name foo --edition=2024 src/foo.rs ...`
/// ```
///
/// (possibly prefixed by a `RUSTC_WRAPPER` such as sccache). Returns the tokenized argv of
/// the *last* such line whose `--crate-name` value equals `crate_name`, or `None` if no
/// matching line is found or the line cannot be tokenized unambiguously.
pub fn extract_rustc_command(stderr: &str, crate_name: &str) -> Option<Vec<String>> {
    let mut result = None;
    for line in stderr.lines() {
        let Some(start) = line.find("Running `") else {
            continue;
        };
        let rest = &line[start + "Running `".len()..];
        let Some(end) = rest.rfind('`') else {
            continue;
        };
        let Some(argv) = shell_tokenize(&rest[..end]) else {
            continue;
        };
        let names_crate = argv
            .iter()
            .zip(argv.iter().skip(1))
            .any(|(flag, value)| flag == "--crate-name" && value == crate_name);
        if names_crate {
            result = Some(argv);
        }
    }
    result
}

/// Tokenize a shell-escaped command line (as printed by cargo's verbose output) into argv.
///
/// Handles POSIX single quotes (used by cargo on unix), double quotes (used on windows),
/// and backslash escapes outside single quotes. Returns `None` on unbalanced quotes so
/// callers can fall back rather than misparse.
fn shell_tokenize(line: &str) -> Option<Vec<String>> {
    let mut argv = Vec::new();
    let mut cur = String::new();
    let mut in_token = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            ' ' | '\t' => {
                if in_token {
                    argv.push(std::mem::take(&mut cur));
                    in_token = false;
                }
            }
            '\'' => {
                in_token = true;
                loop {
                    match chars.next() {
                        Some('\'') => break,
                        Some(c) => cur.push(c),
                        None => return None,
                    }
                }
            }
            '"' => {
                in_token = true;
                loop {
                    match chars.next() {
                        Some('"') => break,
                        Some('\\') => {
                            // Only `\"` is an escape inside double quotes (Windows-style
                            // quoting); a lone backslash (e.g. in paths) is literal.
                            if chars.peek() == Some(&'"') {
                                cur.push(chars.next()?);
                            } else {
                                cur.push('\\');
                            }
                        }
                        Some(c) => cur.push(c),
                        None => return None,
                    }
                }
            }
            '\\' => {
                in_token = true;
                cur.push(chars.next()?);
            }
            c => {
                in_token = true;
                cur.push(c);
            }
        }
    }
    if in_token {
        argv.push(cur);
    }
    Some(argv)
}

/// Turn a captured rustc argv into a reusable template by replacing the
/// per-test values with placeholders and redirecting the per-job target dir
/// paths (whose `deps`/`.fingerprint`/`build` entries are symlinks into the
/// shared target dir) to the shared target dir itself.
///
/// Returns `None` (no template; callers keep using cargo) unless every
/// expected substitution is found exactly once and nothing suspicious
/// (incremental compilation, leftover job-dir paths, embedded newlines)
/// remains.
pub fn build_template(
    argv: Vec<String>,
    crate_name: &str,
    example_name: &str,
    job_target_dir: &Path,
    shared_target_dir: &Path,
) -> Option<Vec<String>> {
    let job_str = job_target_dir.to_str()?;
    let shared_str = shared_target_dir.to_str()?;
    let source_suffixes = [
        format!("examples/{example_name}.rs"),
        format!("examples\\{example_name}.rs"),
    ];

    let (mut saw_crate_name, mut saw_source, mut saw_out_dir) = (0u32, 0u32, 0u32);
    let (mut saw_metadata, mut saw_extra_filename) = (0u32, 0u32);

    let mut out = Vec::with_capacity(argv.len());
    let mut prev: Option<&str> = None;
    for arg in &argv {
        if arg.contains('\n') || arg.contains("incremental") {
            return None;
        }
        let templated = if prev == Some("--crate-name") {
            if arg != crate_name {
                return None;
            }
            saw_crate_name += 1;
            TPL_CRATE_NAME.to_owned()
        } else if prev == Some("--out-dir") {
            saw_out_dir += 1;
            TPL_OUT_DIR.to_owned()
        } else if source_suffixes.iter().any(|s| arg.ends_with(&**s)) {
            saw_source += 1;
            TPL_SOURCE_PATH.to_owned()
        } else if prev == Some("-C") && arg.starts_with("metadata=") {
            saw_metadata += 1;
            format!("metadata={TPL_METADATA}")
        } else if prev == Some("-C") && arg.starts_with("extra-filename=") {
            saw_extra_filename += 1;
            format!("extra-filename=-{TPL_METADATA}")
        } else {
            arg.replace(job_str, shared_str)
        };
        if templated.contains(job_str) {
            return None;
        }
        out.push(templated);
        prev = Some(arg);
    }

    ([
        saw_crate_name,
        saw_source,
        saw_out_dir,
        saw_metadata,
        saw_extra_filename,
    ] == [1; 5])
        .then_some(out)
}

/// Per-test substitutions applied to a template at replay time.
pub struct ReplayParams<'a> {
    /// The example's crate name (`-` replaced by `_`).
    pub crate_name: &'a str,
    /// The example's source path, relative to the template's cwd.
    pub source_path: &'a str,
    /// Directory to place the built artifact in (per-test).
    pub out_dir: &'a Path,
    /// Unique `-C metadata` / `-C extra-filename` value (disambiguates symbols
    /// when multiple compiled examples are loaded into one process).
    pub metadata: &'a str,
    /// Extra environment variables (e.g. `TRYBUILD_LIB_NAME`).
    pub envs: Vec<(String, std::ffi::OsString)>,
}

/// Replay a captured template with per-test substitutions. Returns the path to
/// the linked artifact on success; on any failure returns a description (the
/// caller falls back to the cargo path, which reports errors with full fidelity).
pub fn replay(template: &RustcTemplate, params: ReplayParams<'_>) -> Result<PathBuf, String> {
    use std::process::{Command, Stdio};

    if !template.cwd.join(params.source_path).exists() {
        return Err(format!(
            "example source {} not found under {}",
            params.source_path,
            template.cwd.display()
        ));
    }
    fs::create_dir_all(params.out_dir).map_err(|e| e.to_string())?;
    let out_dir_str = params.out_dir.to_str().ok_or("non-utf8 out dir")?;

    let substituted: Vec<String> = template
        .argv
        .iter()
        .map(|arg| {
            arg.replace(TPL_CRATE_NAME, params.crate_name)
                .replace(TPL_SOURCE_PATH, params.source_path)
                .replace(TPL_OUT_DIR, out_dir_str)
                .replace(TPL_METADATA, params.metadata)
        })
        .collect();

    let mut command = Command::new(&substituted[0]);
    command.args(&substituted[1..]);
    command.current_dir(&template.cwd);
    for (key, value) in &params.envs {
        command.env(key, value);
    }

    tracing::debug!(
        target: "hydro_build",
        "final rustc command (cwd={}): {:?}",
        template.cwd.display(),
        command
    );

    let output = command
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("failed to spawn rustc: {e}"))?;

    // rustc emits JSON messages (diagnostics + artifact notifications) on stderr.
    let stderr = String::from_utf8_lossy(&output.stderr);
    let mut artifact = None;
    let mut rendered_diagnostics = Vec::new();
    for line in stderr.lines() {
        let Ok(msg) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        match msg.get("$message_type").and_then(|t| t.as_str()) {
            Some("artifact") => {
                if msg.get("emit").and_then(|e| e.as_str()) == Some("link") {
                    artifact = msg
                        .get("artifact")
                        .and_then(|a| a.as_str())
                        .map(PathBuf::from);
                }
            }
            Some("diagnostic") => {
                if let Some(rendered) = msg.get("rendered").and_then(|r| r.as_str()) {
                    rendered_diagnostics.push(rendered.to_owned());
                }
            }
            _ => {}
        }
    }

    if !output.status.success() {
        return Err(format!(
            "rustc exited with {}:\n{}",
            output.status,
            rendered_diagnostics.join("\n")
        ));
    }
    // Surface warnings just like the cargo path does.
    for rendered in &rendered_diagnostics {
        eprint!("{rendered}");
    }
    artifact.ok_or_else(|| "rustc succeeded but emitted no linked artifact".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_tokenize() {
        assert_eq!(
            shell_tokenize(r#"rustc --cfg 'feature="build"' --emit=link"#).unwrap(),
            vec!["rustc", "--cfg", r#"feature="build""#, "--emit=link"]
        );
        assert_eq!(
            shell_tokenize(r#""C:\path with space\rustc" --crate-name foo"#).unwrap(),
            vec![r"C:\path with space\rustc", "--crate-name", "foo"]
        );
        assert_eq!(shell_tokenize("unbalanced 'quote"), None);
    }

    #[test]
    fn test_extract_rustc_command() {
        let stderr = "   Compiling foo v0.1.0\n     Running `/usr/bin/rustc --crate-name foo --edition=2024 src/lib.rs`\n     Running `/usr/bin/rustc --crate-name bar --edition=2024 examples/bar.rs --cfg 'feature=\"x\"'`\n    Finished dev\n";
        assert_eq!(
            extract_rustc_command(stderr, "bar").unwrap(),
            vec![
                "/usr/bin/rustc",
                "--crate-name",
                "bar",
                "--edition=2024",
                "examples/bar.rs",
                "--cfg",
                r#"feature="x""#
            ]
        );
        assert_eq!(extract_rustc_command(stderr, "baz"), None);
    }

    #[test]
    fn test_build_template_and_roundtrip() {
        let job = Path::new("/t/jobs/ABCD1234");
        let shared = Path::new("/t");
        let argv: Vec<String> = [
            "/wrap/sccache",
            "/bin/rustc",
            "--crate-name",
            "sim_dylib",
            "--edition=2024",
            "dylib-examples/examples/sim-dylib.rs",
            "--crate-type",
            "cdylib",
            "--emit=dep-info,link",
            "-C",
            "metadata=69bac1f35c17e10b",
            "-C",
            "extra-filename=-036b96e057adb47a",
            "--out-dir",
            "/t/jobs/ABCD1234/debug/examples",
            "-L",
            "dependency=/t/jobs/ABCD1234/debug/deps",
            "--extern",
            "dep=/t/jobs/ABCD1234/debug/deps/libdep.so",
            "-Clink-arg=-Wl,-rpath,/t/jobs/ABCD1234/debug",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();

        let template = build_template(argv.clone(), "sim_dylib", "sim-dylib", job, shared).unwrap();
        assert_eq!(template[2..4], ["--crate-name", TPL_CRATE_NAME]);
        assert_eq!(template[5], TPL_SOURCE_PATH);
        assert_eq!(template[10], format!("metadata={TPL_METADATA}"));
        assert_eq!(template[12], format!("extra-filename=-{TPL_METADATA}"));
        assert_eq!(template[14], TPL_OUT_DIR);
        assert_eq!(template[16], "dependency=/t/debug/deps");
        assert_eq!(template[18], "dep=/t/debug/deps/libdep.so");
        assert_eq!(template[19], "-Clink-arg=-Wl,-rpath,/t/debug");

        // A mismatched crate name must not produce a template.
        assert!(build_template(argv.clone(), "other", "sim-dylib", job, shared).is_none());
        // Incremental compilation args must not produce a template.
        let mut with_incr = argv.clone();
        with_incr.push("-Cincremental=/t/incr".to_owned());
        assert!(build_template(with_incr, "sim_dylib", "sim-dylib", job, shared).is_none());
    }
}
