// Compiled as a standalone, std-only rustc wrapper by coverage.rs.
use std::process::{Command, ExitCode};

/// The last explicit coverage setting, following rustc's codegen-option semantics.
/// Unknown values are left for rustc to diagnose, not replaced with a valid flag.
pub fn coverage_setting<'a>(args: impl IntoIterator<Item = &'a str>) -> Option<bool> {
    let mut args = args.into_iter();
    let mut setting = None;
    while let Some(arg) = args.next() {
        if arg == "--" {
            break;
        }
        // A value can itself look like a codegen flag: `-L -Cinstrument-coverage`
        // names a search directory, not a request for instrumentation. Only the
        // separate-value forms need skipping; attached and `--option=value` forms
        // cannot be mistaken for a codegen option on the next iteration.
        // Option arities: rustc_session::config::rustc_optgroups.
        if matches!(
            arg,
            "-L" | "-l"
                | "-o"
                | "-A"
                | "-W"
                | "-D"
                | "-F"
                | "-Z"
                | "-j"
                | "--cfg"
                | "--check-cfg"
                | "--crate-type"
                | "--crate-name"
                | "--edition"
                | "--emit"
                | "--print"
                | "--out-dir"
                | "--explain"
                | "--target"
                | "--allow"
                | "--warn"
                | "--force-warn"
                | "--deny"
                | "--forbid"
                | "--cap-lints"
                | "--extern"
                | "--sysroot"
                | "--error-format"
                | "--json"
                | "--color"
                | "--diagnostic-width"
                | "--remap-path-prefix"
                | "--remap-path-scope"
                | "--jobs"
                | "--jobs-frontend"
                | "--jobs-backend"
                | "--jobs-linker"
        ) {
            args.next();
            continue;
        }
        let codegen = if arg == "-C" || arg == "--codegen" {
            args.next()
        } else {
            arg.strip_prefix("-C")
                .or_else(|| arg.strip_prefix("--codegen="))
        };
        if let Some(codegen) = codegen {
            let (name, value) = codegen.split_once('=').unwrap_or((codegen, "yes"));
            if name == "instrument-coverage" || name == "instrument_coverage" {
                setting = Some(!matches!(value, "n" | "no" | "off" | "false"));
            }
        }
    }
    setting
}

fn main() -> ExitCode {
    let compiler = std::env::var_os("HYDRO_COVERAGE_RUSTC").expect("missing rustc path");
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    // Cargo supplies --target only for target units when build.target is set.
    // Without build.target, Cargo shares host and target dependencies and applies
    // rustflags (and therefore our additive instrumentation) to both.
    let target_unit = std::env::var_os("HYDRO_COVERAGE_TARGET_ONLY").is_none()
        || args.iter().any(|arg| {
            arg == "--target" || arg.to_str().is_some_and(|arg| arg.starts_with("--target="))
        });
    let explicit = coverage_setting(args.iter().map(|arg| arg.to_str().unwrap_or("")));
    let mut command = Command::new(compiler);
    command.args(&args);
    // Leave explicit on/off options untouched, including their order. In particular,
    // LLVM_PROFILE_FILE is not permission to override -Cinstrument-coverage=off.
    if target_unit && explicit.is_none() {
        command.arg("-Cinstrument-coverage");
    }
    let status = command.status().expect("failed to execute rustc");
    ExitCode::from(
        status
            .code()
            .and_then(|code| u8::try_from(code).ok())
            .unwrap_or(1),
    )
}
