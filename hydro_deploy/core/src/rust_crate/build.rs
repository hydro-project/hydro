use std::error::Error;
use std::fmt::Display;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::OnceLock;

use cargo_metadata::diagnostic::Diagnostic;
use memo_map::MemoMap;
use tokio::sync::OnceCell;

use crate::HostTargetType;
use crate::progress::ProgressTracker;

/// Build parameters for [`build_crate_memoized`].
#[derive(PartialEq, Eq, Hash, Clone)]
pub struct BuildParams {
    /// The working directory for the build, where the `cargo build` command will be run. Crate root.
    /// [`Self::new`] canonicalizes this path.
    src: PathBuf,
    /// The workspace root encompassing the build, which may be a parent of `src` in a multi-crate
    /// workspace.
    workspace_root: PathBuf,
    /// `--bin` binary name parameter.
    bin: Option<String>,
    /// `--example` parameter.
    example: Option<String>,
    /// `--profile` parameter.
    profile: Option<String>,
    rustflags: Option<String>,
    target_dir: Option<PathBuf>,
    // Environment variables available during build
    build_env: Vec<(String, String)>,
    no_default_features: bool,
    /// `--target <linux>` if cross-compiling for linux ([`HostTargetType::Linux`]).
    target_type: HostTargetType,
    /// True is the build should use dynamic linking.
    is_dylib: bool,
    /// `--features` flags, will be comma-delimited.
    features: Option<Vec<String>>,
    /// `--config` flag
    config: Vec<String>,
}
impl BuildParams {
    /// Creates a new `BuildParams` and canonicalizes the `src` path.
    #[expect(clippy::too_many_arguments, reason = "internal code")]
    pub fn new(
        src: impl AsRef<Path>,
        workspace_root: impl AsRef<Path>,
        bin: Option<String>,
        example: Option<String>,
        profile: Option<String>,
        rustflags: Option<String>,
        target_dir: Option<PathBuf>,
        build_env: Vec<(String, String)>,
        no_default_features: bool,
        target_type: HostTargetType,
        is_dylib: bool,
        features: Option<Vec<String>>,
        config: Vec<String>,
    ) -> Self {
        // `fs::canonicalize` prepends windows paths with the `r"\\?\"`
        // https://stackoverflow.com/questions/21194530/what-does-mean-when-prepended-to-a-file-path
        // However, this breaks the `include!(concat!(env!("OUT_DIR"), "/my/forward/slash/path.rs"))`
        // Rust codegen pattern on windows. To help mitigate this happening in third party crates, we
        // instead use `dunce::canonicalize` which is the same as `fs::canonicalize` but avoids the
        // `\\?\` prefix when possible.
        let src = dunce::canonicalize(src.as_ref()).unwrap_or_else(|e| {
            panic!(
                "Failed to canonicalize path `{}` for build: {e}.",
                src.as_ref().display(),
            )
        });

        let workspace_root = dunce::canonicalize(workspace_root.as_ref()).unwrap_or_else(|e| {
            panic!(
                "Failed to canonicalize path `{}` for build: {e}.",
                workspace_root.as_ref().display(),
            )
        });

        BuildParams {
            src,
            workspace_root,
            bin,
            example,
            profile,
            rustflags,
            target_dir,
            build_env,
            no_default_features,
            target_type,
            is_dylib,
            features,
            config,
        }
    }
}

/// Information about a built crate. See [`build_crate_memoized`].
pub struct BuildOutput {
    /// The binary contents as a byte array.
    pub bin_data: Vec<u8>,
    /// The path to the binary file. [`Self::bin_data`] has a copy of the content.
    pub bin_path: PathBuf,
    /// Shared library path, containing any necessary dylibs.
    pub shared_library_path: Option<PathBuf>,
}
impl BuildOutput {
    /// A unique ID for the binary, based its contents.
    pub fn unique_id(&self) -> impl use<> + Display {
        blake3::hash(&self.bin_data).to_hex()
    }
}

/// Build memoization cache.
static BUILDS: OnceLock<MemoMap<BuildParams, OnceCell<BuildOutput>>> = OnceLock::new();

pub async fn build_crate_memoized(params: BuildParams) -> Result<&'static BuildOutput, BuildError> {
    BUILDS
        .get_or_init(MemoMap::new)
        .get_or_insert(&params, Default::default)
        .get_or_try_init(move || {
            ProgressTracker::rich_leaf("build", move |set_msg| async move {
                tokio::task::spawn_blocking(move || {
                    let base_target_dir = params
                        .target_dir
                        .as_ref()
                        .cloned()
                        .unwrap_or_else(|| params.src.join("target"));
                    let job_name = params
                        .bin
                        .as_deref()
                        .or(params.example.as_deref())
                        .unwrap_or("default");

                    // Only use prebuild + per-job target dirs for dylib mode.
                    // Without dylib, build directly into the base target dir.
                    let (per_job_target_dir, _prebuild_guard, _cargo_lock) = if params.is_dylib {
                        let shared_debug = base_target_dir.join("debug");
                        let jobs_dir = base_target_dir.join("jobs");
                        let per_job = hydro_concurrent_cargo::setup_job_dir(&jobs_dir, job_name, &shared_debug);

                        let features = params.features.clone().unwrap_or_default();
                        let staged_paths = vec![
                            params.src.join("src").join("__staged.rs"),
                            params.src.join("Cargo.lock"),
                            std::env::current_exe().unwrap(),
                        ];

                        let src = params.src.clone();
                        let profile = params.profile.clone();
                        let target_type = params.target_type;
                        let no_default_features = params.no_default_features;
                        let features_for_closure = features.clone();
                        let config = params.config.clone();
                        let rustflags = params.rustflags.clone();
                        let build_env = params.build_env.clone();

                        let (prebuild_guard, cargo_lock) = hydro_concurrent_cargo::run_prebuild(
                            &base_target_dir,
                            params.src.parent().unwrap().file_name().unwrap().to_str().unwrap(),
                            &features,
                            &staged_paths,
                            |prebuild_target| {
                                set_msg("building dependencies".to_owned());

                                let mut dep_cmd = Command::new("cargo");
                                dep_cmd.current_dir(src.join("..").join("dylib"));
                                dep_cmd.args(["build", "--locked"]);

                                if let Some(profile) = profile.as_ref() {
                                    dep_cmd.args(["--profile", profile]);
                                }

                                match target_type {
                                    HostTargetType::Local => {}
                                    HostTargetType::Linux(crate::LinuxCompileType::Glibc) => {
                                        dep_cmd.args(["--target", "x86_64-unknown-linux-gnu"]);
                                    }
                                    HostTargetType::Linux(crate::LinuxCompileType::Musl) => {
                                        dep_cmd.args(["--target", "x86_64-unknown-linux-musl"]);
                                    }
                                }

                                if no_default_features {
                                    dep_cmd.arg("--no-default-features");
                                }

                                if !features_for_closure.is_empty() {
                                    dep_cmd.args(["--features", &features_for_closure.join(",")]);
                                }

                                for c in &config {
                                    dep_cmd.args(["--config", c]);
                                }

                                dep_cmd.args(["--target-dir", prebuild_target.to_str().unwrap()]);

                                if let Some(rustflags) = rustflags.as_ref() {
                                    dep_cmd.env("RUSTFLAGS", rustflags);
                                }

                                for (k, v) in &build_env {
                                    dep_cmd.env(k, v);
                                }

                                eprintln!("[hydro-build] starting deploy prebuild child cargo");
                                let status = dep_cmd
                                    .stdin(Stdio::null())
                                    .status()
                                    .unwrap();
                                eprintln!("[hydro-build] deploy prebuild child cargo finished, success={}", status.success());
                                if !status.success() {
                                    panic!("dep prebuild failed");
                                }

                                // Also prebuild the dylib-examples lib.
                                eprintln!("[hydro-build] starting deploy prebuild dylib-examples lib");
                                let mut lib_cmd = Command::new("cargo");
                                lib_cmd.current_dir(&src);
                                lib_cmd.args(["build", "--locked", "--lib"]);
                                if let Some(profile) = profile.as_ref() {
                                    lib_cmd.args(["--profile", profile]);
                                }
                                match target_type {
                                    HostTargetType::Local => {}
                                    HostTargetType::Linux(crate::LinuxCompileType::Glibc) => {
                                        lib_cmd.args(["--target", "x86_64-unknown-linux-gnu"]);
                                    }
                                    HostTargetType::Linux(crate::LinuxCompileType::Musl) => {
                                        lib_cmd.args(["--target", "x86_64-unknown-linux-musl"]);
                                    }
                                }
                                if no_default_features {
                                    lib_cmd.arg("--no-default-features");
                                }
                                if !features_for_closure.is_empty() {
                                    lib_cmd.args(["--features", &features_for_closure.join(",")]);
                                }
                                for c in &config {
                                    lib_cmd.args(["--config", c]);
                                }
                                lib_cmd.args(["--target-dir", prebuild_target.to_str().unwrap()]);
                                if let Some(rustflags) = rustflags.as_ref() {
                                    lib_cmd.env("RUSTFLAGS", rustflags);
                                }
                                for (k, v) in &build_env {
                                    lib_cmd.env(k, v);
                                }
                                let lib_status = lib_cmd
                                    .stdin(Stdio::null())
                                    .status()
                                    .unwrap();
                                if !lib_status.success() {
                                    panic!("dylib-examples lib prebuild failed");
                                }
                            },
                        );

                        (per_job, Some(prebuild_guard), Some(cargo_lock))
                    } else {
                        (base_target_dir.clone(), None, None)
                    };

                    hydro_concurrent_cargo::log_build_event(&base_target_dir, "deploy: starting final build");

                    // Populate per-job build/ dir. Hold guard for entire final build.
                    let _job_build_guard = if params.is_dylib {
                        let shared_debug = base_target_dir.join("debug");
                        Some(hydro_concurrent_cargo::populate_job_build_dir(&per_job_target_dir.join("debug"), &shared_debug))
                    } else {
                        None
                    };

                    let mut command = Command::new("cargo");
                    command.args(["build", if params.is_dylib { "--frozen" } else { "--locked" }]);

                    if let Some(profile) = params.profile.as_ref() {
                        command.args(["--profile", profile]);
                    }

                    if let Some(bin) = params.bin.as_ref() {
                        command.args(["--bin", bin]);
                    }

                    if let Some(example) = params.example.as_ref() {
                        command.args(["--example", example]);
                    }

                    match params.target_type {
                        HostTargetType::Local => {}
                        HostTargetType::Linux(crate::LinuxCompileType::Glibc) => {
                            command.args(["--target", "x86_64-unknown-linux-gnu"]);
                        }
                        HostTargetType::Linux(crate::LinuxCompileType::Musl) => {
                            command.args(["--target", "x86_64-unknown-linux-musl"]);
                        }
                    }

                    if params.no_default_features {
                        command.arg("--no-default-features");
                    }

                    if let Some(features) = params.features {
                        command.args(["--features", &features.join(",")]);
                    }

                    for config in &params.config {
                        command.args(["--config", config]);
                    }

                    command.arg("--message-format=json-diagnostic-rendered-ansi");
                    command.args(["--target-dir", per_job_target_dir.to_str().unwrap()]);

                    if let Some(rustflags) = params.rustflags.as_ref() {
                        command.env("RUSTFLAGS", rustflags);
                    }

                    for (k, v) in params.build_env {
                        command.env(k, v);
                    }

                    let mut spawned = command
                        .current_dir(&params.src)
                        .stdout(Stdio::piped())
                        .stderr(Stdio::piped())
                        .stdin(Stdio::null())
                        .spawn()
                        .unwrap();

                    let reader = std::io::BufReader::new(spawned.stdout.take().unwrap());
                    let stderr_reader = std::io::BufReader::new(spawned.stderr.take().unwrap());

                    let stderr_worker = std::thread::spawn(move || {
                        let mut stderr_lines = Vec::new();
                        for line in stderr_reader.lines() {
                            let Ok(line) = line else {
                                break;
                            };
                            set_msg(line.clone());
                            stderr_lines.push(line);
                        }
                        stderr_lines
                    });

                    let mut diagnostics = Vec::new();
                    let mut text_lines = Vec::new();
                    for message in cargo_metadata::Message::parse_stream(reader) {
                        match message.unwrap() {
                            cargo_metadata::Message::CompilerArtifact(artifact) => {
                                let is_output = if params.example.is_some() {
                                    artifact.target.kind.iter().any(|k| "example" == k)
                                } else {
                                    artifact.target.kind.iter().any(|k| "bin" == k)
                                };

                                if is_output {
                                    let path = artifact.executable.unwrap();
                                    let path_buf: PathBuf = path.clone().into();
                                    let path = path.into_string();
                                    let data = std::fs::read(path).unwrap();
                                    let exit_status = spawned.wait().unwrap();

                                    let stderr_lines = stderr_worker.join().unwrap();

                                    // Check for unexpected recompilations (only in dylib mode with prebuild).
                                    if params.is_dylib {
                                        for line in &stderr_lines {
                                            if line.contains("Compiling") && !line.contains("dylib-examples") && !line.contains(job_name) {
                                                panic!(
                                                    "unexpected recompilation in deploy final build: {line}\nfull stderr:\n{}",
                                                    stderr_lines.join("\n")
                                                );
                                            }
                                        }
                                    }

                                    assert!(exit_status.success(), "deploy final build failed:\n{}", stderr_lines.join("\n"));

                                    return Ok(BuildOutput {
                                        bin_data: data,
                                        bin_path: path_buf,
                                        shared_library_path: if params.is_dylib {
                                            Some(per_job_target_dir.join("debug").join("deps"))
                                        } else {
                                            None
                                        },
                                    });
                                }
                            }
                            cargo_metadata::Message::CompilerMessage(mut msg) => {
                                // Update the path displayed to enable clicking in IDE.
                                // TODO(mingwei): deduplicate code with hydro_lang sim/graph.rs
                                if let Some(rendered) = msg.message.rendered.as_mut() {
                                    let file_names = msg
                                        .message
                                        .spans
                                        .iter()
                                        .map(|s| &s.file_name)
                                        .collect::<std::collections::BTreeSet<_>>();
                                    for file_name in file_names {
                                        if Path::new(file_name).is_relative() {
                                            *rendered = rendered.replace(
                                                file_name,
                                                &format!(
                                                    "(full path) {}/{file_name}",
                                                    params.workspace_root.display(),
                                                ),
                                            )
                                        }
                                    }
                                }
                                ProgressTracker::println(msg.message.to_string());
                                diagnostics.push(msg.message);
                            }
                            cargo_metadata::Message::TextLine(line) => {
                                ProgressTracker::println(&line);
                                text_lines.push(line);
                            }
                            cargo_metadata::Message::BuildFinished(_) => {}
                            cargo_metadata::Message::BuildScriptExecuted(_) => {}
                            msg => panic!("Unexpected message type: {:?}", msg),
                        }
                    }

                    let exit_status = spawned.wait().unwrap();
                    if exit_status.success() {
                        Err(BuildError::NoBinaryEmitted)
                    } else {
                        let stderr_lines = stderr_worker
                            .join()
                            .expect("Stderr worker unexpectedly panicked.");

                        Err(BuildError::FailedToBuildCrate {
                            exit_status,
                            diagnostics,
                            text_lines,
                            stderr_lines,
                        })
                    }
                })
                .await
                .map_err(|_| BuildError::TokioJoinError)?
            })
        })
        .await
}

#[derive(Clone, Debug)]
pub enum BuildError {
    FailedToBuildCrate {
        exit_status: ExitStatus,
        diagnostics: Vec<Diagnostic>,
        text_lines: Vec<String>,
        stderr_lines: Vec<String>,
    },
    TokioJoinError,
    NoBinaryEmitted,
}

impl Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FailedToBuildCrate {
                exit_status,
                diagnostics,
                text_lines,
                stderr_lines,
            } => {
                writeln!(f, "Failed to build crate ({})", exit_status)?;
                writeln!(f, "Diagnostics ({}):", diagnostics.len())?;
                for diagnostic in diagnostics {
                    write!(f, "{}", diagnostic)?;
                }
                writeln!(f, "Text output ({} lines):", text_lines.len())?;
                for line in text_lines {
                    writeln!(f, "{}", line)?;
                }
                writeln!(f, "Stderr output ({} lines):", stderr_lines.len())?;
                for line in stderr_lines {
                    writeln!(f, "{}", line)?;
                }
            }
            Self::TokioJoinError => {
                write!(f, "Failed to spawn tokio blocking task.")?;
            }
            Self::NoBinaryEmitted => {
                write!(f, "`cargo build` succeeded but no binary was emitted.")?;
            }
        }
        Ok(())
    }
}

impl Error for BuildError {}
