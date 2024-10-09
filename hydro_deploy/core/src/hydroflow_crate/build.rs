use std::error::Error;
use std::fmt::Display;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use cargo_metadata::diagnostic::Diagnostic;
use memo_map::MemoMap;
use nanoid::nanoid;
use tokio::sync::OnceCell;

use crate::progress::ProgressTracker;
use crate::HostTargetType;

/// Build parameters for [`build_crate_memoized`].
#[derive(PartialEq, Eq, Hash, Clone)]
pub struct BuildParams {
    /// The working directory for the build, where the `cargo build` command will be run. Crate root.
    /// [`Self::new`] canonicalizes this path.
    src: PathBuf,
    /// `--bin` binary name parameter.
    bin: Option<String>,
    /// `--example` parameter.
    example: Option<String>,
    /// `--profile` parameter.
    profile: Option<String>,
    rustflags: Option<String>,
    target_dir: Option<PathBuf>,
    no_default_features: bool,
    /// `--target <linux>` if cross-compiling for linux ([`HostTargetType::Linux`]).
    target_type: HostTargetType,
    /// `--features` flags, will be comma-delimited.
    features: Option<Vec<String>>,
}
impl BuildParams {
    /// Creates a new `BuildParams` and canonicalizes the `src` path.
    #[expect(clippy::too_many_arguments, reason = "internal code")]
    pub fn new(
        src: impl AsRef<Path>,
        bin: Option<String>,
        example: Option<String>,
        profile: Option<String>,
        rustflags: Option<String>,
        target_dir: Option<PathBuf>,
        no_default_features: bool,
        target_type: HostTargetType,
        features: Option<Vec<String>>,
    ) -> Self {
        // `fs::canonicalize` prepends windows paths with the `r"\\?\"`
        // https://stackoverflow.com/questions/21194530/what-does-mean-when-prepended-to-a-file-path
        // However, this breaks the `include!(concat!(env!("OUT_DIR"), "/my/forward/slash/path.rs"))`
        // Rust codegen pattern on windows. To help mitigate this happening in third party crates, we
        // instead use `dunce::canonicalize` which is the same as `fs::canonicalize` but avoids the
        // `\\?\` prefix when possible.
        let src = dunce::canonicalize(src).expect("Failed to canonicalize path for build.");

        BuildParams {
            src,
            bin,
            example,
            profile,
            rustflags,
            target_dir,
            no_default_features,
            target_type,
            features,
        }
    }
}

/// Information about a built crate. See [`build_crate`].
pub struct BuildOutput {
    /// A unique but meaningless id.
    pub unique_id: String,
    /// The binary contents as a byte array.
    pub bin_data: Vec<u8>,
    /// The path to the binary file. [`Self::bin_data`] has a copy of the content.
    pub bin_path: PathBuf,
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
                    let mut command = Command::new("cargo");
                    command.args(["build"]);

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
                        HostTargetType::Linux => {
                            command.args(["--target", "x86_64-unknown-linux-musl"]);
                        }
                    }

                    if params.no_default_features {
                        command.arg("--no-default-features");
                    }

                    if let Some(features) = params.features {
                        command.args(["--features", &features.join(",")]);
                    }

                    command.arg("--message-format=json-diagnostic-rendered-ansi");

                    if let Some(rustflags) = params.rustflags.as_ref() {
                        command.env("RUSTFLAGS", rustflags);
                    }

                    if let Some(target_dir) = params.target_dir.as_ref() {
                        command.env("CARGO_TARGET_DIR", target_dir);
                    }

                    let mut spawned = command
                        .current_dir(&params.src)
                        .stdout(Stdio::piped())
                        .stderr(Stdio::piped())
                        .stdin(Stdio::null())
                        .spawn()
                        .unwrap();

                    let reader = std::io::BufReader::new(spawned.stdout.take().unwrap());
                    let mut stderr_reader = std::io::BufReader::new(spawned.stderr.take().unwrap());
                    std::thread::spawn(move || loop {
                        let mut buf = String::new();
                        if let Ok(size) = stderr_reader.read_line(&mut buf) {
                            if size == 0 {
                                break;
                            } else {
                                set_msg(buf.trim().to_string());
                            }
                        } else {
                            break;
                        }
                    });

                    let mut diagnostics = Vec::new();
                    for message in cargo_metadata::Message::parse_stream(reader) {
                        match message.unwrap() {
                            cargo_metadata::Message::CompilerArtifact(artifact) => {
                                let is_output = if params.example.is_some() {
                                    artifact.target.kind.contains(&"example".to_string())
                                } else {
                                    artifact.target.kind.contains(&"bin".to_string())
                                };

                                if is_output {
                                    let path = artifact.executable.unwrap();
                                    let path_buf: PathBuf = path.clone().into();
                                    let path = path.into_string();
                                    let data = std::fs::read(path).unwrap();
                                    return Ok(BuildOutput {
                                        unique_id: nanoid!(8),
                                        bin_data: data,
                                        bin_path: path_buf,
                                    });
                                }
                            }
                            cargo_metadata::Message::CompilerMessage(msg) => {
                                ProgressTracker::println(msg.message.rendered.as_deref().unwrap());
                                diagnostics.push(msg.message);
                            }
                            _ => {}
                        }
                    }

                    if spawned.wait().unwrap().success() {
                        Err(BuildError::NoBinaryEmitted)
                    } else {
                        Err(BuildError::FailedToBuildCrate(diagnostics))
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
    FailedToBuildCrate(Vec<Diagnostic>),
    TokioJoinError,
    NoBinaryEmitted,
}

impl Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FailedToBuildCrate(diagnostics) => {
                writeln!(f, "Failed to build crate:")?;
                for diagnostic in diagnostics {
                    write!(f, "{}", diagnostic)?;
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
