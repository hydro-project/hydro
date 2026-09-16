use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::{BuiltArtifact, ExampleBuildConfig, TrybuildConfig, compile_trybuild_example};

const CHILD: &str = "HYDRO_COVERAGE_TEST_CASE";

fn run(command: &mut Command) -> std::process::Output {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{command:?}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn isolated(command: &mut Command, root: &Path) {
    for (key, _) in std::env::vars_os() {
        if key.to_str().is_some_and(|key| {
            key.starts_with("CARGO_")
                || key.starts_with("RUSTC")
                || key.starts_with("RUSTFLAGS")
                || key.starts_with("LLVM_PROFILE")
                || key.starts_with("HYDRO_COVERAGE_")
                || key == "BOLERO_FUZZER"
        }) {
            command.env_remove(key);
        }
    }
    command.env("CARGO_HOME", root.join("cargo-home"));
}

fn write(path: impl AsRef<Path>, content: impl AsRef<[u8]>) {
    let path = path.as_ref();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn files_with_extension(directory: &Path, extension: &str) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(files_with_extension(&path, extension));
        } else if path.extension() == Some(OsStr::new(extension)) {
            files.push(path);
        }
    }
    files
}

fn flags_config(flags: &[&str]) -> String {
    format!(
        "rustflags = {}\n",
        toml::Value::Array(
            flags
                .iter()
                .map(|flag| toml::Value::String((*flag).to_owned()))
                .collect()
        )
    )
}

#[test]
fn coverage_preserves_cargo_flags() {
    if let Ok(case) = std::env::var(CHILD) {
        exercise(&case);
        return;
    }

    let host_output = run(Command::new("rustc").arg("-vV"));
    let host = String::from_utf8(host_output.stdout).unwrap();
    let host = host
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .unwrap();
    for case in [
        "config",
        "plain",
        "encoded",
        "empty_encoded",
        "target",
        "target_env",
        "enabled",
        "disabled",
        "last_disabled",
        "wrappers",
        "plain_no_coverage",
        "encoded_no_coverage",
        "encoded_enabled",
        "configured_compiler",
    ] {
        let root = tempfile::tempdir().unwrap();
        let workspace = root.path().join("workspace");
        let example = workspace.join("dylib-examples");
        let profiles = root.path().join("profiles");
        fs::create_dir_all(&profiles).unwrap();
        fs::create_dir_all(root.path().join("cargo-home")).unwrap();
        write(
            workspace.join("Cargo.toml"),
            "[workspace]\nmembers = [\"dylib-examples\"]\nresolver = \"2\"\n",
        );
        write(
            example.join("Cargo.toml"),
            r#"[package]
name = "dylib-examples"
version = "0.0.0"
edition = "2024"

[dependencies]
coverage-dependency = { path = "../../dependency" }

[features]
test_runtime = []
"#,
        );
        write(example.join("src/lib.rs"), "");
        write(
            root.path().join("dependency/Cargo.toml"),
            r#"[package]
name = "coverage-dependency"
version = "0.0.0"
edition = "2024"
[workspace]
"#,
        );
        let empty = case == "empty_encoded";
        let target = matches!(case, "target" | "target_env");
        let wrappers = case == "wrappers";
        let no_coverage = case.ends_with("no_coverage");
        let dependency = format!(
            r#"#![allow(unexpected_cfgs)]
#[cfg({})]
compile_error!("Cargo's selected cfg was not preserved");
#[cfg(forbidden)]
compile_error!("a lower-precedence flag source leaked into this build");
#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn coverage_dependency_probe(value: u32) -> u32 {{
    if value > 2 {{ value + 40 }} else {{ value + 1 }}
}}
"#,
            if empty { "probe" } else { "not(probe)" }
        );
        write(root.path().join("dependency/src/lib.rs"), dependency);
        let mut source = String::from(
            "#![allow(unexpected_cfgs)]\nfn main() { assert_eq!(coverage_dependency::coverage_dependency_probe(2), 3); assert_eq!(coverage_dependency::coverage_dependency_probe(3), 43); }\n",
        );
        if wrappers {
            source.push_str("#[cfg(not(all(outer_wrapper, workspace_wrapper)))] compile_error!(\"compiler wrappers were lost\");\n");
        }
        if matches!(case, "config" | "encoded") {
            source.push_str("#[cfg(not(message = \"two words\"))] compile_error!(\"flag argument was split\");\n");
        }
        write(example.join("examples/probe.rs"), source);
        write(
            example.join("build.rs"),
            format!(
                "#![allow(unexpected_cfgs)]\nfn main() {{ assert_eq!(cfg!(probe), {}); }}\n",
                !target && !empty
            ),
        );

        let mut child = Command::new(std::env::current_exe().unwrap());
        child.args([
            "--exact",
            &format!(
                "{}::coverage_preserves_cargo_flags",
                module_path!().split_once("::").unwrap().1
            ),
            "--nocapture",
        ]);
        child.current_dir(&example);
        isolated(&mut child, root.path());
        child.env(CHILD, case);
        if !no_coverage && case != "encoded_enabled" {
            child.env("LLVM_PROFILE_FILE", profiles.join("build-%p.profraw"));
        }
        let base = ["--cfg=probe", "--emit=llvm-ir"];
        let forbidden = ["--cfg=forbidden"];
        let mut config = String::from("[build]\n");
        match case {
            "config" => config.push_str(&flags_config(&[
                base[0],
                base[1],
                "--cfg=message=\"two words\"",
                "-L",
                "-Cinstrument-coverage",
            ])),
            "plain" | "plain_no_coverage" => {
                config.push_str(&flags_config(&forbidden));
                child.env("RUSTFLAGS", base.join(" "));
            }
            "encoded" | "encoded_no_coverage" => {
                config.push_str(&flags_config(&forbidden));
                if case != "encoded_no_coverage" {
                    child.env("RUSTFLAGS", "--cfg=forbidden -Cinstrument-coverage=off");
                }
                child.env(
                    "CARGO_ENCODED_RUSTFLAGS",
                    [base[0], base[1], "--cfg=message=\"two words\""].join("\x1f"),
                );
            }
            "empty_encoded" => {
                config.push_str(&flags_config(&forbidden));
                child.env("RUSTFLAGS", "--cfg=forbidden -Cinstrument-coverage=off");
                child.env("CARGO_ENCODED_RUSTFLAGS", "");
            }
            "target" | "target_env" => {
                config.push_str(&format!("target = {host:?}\n"));
                config.push_str(&flags_config(&forbidden));
                config.push_str(&format!("[target.{host}]\n"));
                config.push_str(&flags_config(&["--emit=llvm-ir"]));
                config.push_str("[target.'cfg(unix)']\n");
                config.push_str(&flags_config(&["--cfg=probe"]));
                config.push_str("[target.'cfg(windows)']\n");
                config.push_str(&flags_config(&["--cfg=probe"]));
                if case == "target_env" {
                    child.env(
                        format!(
                            "CARGO_TARGET_{}_RUSTFLAGS",
                            host.to_uppercase().replace('-', "_")
                        ),
                        "--cfg=target_env_probe",
                    );
                    write(
                        example.join("src/lib.rs"),
                        "#![allow(unexpected_cfgs)]\n#[cfg(not(target_env_probe))] compile_error!(\"target env flags lost\");\n",
                    );
                }
            }
            "encoded_enabled" => {
                config.push_str(&flags_config(&forbidden));
                child.env("RUSTFLAGS", "--cfg=forbidden -Cinstrument-coverage=off");
                child.env(
                    "CARGO_ENCODED_RUSTFLAGS",
                    [base[0], base[1], "-Cinstrument-coverage=yes"].join("\x1f"),
                );
            }
            "enabled" => {
                config.push_str(&flags_config(&[
                    base[0],
                    base[1],
                    "--codegen",
                    "instrument-coverage=yes",
                ]));
            }
            "disabled" | "last_disabled" => {
                let mut flags = base.to_vec();
                if case == "last_disabled" {
                    flags.push("-Cinstrument-coverage");
                }
                flags.extend(["-C", "instrument-coverage=off"]);
                config.push_str(&flags_config(&flags));
            }
            "wrappers" => {
                config.push_str(&flags_config(&base));
                let wrapper_source = root.path().join("wrapper.rs");
                write(
                    &wrapper_source,
                    r#"use std::process::{Command, ExitCode};
fn main() -> ExitCode {
    let mut args = std::env::args_os();
    let outer = args.next().unwrap().to_string_lossy().contains("outer");
    let mut command = Command::new(args.next().unwrap());
    command.args(args).arg(if outer { "--cfg=outer_wrapper" } else { "--cfg=workspace_wrapper" });
    if outer {
        command.arg("-Cinstrument-coverage=yes");
        command.arg("-L").arg(std::env::current_dir().unwrap().join("native libs with spaces"));
    }
    ExitCode::from(command.status().unwrap().code().unwrap_or(1) as u8)
}
"#,
                );
                let outer = root
                    .path()
                    .join(format!("outer-wrapper{}", std::env::consts::EXE_SUFFIX));
                let inner = root
                    .path()
                    .join(format!("workspace-wrapper{}", std::env::consts::EXE_SUFFIX));
                run(Command::new("rustc")
                    .arg(&wrapper_source)
                    .args(["--edition=2024", "-o"])
                    .arg(&outer));
                fs::copy(&outer, &inner).unwrap();
                config.push_str(&format!(
                    "rustc-wrapper = {}\nrustc-workspace-wrapper = {}\n",
                    toml::Value::String(outer.to_str().unwrap().to_owned()),
                    toml::Value::String(inner.to_str().unwrap().to_owned())
                ));
            }
            "configured_compiler" => {
                config.push_str(&flags_config(&base));
                let source = root.path().join("compiler.rs");
                let compiler = root.path().join(format!(
                    "configured-compiler{}",
                    std::env::consts::EXE_SUFFIX
                ));
                write(
                    &source,
                    r#"use std::process::{Command, ExitCode};
fn main() -> ExitCode {
    let mut command = Command::new("rustc");
    command.args(std::env::args_os().skip(1)).arg("--cfg=configured_compiler");
    ExitCode::from(command.status().unwrap().code().unwrap_or(1) as u8)
}
"#,
                );
                run(Command::new("rustc")
                    .arg(&source)
                    .args(["--edition=2024", "-o"])
                    .arg(&compiler));
                config.push_str(&format!(
                    "rustc = {}\n",
                    toml::Value::String(compiler.to_str().unwrap().to_owned())
                ));
                write(
                    example.join("src/lib.rs"),
                    "#![allow(unexpected_cfgs)]\n#[cfg(not(configured_compiler))] compile_error!(\"configured compiler was lost\");\n",
                );
            }
            _ => unreachable!(),
        }
        write(workspace.join(".cargo/config.toml"), config);
        let mut lock = Command::new("cargo");
        lock.current_dir(&example)
            .args(["generate-lockfile", "--offline"]);
        isolated(&mut lock, root.path());
        run(&mut lock);
        run(&mut child);
    }
}

fn exercise(case: &str) {
    let example = std::env::current_dir().unwrap();
    let workspace = example.parent().unwrap();
    let root = workspace.parent().unwrap();
    let target = root.join("target");
    let artifact = compile_trybuild_example(ExampleBuildConfig {
        trybuild: TrybuildConfig {
            project_dir: workspace.to_path_buf(),
            target_dir: target.clone(),
            features: None,
            #[cfg(any(feature = "deploy", feature = "maelstrom"))]
            linking_mode: super::LinkingMode::Dynamic,
        },
        bin_name: "coverage-probe".to_owned(),
        runtime_feature: "test_runtime",
        example_name: "probe".to_owned(),
        crate_type: None,
        set_trybuild_lib_name: false,
        allow_fuzz: false,
    })
    .unwrap();
    let coverage = !case.ends_with("no_coverage");
    assert_eq!(matches!(&artifact, BuiltArtifact::Persisted(_)), coverage);
    assert_eq!(target.join("coverage").exists(), coverage);
    assert!(
        !target.join("jobs").exists(),
        "custom flags must bypass prebuild caching"
    );
    let enabled = coverage && !matches!(case, "disabled" | "last_disabled");
    if case != "empty_encoded" {
        let ir = files_with_extension(&target, "ll")
            .into_iter()
            .filter(|path| {
                path.file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .starts_with("coverage_dependency-")
            })
            .map(|path| fs::read_to_string(path).unwrap())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            ir.contains("define") && ir.contains("@coverage_dependency_probe("),
            "dependency IR missing"
        );
        assert_eq!(
            ir.contains("@__profc_coverage_dependency_probe"),
            enabled,
            "dependency coverage counters"
        );
    }
    let build_profiles = files_with_extension(&root.join("profiles"), "profraw");
    assert_eq!(
        !build_profiles.is_empty(),
        enabled && !matches!(case, "target" | "target_env" | "encoded_enabled"),
        "host build-script instrumentation"
    );
    let profile = root.join("runtime.profraw");
    // The non-coverage artifact is a temporary file; copy it to an executable path
    // and close the write handle before executing it (required on Unix/Windows).
    let executable = root.join(format!("probe{}", std::env::consts::EXE_SUFFIX));
    fs::copy(&*artifact, &executable).unwrap();
    run(Command::new(&executable).env("LLVM_PROFILE_FILE", &profile));
    assert_eq!(profile.exists(), enabled, "runtime coverage profile");
    if enabled {
        assert!(fs::metadata(profile).unwrap().len() > 0);
    }
}
