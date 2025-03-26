#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
use std::process::{ExitStatus, Stdio};
use std::sync::{Arc, Mutex};

use anyhow::{Result, bail};
use async_process::Command;
use async_trait::async_trait;
use futures::io::BufReader as FuturesBufReader;
use futures::{AsyncBufReadExt as _, AsyncWriteExt as _};
use inferno::collapse::Collapse;
use inferno::collapse::dtrace::Folder as DtraceFolder;
use inferno::collapse::perf::Folder as PerfFolder;
use tempfile::NamedTempFile;
use tokio::io::{AsyncBufReadExt as _, BufReader as TokioBufReader};
use tokio::sync::{mpsc, oneshot};
use tokio_util::compat::FuturesAsyncReadCompatExt;
use tokio_util::io::SyncIoBridge;

use super::samply::{FxProfile, samply_to_folded};
use crate::progress::ProgressTracker;
use crate::rust_crate::flamegraph::handle_fold_data;
use crate::rust_crate::tracing_options::TracingOptions;
use crate::ssh::PrefixFilteredChannel;
use crate::util::prioritized_broadcast;
use crate::{LaunchedBinary, TracingResults};

pub(super) struct TracingDataLocal {
    pub(super) outfile: NamedTempFile,
}

pub struct LaunchedLocalhostBinary {
    child: Mutex<async_process::Child>,
    tracing_config: Option<TracingOptions>,
    tracing_data_local: Option<TracingDataLocal>,
    tracing_results: Option<TracingResults>,
    stdin_sender: mpsc::UnboundedSender<String>,
    stdout_deploy_receivers: Arc<Mutex<Option<oneshot::Sender<String>>>>,
    stdout_receivers: Arc<Mutex<Vec<PrefixFilteredChannel>>>,
    stderr_receivers: Arc<Mutex<Vec<PrefixFilteredChannel>>>,
}

#[cfg(unix)]
impl Drop for LaunchedLocalhostBinary {
    fn drop(&mut self) {
        let mut child = self.child.lock().unwrap();

        if let Ok(Some(_)) = child.try_status() {
            return;
        }

        let pid = child.id();
        if let Err(e) = nix::sys::signal::kill(
            nix::unistd::Pid::from_raw(pid as i32),
            nix::sys::signal::SIGTERM,
        ) {
            ProgressTracker::println(format!("Failed to SIGTERM process {}: {}", pid, e));
        }
    }
}

impl LaunchedLocalhostBinary {
    pub(super) fn new(
        mut child: async_process::Child,
        id: String,
        tracing_config: Option<TracingOptions>,
        tracing_data_local: Option<TracingDataLocal>,
    ) -> Self {
        let (stdin_sender, mut stdin_receiver) = mpsc::unbounded_channel::<String>();
        let mut stdin = child.stdin.take().unwrap();
        tokio::spawn(async move {
            while let Some(line) = stdin_receiver.recv().await {
                if stdin.write_all(line.as_bytes()).await.is_err() {
                    break;
                }

                stdin.flush().await.ok();
            }
        });

        let id_clone = id.clone();
        let (stdout_deploy_receivers, stdout_receivers) = prioritized_broadcast(
            FuturesBufReader::new(child.stdout.take().unwrap()).lines(),
            move |s| ProgressTracker::println(format!("[{id_clone}] {s}")),
        );
        let (_, stderr_receivers) = prioritized_broadcast(
            FuturesBufReader::new(child.stderr.take().unwrap()).lines(),
            move |s| ProgressTracker::println(format!("[{id} stderr] {s}")),
        );

        Self {
            child: Mutex::new(child),
            tracing_config,
            tracing_data_local,
            tracing_results: None,
            stdin_sender,
            stdout_deploy_receivers,
            stdout_receivers,
            stderr_receivers,
        }
    }
}

#[async_trait]
impl LaunchedBinary for LaunchedLocalhostBinary {
    fn stdin(&self) -> mpsc::UnboundedSender<String> {
        self.stdin_sender.clone()
    }

    fn deploy_stdout(&self) -> oneshot::Receiver<String> {
        let mut receivers = self.stdout_deploy_receivers.lock().unwrap();

        if receivers.is_some() {
            panic!("Only one deploy stdout receiver is allowed at a time");
        }

        let (sender, receiver) = oneshot::channel::<String>();
        *receivers = Some(sender);
        receiver
    }

    fn stdout(&self) -> mpsc::UnboundedReceiver<String> {
        let mut receivers = self.stdout_receivers.lock().unwrap();
        let (sender, receiver) = mpsc::unbounded_channel::<String>();
        receivers.push((None, sender));
        receiver
    }

    fn stderr(&self) -> mpsc::UnboundedReceiver<String> {
        let mut receivers = self.stderr_receivers.lock().unwrap();
        let (sender, receiver) = mpsc::unbounded_channel::<String>();
        receivers.push((None, sender));
        receiver
    }

    fn stdout_filter(&self, prefix: String) -> mpsc::UnboundedReceiver<String> {
        let mut receivers = self.stdout_receivers.lock().unwrap();
        let (sender, receiver) = mpsc::unbounded_channel::<String>();
        receivers.push((Some(prefix), sender));
        receiver
    }

    fn stderr_filter(&self, prefix: String) -> mpsc::UnboundedReceiver<String> {
        let mut receivers = self.stderr_receivers.lock().unwrap();
        let (sender, receiver) = mpsc::unbounded_channel::<String>();
        receivers.push((Some(prefix), sender));
        receiver
    }

    fn tracing_results(&self) -> Option<&TracingResults> {
        self.tracing_results.as_ref()
    }

    fn exit_code(&self) -> Option<i32> {
        self.child
            .lock()
            .unwrap()
            .try_status()
            .ok()
            .flatten()
            .map(exit_code)
    }

    async fn wait(&mut self) -> Result<i32> {
        Ok(exit_code(self.child.get_mut().unwrap().status().await?))
    }

    async fn stop(&mut self) -> Result<()> {
        if let Err(err) = self.child.get_mut().unwrap().kill() {
            if !matches!(err.kind(), std::io::ErrorKind::InvalidInput) {
                Err(err)?;
            }
        }

        // Run perf post-processing and download perf output.
        if let Some(tracing_config) = self.tracing_config.as_ref() {
            if self.tracing_results.is_none() {
                let tracing_data = self.tracing_data_local.take().unwrap();

                if cfg!(target_os = "macos") || cfg!(target_family = "windows") {
                    if let Some(samply_outfile) = tracing_config.samply_outfile.as_ref() {
                        std::fs::copy(&tracing_data.outfile, samply_outfile)?;
                    }
                } else if cfg!(target_family = "unix") {
                    if let Some(perf_outfile) = tracing_config.perf_raw_outfile.as_ref() {
                        std::fs::copy(&tracing_data.outfile, perf_outfile)?;
                    }
                }

                let fold_data = if cfg!(target_os = "macos") {
                    let loaded = serde_json::from_reader::<_, FxProfile>(std::fs::File::open(
                        tracing_data.outfile.path(),
                    )?)?;

                    samply_to_folded(loaded).await.into()
                } else if cfg!(target_family = "windows") {
                    let mut fold_er = DtraceFolder::from(
                        tracing_config
                            .fold_dtrace_options
                            .clone()
                            .unwrap_or_default(),
                    );

                    let fold_data =
                        ProgressTracker::leaf("fold dtrace output".to_owned(), async move {
                            let mut fold_data = Vec::new();
                            fold_er.collapse_file(Some(tracing_data.outfile), &mut fold_data)?;
                            Result::<_>::Ok(fold_data)
                        })
                        .await?;
                    fold_data
                } else if cfg!(target_family = "unix") {
                    // Run perf script.
                    let mut perf_script = Command::new("perf")
                        .args(["script", "--symfs=/", "-i"])
                        .arg(tracing_data.outfile.path())
                        .stdout(Stdio::piped())
                        .stderr(Stdio::piped())
                        .spawn()?;

                    let stdout = perf_script.stdout.take().unwrap().compat();
                    let mut stderr_lines =
                        TokioBufReader::new(perf_script.stderr.take().unwrap().compat()).lines();

                    let mut fold_er = PerfFolder::from(
                        tracing_config.fold_perf_options.clone().unwrap_or_default(),
                    );

                    // Pattern on `()` to make sure no `Result`s are ignored.
                    let ((), fold_data, ()) = tokio::try_join!(
                        async move {
                            // Log stderr.
                            while let Ok(Some(s)) = stderr_lines.next_line().await {
                                ProgressTracker::println(format!("[perf script stderr] {s}"));
                            }
                            Result::<_>::Ok(())
                        },
                        async move {
                            // Stream `perf script` stdout and fold.
                            tokio::task::spawn_blocking(move || {
                                let mut fold_data = Vec::new();
                                fold_er.collapse(
                                    SyncIoBridge::new(tokio::io::BufReader::new(stdout)),
                                    &mut fold_data,
                                )?;
                                Ok(fold_data)
                            })
                            .await?
                        },
                        async move {
                            // Close stdin and wait for command exit.
                            perf_script.status().await?;
                            Ok(())
                        },
                    )?;
                    fold_data
                } else {
                    bail!(
                        "Unknown OS for perf/dtrace tracing: {}",
                        std::env::consts::OS
                    );
                };

                handle_fold_data(tracing_config, fold_data.clone()).await?;

                self.tracing_results = Some(TracingResults {
                    folded_data: fold_data,
                });
            }
        };

        Ok(())
    }
}

fn exit_code(c: ExitStatus) -> i32 {
    #[cfg(unix)]
    return c.code().or(c.signal()).unwrap();
    #[cfg(not(unix))]
    return c.code().unwrap();
}
