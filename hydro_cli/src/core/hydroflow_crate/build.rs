use std::collections::HashMap;
use std::io::BufRead;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use anyhow::{bail, Result};
use nanoid::nanoid;
use once_cell::sync::Lazy;
use tokio::sync::OnceCell;

use crate::core::progress::ProgressTracker;
use crate::core::HostTargetType;

type CacheKey = (PathBuf, Option<String>, HostTargetType, Option<Vec<String>>);

pub type BuildResult = Result<Arc<(String, Vec<u8>)>>;

static BUILDS: Lazy<Mutex<HashMap<CacheKey, Arc<OnceCell<BuildResult>>>>> =
    Lazy::new(Default::default);

pub async fn build_crate(
    src: PathBuf,
    example: Option<String>,
    target_type: HostTargetType,
    features: Option<Vec<String>>,
) -> BuildResult {
    let key = (src.clone(), example.clone(), target_type, features.clone());
    let unit_of_work = {
        let mut builds = BUILDS.lock().unwrap();
        builds.entry(key).or_default().clone()
        // Release BUILDS table lock here.
    };
    unit_of_work
        .get_or_init(move || {
            ProgressTracker::rich_leaf("build".to_string(), move |_, set_msg| async move {
                tokio::task::spawn_blocking(move || {
                    let mut command = Command::new("cargo");
                    command.args(["build".to_string(), "--release".to_string()]);

                    if let Some(example) = example.as_ref() {
                        command.args(["--example", example]);
                    }

                    match target_type {
                        HostTargetType::Local => {}
                        HostTargetType::Linux => {
                            command.args(["--target", "x86_64-unknown-linux-musl"]);
                        }
                    }

                    if let Some(features) = features {
                        command.args(["--features", &features.join(",")]);
                    }

                    command.arg("--message-format=json-diagnostic-rendered-ansi");

                    let mut spawned = command
                        .current_dir(&src)
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

                    for message in cargo_metadata::Message::parse_stream(reader) {
                        match message.unwrap() {
                            cargo_metadata::Message::CompilerArtifact(artifact) => {
                                let is_output = if example.is_some() {
                                    artifact.target.kind.contains(&"example".to_string())
                                } else {
                                    artifact.target.kind.contains(&"bin".to_string())
                                };

                                if is_output {
                                    let path = artifact.executable.unwrap();
                                    let path = path.into_string();
                                    let data = std::fs::read(path).unwrap();
                                    return Ok(Arc::new((nanoid!(8), data)));
                                }
                            }
                            cargo_metadata::Message::CompilerMessage(msg) => {
                                ProgressTracker::println(&msg.message.rendered.unwrap())
                            }
                            _ => {}
                        }
                    }

                    if spawned.wait().unwrap().success() {
                        bail!("cargo build succeeded but no binary was emitted")
                    } else {
                        bail!("failed to build crate")
                    }
                })
                .await
                .unwrap()
            })
        })
        .await
        .as_ref()
        .map(|x| x.clone())
        .map_err(|e| anyhow::anyhow!(e.to_string()))
}
