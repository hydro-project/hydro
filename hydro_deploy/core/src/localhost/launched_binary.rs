#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
use std::process::ExitStatus;
#[cfg(feature = "profile-folding")]
use std::process::Stdio;
#[cfg(feature = "profile-folding")]
use std::sync::OnceLock;

use anyhow::Result;
#[cfg(feature = "profile-folding")]
use anyhow::bail;
#[cfg(feature = "profile-folding")]
use async_process::Command;
use async_trait::async_trait;
use futures::io::BufReader as FuturesBufReader;
use futures::{AsyncBufReadExt as _, AsyncWriteExt as _};
#[cfg(feature = "profile-folding")]
use inferno::collapse::Collapse;
#[cfg(feature = "profile-folding")]
use inferno::collapse::perf::Folder as PerfFolder;
use tempfile::NamedTempFile;
#[cfg(feature = "profile-folding")]
use tokio::io::{AsyncBufReadExt as _, BufReader as TokioBufReader};
use tokio::sync::{mpsc, oneshot};
#[cfg(feature = "profile-folding")]
use tokio_util::compat::FuturesAsyncReadCompatExt;
#[cfg(feature = "profile-folding")]
use tokio_util::io::SyncIoBridge;

#[cfg(feature = "profile-folding")]
#[cfg(any(target_os = "macos", target_family = "windows"))]
use super::samply::samply_to_folded;
use crate::LaunchedBinary;
#[cfg(feature = "profile-folding")]
use crate::TracingResults;
use crate::progress::ProgressTracker;
#[cfg(feature = "profile-folding")]
use crate::rust_crate::flamegraph::handle_fold_data;
use crate::rust_crate::tracing_options::TracingOptions;
use crate::util::{PriorityBroadcast, prioritized_broadcast};

pub(super) struct TracingDataLocal {
    pub(super) outfile: NamedTempFile,
}

pub struct LaunchedLocalhostBinary {
    /// Must use async mutex -- we will .await methods within the child (while holding lock).
    child: tokio::sync::Mutex<async_process::Child>,
    tracing_config: Option<TracingOptions>,
    tracing_data_local: std::sync::Mutex<Option<TracingDataLocal>>,
    #[cfg(feature = "profile-folding")]
    tracing_results: OnceLock<TracingResults>,
    stdin_sender: mpsc::UnboundedSender<String>,
    stdout_broadcast: PriorityBroadcast,
    stderr_broadcast: PriorityBroadcast,
}

#[cfg(unix)]
impl Drop for LaunchedLocalhostBinary {
    fn drop(&mut self) {
        let child = self.child.get_mut();

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
        let stdout_broadcast = prioritized_broadcast(
            FuturesBufReader::new(child.stdout.take().unwrap()).lines(),
            move |s| ProgressTracker::println(format!("[{id_clone}] {s}")),
        );
        let stderr_broadcast = prioritized_broadcast(
            FuturesBufReader::new(child.stderr.take().unwrap()).lines(),
            move |s| ProgressTracker::println(format!("[{id} stderr] {s}")),
        );

        Self {
            child: tokio::sync::Mutex::new(child),
            tracing_config,
            tracing_data_local: std::sync::Mutex::new(tracing_data_local),
            #[cfg(feature = "profile-folding")]
            tracing_results: OnceLock::new(),
            stdin_sender,
            stdout_broadcast,
            stderr_broadcast,
        }
    }
}

#[async_trait]
impl LaunchedBinary for LaunchedLocalhostBinary {
    fn stdin(&self) -> mpsc::UnboundedSender<String> {
        self.stdin_sender.clone()
    }

    fn deploy_stdout(&self) -> oneshot::Receiver<String> {
        self.stdout_broadcast.receive_priority()
    }

    fn stdout(&self) -> mpsc::UnboundedReceiver<String> {
        self.stdout_broadcast.receive(None)
    }

    fn stderr(&self) -> mpsc::UnboundedReceiver<String> {
        self.stderr_broadcast.receive(None)
    }

    fn stdout_filter(&self, prefix: String) -> mpsc::UnboundedReceiver<String> {
        self.stdout_broadcast.receive(Some(prefix))
    }

    fn stderr_filter(&self, prefix: String) -> mpsc::UnboundedReceiver<String> {
        self.stderr_broadcast.receive(Some(prefix))
    }

    #[cfg(feature = "profile-folding")]
    fn tracing_results(&self) -> Option<&TracingResults> {
        self.tracing_results.get()
    }

    fn exit_code(&self) -> Option<i32> {
        self.child
            .try_lock()
            .ok()
            .and_then(|mut child| child.try_status().ok())
            .flatten()
            .map(exit_code)
    }

    async fn wait(&self) -> Result<i32> {
        Ok(exit_code(self.child.lock().await.status().await?))
    }

    async fn stop(&self) -> Result<()> {
        if let Err(err) = { self.child.lock().await.kill() }
            && !matches!(err.kind(), std::io::ErrorKind::InvalidInput)
        {
            Err(err)?;
        }

        // Run perf post-processing and download perf output.
        if let Some(tracing_config) = self.tracing_config.as_ref() {
            #[cfg(feature = "profile-folding")]
            assert!(
                self.tracing_results.get().is_none(),
                "`tracing_results` already set! Was `stop()` called twice? This is a bug."
            );
            let tracing_data =
                {
                    self.tracing_data_local.lock().unwrap().take().expect(
                        "`tracing_data_local` empty, was `stop()` called twice? This is a bug.",
                    )
                };

            if cfg!(any(target_os = "macos", target_family = "windows")) {
                if let Some(samply_outfile) = tracing_config.samply_outfile.as_ref() {
                    std::fs::copy(&tracing_data.outfile, samply_outfile)?;
                }
            } else if cfg!(target_family = "unix")
                && let Some(perf_outfile) = tracing_config.perf_raw_outfile.as_ref()
            {
                std::fs::copy(&tracing_data.outfile, perf_outfile)?;
            }

            #[cfg(feature = "profile-folding")]
            let fold_data = if cfg!(any(target_os = "macos", target_family = "windows")) {
                #[cfg(any(target_os = "macos", target_family = "windows"))]
                {
                    let loaded =
                        serde_json::from_reader(std::fs::File::open(tracing_data.outfile.path())?)?;

                    ProgressTracker::leaf("processing samply", samply_to_folded(loaded))
                        .await
                        .into()
                }

                #[cfg(not(any(target_os = "macos", target_family = "windows")))]
                {
                    unreachable!()
                }
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

                let mut fold_er =
                    PerfFolder::from(tracing_config.fold_perf_options.clone().unwrap_or_default());

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
                    "Unknown OS for samply/perf tracing: {}",
                    std::env::consts::OS
                );
            };

            #[cfg(feature = "profile-folding")]
            handle_fold_data(tracing_config, fold_data.clone()).await?;

            #[cfg(feature = "profile-folding")]
            self.tracing_results
                .set(TracingResults {
                    folded_data: fold_data,
                })
                .expect("`tracing_results` already set! This is a bug.");
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
