use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context as _, Result};
use async_ssh2_russh::russh::client::{Config, Handler};
use async_ssh2_russh::russh::{Disconnect, compression};
use async_ssh2_russh::russh_sftp::protocol::{Status, StatusCode};
use async_ssh2_russh::sftp::SftpError;
use async_ssh2_russh::{AsyncChannel, AsyncSession, NoCheckHandler};
use async_trait::async_trait;
use hydro_deploy_integration::ServerBindConfig;
use inferno::collapse::Collapse;
use inferno::collapse::perf::Folder;
use nanoid::nanoid;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot};
use tokio_stream::StreamExt;
use tokio_stream::wrappers::LinesStream;
use tokio_util::io::SyncIoBridge;

use crate::progress::ProgressTracker;
use crate::rust_crate::build::BuildOutput;
use crate::rust_crate::flamegraph::handle_fold_data;
use crate::rust_crate::tracing_options::TracingOptions;
use crate::util::{PriorityBroadcast, async_retry, prioritized_broadcast};
use crate::{LaunchedBinary, LaunchedHost, ResourceResult, ServerStrategy, TracingResults};

const PERF_OUTFILE: &str = "__profile.perf.data";

struct LaunchedSshBinary {
    _resource_result: Arc<ResourceResult>,
    // TODO(mingwei): instead of using `NoCheckHandler`, we should check the server's public key
    // fingerprint (get it somehow via terraform), but ssh `publickey` authentication already
    // generally prevents MITM attacks.
    session: Option<AsyncSession<NoCheckHandler>>,
    channel: AsyncChannel,
    stdin_sender: mpsc::UnboundedSender<String>,
    stdout_broadcast: PriorityBroadcast,
    stderr_broadcast: PriorityBroadcast,
    tracing: Option<TracingOptions>,
    tracing_results: Option<TracingResults>,
}

#[async_trait]
impl LaunchedBinary for LaunchedSshBinary {
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

    fn tracing_results(&self) -> Option<&TracingResults> {
        self.tracing_results.as_ref()
    }

    fn exit_code(&self) -> Option<i32> {
        // until the program exits, the exit status is meaningless
        self.channel
            .recv_exit_status()
            .try_get()
            .map(|&ec| ec as _)
            .ok()
    }

    async fn wait(&mut self) -> Result<i32> {
        self.channel.wait_close().await;
        Ok(self
            .channel
            .recv_exit_status()
            .try_get()
            .map(|&ec| ec as _)?)
    }

    async fn stop(&mut self) -> Result<()> {
        if !self.channel.is_closed() {
            ProgressTracker::leaf("force stopping", async {
                // self.channel.signal(russh::Sig::INT).await?; // `^C`
                self.channel.eof().await?; // Send EOF.
                self.channel.close().await?; // Close the channel.
                self.channel.wait_close().await;
                Result::<_>::Ok(())
            })
            .await?;
        }

        // Run perf post-processing and download perf output.
        if let Some(tracing) = self.tracing.as_ref() {
            let session = self.session.as_ref().unwrap();
            if let Some(local_raw_perf) = tracing.perf_raw_outfile.as_ref() {
                ProgressTracker::progress_leaf("downloading perf data", |progress, _| async move {
                    let sftp =
                        async_retry(&|| session.open_sftp(), 10, Duration::from_secs(1)).await?;

                    let mut remote_raw_perf = sftp.open(PERF_OUTFILE).await?;
                    let mut local_raw_perf = File::create(local_raw_perf).await?;

                    let total_size = remote_raw_perf.metadata().await?.size.unwrap();

                    use tokio::io::AsyncWriteExt;
                    let mut index = 0;
                    loop {
                        let mut buffer = [0; 16 * 1024];
                        let n = remote_raw_perf.read(&mut buffer).await?;
                        if n == 0 {
                            break;
                        }
                        local_raw_perf.write_all(&buffer[..n]).await?;
                        index += n;
                        progress(((index as f64 / total_size as f64) * 100.0) as u64);
                    }

                    Ok::<(), anyhow::Error>(())
                })
                .await?;
            }

            let script_channel = session.open_channel().await?;
            let mut fold_er = Folder::from(tracing.fold_perf_options.clone().unwrap_or_default());

            let fold_data = ProgressTracker::leaf("perf script & folding", async move {
                let mut stderr_lines = script_channel.stderr().lines();
                let stdout = script_channel.stdout();

                // Pattern on `()` to make sure no `Result`s are ignored.
                let ((), fold_data, ()) = tokio::try_join!(
                    async move {
                        // Log stderr.
                        while let Ok(Some(s)) = stderr_lines.next_line().await {
                            ProgressTracker::eprintln(format!("[perf stderr] {s}"));
                        }
                        Result::<_>::Ok(())
                    },
                    async move {
                        // Download perf output and fold.
                        tokio::task::spawn_blocking(move || {
                            let mut fold_data = Vec::new();
                            fold_er.collapse(
                                SyncIoBridge::new(BufReader::new(stdout)),
                                &mut fold_data,
                            )?;
                            Ok(fold_data)
                        })
                        .await?
                    },
                    async move {
                        // Run command (last!).
                        script_channel
                            .exec(false, format!("perf script --symfs=/ -i {PERF_OUTFILE}"))
                            .await?;
                        Ok(())
                    },
                )?;
                Result::<_>::Ok(fold_data)
            })
            .await?;

            self.tracing_results = Some(TracingResults {
                folded_data: fold_data.clone(),
            });

            handle_fold_data(tracing, fold_data).await?;
        };

        Ok(())
    }
}

impl Drop for LaunchedSshBinary {
    fn drop(&mut self) {
        if let Some(session) = self.session.take() {
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(session.disconnect(
                    Disconnect::ByApplication,
                    "",
                    "",
                ))
            })
            .unwrap();
        }
    }
}

#[async_trait]
pub trait LaunchedSshHost: Send + Sync {
    fn get_internal_ip(&self) -> String;
    fn get_external_ip(&self) -> Option<String>;
    fn get_cloud_provider(&self) -> String;
    fn resource_result(&self) -> &Arc<ResourceResult>;
    fn ssh_user(&self) -> &str;

    fn ssh_key_path(&self) -> PathBuf {
        self.resource_result()
            .terraform
            .deployment_folder
            .as_ref()
            .unwrap()
            .path()
            .join(".ssh")
            .join("vm_instance_ssh_key_pem")
    }

    fn server_config(&self, bind_type: &ServerStrategy) -> ServerBindConfig {
        match bind_type {
            ServerStrategy::UnixSocket => ServerBindConfig::UnixSocket,
            ServerStrategy::InternalTcpPort => {
                ServerBindConfig::TcpPort(self.get_internal_ip().clone())
            }
            ServerStrategy::ExternalTcpPort(_) => todo!(),
            ServerStrategy::Demux(demux) => {
                let mut config_map = HashMap::new();
                for (key, underlying) in demux {
                    config_map.insert(*key, LaunchedSshHost::server_config(self, underlying));
                }

                ServerBindConfig::Demux(config_map)
            }
            ServerStrategy::Merge(merge) => {
                let mut configs = vec![];
                for underlying in merge {
                    configs.push(LaunchedSshHost::server_config(self, underlying));
                }

                ServerBindConfig::Merge(configs)
            }
            ServerStrategy::Tagged(underlying, id) => ServerBindConfig::Tagged(
                Box::new(LaunchedSshHost::server_config(self, underlying)),
                *id,
            ),
            ServerStrategy::Null => ServerBindConfig::Null,
        }
    }

    async fn open_ssh_session(&self) -> Result<AsyncSession<NoCheckHandler>> {
        let target_addr = SocketAddr::new(
            self.get_external_ip()
                .as_ref()
                .context(
                    self.get_cloud_provider()
                        + " host must be configured with an external IP to launch binaries",
                )?
                .parse()
                .unwrap(),
            22,
        );

        let res = ProgressTracker::leaf(
            format!(
                "connecting to host @ {}",
                self.get_external_ip().as_ref().unwrap()
            ),
            async_retry(
                &|| async {
                    let mut config = Config::default();
                    config.preferred.compression = (&[
                        compression::ZLIB,
                        compression::ZLIB_LEGACY,
                        compression::NONE,
                    ])
                        .into();
                    AsyncSession::connect_publickey(
                        config,
                        target_addr,
                        self.ssh_user(),
                        self.ssh_key_path(),
                    )
                    .await
                },
                10,
                Duration::from_secs(1),
            ),
        )
        .await?;

        Ok(res)
    }
}

async fn create_channel<H>(session: &AsyncSession<H>) -> Result<AsyncChannel>
where
    H: 'static + Handler,
{
    async_retry(
        &|| async {
            Ok(tokio::time::timeout(Duration::from_secs(60), session.open_channel()).await??)
        },
        10,
        Duration::from_secs(1),
    )
    .await
}

#[async_trait]
impl<T: LaunchedSshHost> LaunchedHost for T {
    fn server_config(&self, bind_type: &ServerStrategy) -> ServerBindConfig {
        LaunchedSshHost::server_config(self, bind_type)
    }

    async fn copy_binary(&self, binary: &BuildOutput) -> Result<()> {
        let session = self.open_ssh_session().await?;

        let sftp = async_retry(&|| session.open_sftp(), 10, Duration::from_secs(1)).await?;

        let user = self.ssh_user();
        // we may be deploying multiple binaries, so give each a unique name
        let binary_path = format!("/home/{user}/hydro-{}", binary.unique_id());

        if sftp.metadata(&binary_path).await.is_err() {
            let random = nanoid!(8);
            let temp_path = format!("/home/{user}/hydro-{random}");
            let sftp = &sftp;

            ProgressTracker::progress_leaf(
                format!("uploading binary to {}", binary_path),
                |set_progress, _| {
                    async move {
                        let mut created_file = sftp.create(&temp_path).await?;

                        let mut index = 0;
                        while index < binary.bin_data.len() {
                            let written = created_file
                                .write(
                                    &binary.bin_data[index
                                        ..std::cmp::min(index + 128 * 1024, binary.bin_data.len())],
                                )
                                .await?;
                            index += written;
                            set_progress(
                                ((index as f64 / binary.bin_data.len() as f64) * 100.0) as u64,
                            );
                        }
                        let mut orig_file_stat = sftp.metadata(&temp_path).await?;
                        orig_file_stat.permissions = Some(0o755); // allow the copied binary to be executed by anyone
                        created_file.set_metadata(orig_file_stat).await?;
                        created_file.sync_all().await?;
                        drop(created_file);

                        match sftp.rename(&temp_path, binary_path).await {
                            Ok(_) => {}
                            Err(SftpError::Status(Status {
                                status_code: StatusCode::Failure, // SSH_FXP_STATUS = 4
                                ..
                            })) => {
                                // file already exists
                                sftp.remove_file(temp_path).await?;
                            }
                            Err(e) => return Err(e.into()),
                        }

                        anyhow::Ok(())
                    }
                },
            )
            .await?;
        }
        sftp.close().await?;

        Ok(())
    }

    async fn launch_binary(
        &self,
        id: String,
        binary: &BuildOutput,
        args: &[String],
        tracing: Option<TracingOptions>,
    ) -> Result<Box<dyn LaunchedBinary>> {
        let session = self.open_ssh_session().await?;

        let user = self.ssh_user();
        let binary_path = PathBuf::from(format!("/home/{user}/hydro-{}", binary.unique_id()));

        let mut command = binary_path.to_str().unwrap().to_owned();
        for arg in args {
            command.push(' ');
            command.push_str(&shell_escape::unix::escape(arg.into()))
        }

        // Launch with tracing if specified.
        if let Some(TracingOptions {
            frequency,
            setup_command,
            ..
        }) = tracing.clone()
        {
            let id_clone = id.clone();
            ProgressTracker::leaf("install perf", async {
                // Run setup command
                if let Some(setup_command) = setup_command {
                    let mut setup_channel = create_channel(&session).await?;
                    let (setup_stdout, setup_stderr) =
                        (setup_channel.stdout(), setup_channel.stderr());
                    setup_channel.exec(false, &*setup_command).await?;

                    // log outputs
                    let mut output_lines = LinesStream::new(setup_stdout.lines())
                        .merge(LinesStream::new(setup_stderr.lines()));
                    while let Some(line) = output_lines.next().await {
                        ProgressTracker::eprintln(format!(
                            "[{} install perf] {}",
                            id_clone,
                            line.unwrap()
                        ));
                    }

                    let exit_code = setup_channel.recv_exit_status().wait().await.copied();
                    setup_channel.wait_close().await;
                    if Some(0) != exit_code {
                        anyhow::bail!("Failed to install perf on remote host");
                    }
                }
                Ok(())
            })
            .await?;

            // Attach perf to the command
            // Note: `LaunchedSshHost` assumes `perf` on linux.
            command = format!(
                "perf record -F {frequency} -e cycles:u --call-graph dwarf,65528 -o {PERF_OUTFILE} {command}",
            );
        }

        let (channel, stdout, stderr) = ProgressTracker::leaf(
            format!("launching binary {}", binary_path.display()),
            async {
                let channel = create_channel(&session).await?;
                // Make sure to begin reading stdout/stderr before running the command.
                let (stdout, stderr) = (channel.stdout(), channel.stderr());
                channel.exec(false, command).await?;
                anyhow::Ok((channel, stdout, stderr))
            },
        )
        .await?;

        let (stdin_sender, mut stdin_receiver) = mpsc::unbounded_channel::<String>();
        let mut stdin = channel.stdin();

        tokio::spawn(async move {
            while let Some(line) = stdin_receiver.recv().await {
                if stdin.write_all(line.as_bytes()).await.is_err() {
                    break;
                }
                stdin.flush().await.unwrap();
            }
        });

        let id_clone = id.clone();
        let stdout_broadcast = prioritized_broadcast(LinesStream::new(stdout.lines()), move |s| {
            ProgressTracker::println(format!("[{id_clone}] {s}"));
        });
        let stderr_broadcast = prioritized_broadcast(LinesStream::new(stderr.lines()), move |s| {
            ProgressTracker::println(format!("[{id} stderr] {s}"));
        });

        Ok(Box::new(LaunchedSshBinary {
            _resource_result: self.resource_result().clone(),
            session: Some(session),
            channel,
            stdin_sender,
            // stdout_deploy_receivers,
            stdout_broadcast,
            stderr_broadcast,
            tracing,
            tracing_results: None,
        }))
    }

    async fn forward_port(&self, addr: &SocketAddr) -> Result<SocketAddr> {
        let session = self.open_ssh_session().await?;

        let local_port = TcpListener::bind("127.0.0.1:0").await?;
        let local_addr = local_port.local_addr()?;

        let internal_ip = addr.ip().to_string();
        let port = addr.port();

        tokio::spawn(async move {
            #[expect(clippy::never_loop, reason = "tcp accept loop pattern")]
            while let Ok((mut local_stream, _)) = local_port.accept().await {
                let mut channel = session
                    .channel_open_direct_tcpip(internal_ip, port.into(), "127.0.0.1", 22)
                    .await
                    .unwrap()
                    .into_stream();
                let _ = tokio::io::copy_bidirectional(&mut local_stream, &mut channel).await;
                break;
                // TODO(shadaj): we should be returning an Arc so that we know
                // if anyone wants to connect to this forwarded port
            }

            ProgressTracker::println("[hydro] closing forwarded port");
        });

        Ok(local_addr)
    }
}
