use std::borrow::Cow;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context as _, Result};
use async_ssh2_lite::ssh2::ErrorCode;
use async_ssh2_lite::{AsyncChannel, AsyncSession, SessionConfiguration};
use async_trait::async_trait;
use futures::io::BufReader as FuturesBufReader;
use futures::{AsyncBufReadExt, AsyncWriteExt};
use hydro_deploy_integration::ServerBindConfig;
use inferno::collapse::Collapse;
use inferno::collapse::perf::Folder;
use nanoid::nanoid;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, BufReader as TokioBufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::Handle;
use tokio::sync::{mpsc, oneshot};
use tokio_stream::StreamExt;
use tokio_util::compat::FuturesAsyncReadCompatExt;
use tokio_util::io::SyncIoBridge;

use crate::progress::ProgressTracker;
use crate::rust_crate::build::BuildOutput;
use crate::rust_crate::flamegraph::handle_fold_data;
use crate::rust_crate::tracing_options::TracingOptions;
use crate::util::{async_retry, prioritized_broadcast};
use crate::{LaunchedBinary, LaunchedHost, ResourceResult, ServerStrategy, TracingResults};

const PERF_OUTFILE: &str = "__profile.perf.data";

pub type PrefixFilteredChannel = (Option<String>, mpsc::UnboundedSender<String>);

struct LaunchedSshBinary {
    _resource_result: Arc<ResourceResult>,
    session: Option<AsyncSession<TcpStream>>,
    channel: AsyncChannel<TcpStream>,
    stdin_sender: mpsc::UnboundedSender<String>,
    stdout_receivers: Arc<Mutex<Vec<PrefixFilteredChannel>>>,
    stdout_deploy_receivers: Arc<Mutex<Option<oneshot::Sender<String>>>>,
    stderr_receivers: Arc<Mutex<Vec<PrefixFilteredChannel>>>,
    tracing: Option<TracingOptions>,
    tracing_results: Option<TracingResults>,
}

#[async_trait]
impl LaunchedBinary for LaunchedSshBinary {
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
        // until the program exits, the exit status is meaningless
        if self.channel.eof() {
            self.channel.exit_status().ok()
        } else {
            None
        }
    }

    async fn wait(&mut self) -> Result<i32> {
        self.channel.wait_eof().await.unwrap();
        let exit_code = self.channel.exit_status()?;
        self.channel.wait_close().await.unwrap();

        Ok(exit_code)
    }

    async fn stop(&mut self) -> Result<()> {
        if !self.channel.eof() {
            ProgressTracker::leaf("force stopping", async {
                self.channel.write_all(b"\x03").await?; // `^C`
                self.channel.send_eof().await?;
                self.channel.wait_eof().await?;
                // `exit_status()`
                self.channel.wait_close().await?;
                Result::<_>::Ok(())
            })
            .await?;
        }

        // Run perf post-processing and download perf output.
        if let Some(tracing) = self.tracing.as_ref() {
            let session = self.session.as_ref().unwrap();
            if let Some(local_raw_perf) = tracing.perf_raw_outfile.as_ref() {
                ProgressTracker::progress_leaf("downloading perf data", |progress, _| async move {
                    let sftp = async_retry(
                        &|| async { Ok(session.sftp().await?) },
                        10,
                        Duration::from_secs(1),
                    )
                    .await?;

                    let mut remote_raw_perf = sftp.open(&PathBuf::from(PERF_OUTFILE)).await?;
                    let mut local_raw_perf = File::create(local_raw_perf).await?;

                    let total_size = remote_raw_perf.stat().await?.size.unwrap();
                    let mut remote_tokio = remote_raw_perf.compat();

                    use tokio::io::AsyncWriteExt;
                    let mut index = 0;
                    loop {
                        let mut buffer = [0; 16 * 1024];
                        let n = remote_tokio.read(&mut buffer).await?;
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

            let mut script_channel = session.channel_session().await?;
            let mut fold_er = Folder::from(tracing.fold_perf_options.clone().unwrap_or_default());

            let fold_data = ProgressTracker::leaf("perf script & folding", async move {
                let mut stderr_lines = FuturesBufReader::new(script_channel.stderr()).lines();
                let stdout = script_channel.stream(0);

                // Pattern on `()` to make sure no `Result`s are ignored.
                let ((), fold_data, ()) = tokio::try_join!(
                    async move {
                        // Log stderr.
                        while let Some(Ok(s)) = stderr_lines.next().await {
                            ProgressTracker::eprintln(format!("[perf stderr] {s}"));
                        }
                        Result::<_>::Ok(())
                    },
                    async move {
                        // Download perf output and fold.
                        tokio::task::spawn_blocking(move || {
                            let mut fold_data = Vec::new();
                            fold_er.collapse(
                                SyncIoBridge::new(TokioBufReader::new(stdout)),
                                &mut fold_data,
                            )?;
                            Ok(fold_data)
                        })
                        .await?
                    },
                    async move {
                        // Run command (last!).
                        script_channel
                            .exec(&format!("perf script --symfs=/ -i {PERF_OUTFILE}"))
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
                Handle::current().block_on(session.disconnect(None, "", None))
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

    async fn open_ssh_session(&self) -> Result<AsyncSession<TcpStream>> {
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
                    let mut config = SessionConfiguration::new();
                    config.set_compress(true);

                    let mut session =
                        AsyncSession::<TcpStream>::connect(target_addr, Some(config)).await?;

                    session.handshake().await?;

                    session
                        .userauth_pubkey_file(
                            self.ssh_user(),
                            None,
                            self.ssh_key_path().as_path(),
                            None,
                        )
                        .await?;

                    Ok(session)
                },
                10,
                Duration::from_secs(1),
            ),
        )
        .await?;

        Ok(res)
    }
}

async fn create_channel(session: &AsyncSession<TcpStream>) -> Result<AsyncChannel<TcpStream>> {
    async_retry(
        &|| async {
            Ok(tokio::time::timeout(
                Duration::from_secs(60),
                session.channel_session(),
            )
            .await??)
        },
        10,
        Duration::from_secs(1),
    ).await
}

#[async_trait]
impl<T: LaunchedSshHost> LaunchedHost for T {
    fn server_config(&self, bind_type: &ServerStrategy) -> ServerBindConfig {
        LaunchedSshHost::server_config(self, bind_type)
    }

    async fn copy_binary(&self, binary: &BuildOutput) -> Result<()> {
        let session = self.open_ssh_session().await?;

        let sftp = async_retry(
            &|| async { Ok(session.sftp().await?) },
            10,
            Duration::from_secs(1),
        )
        .await?;

        // we may be deploying multiple binaries, so give each a unique name
        let unique_name = &binary.unique_id;

        let user = self.ssh_user();
        let binary_path = PathBuf::from(format!("/home/{user}/hydro-{unique_name}"));

        if sftp.stat(&binary_path).await.is_err() {
            let random = nanoid!(8);
            let temp_path = PathBuf::from(format!("/home/{user}/hydro-{random}"));
            let sftp = &sftp;

            ProgressTracker::progress_leaf(
                format!("uploading binary to {}", binary_path.display()),
                |set_progress, _| {
                    let binary = &binary;
                    let binary_path = &binary_path;
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
                        let mut orig_file_stat = sftp.stat(&temp_path).await?;
                        orig_file_stat.perm = Some(0o755); // allow the copied binary to be executed by anyone
                        created_file.setstat(orig_file_stat).await?;
                        created_file.close().await?;
                        drop(created_file);

                        match sftp.rename(&temp_path, binary_path, None).await {
                            Ok(_) => {}
                            Err(async_ssh2_lite::Error::Ssh2(e))
                                if e.code() == ErrorCode::SFTP(4) =>
                            {
                                // file already exists
                                sftp.unlink(&temp_path).await?;
                            }
                            Err(e) => return Err(e.into()),
                        }

                        anyhow::Ok(())
                    }
                },
            )
            .await?;
        }
        drop(sftp);

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

        let unique_name = &binary.unique_id;

        let user = self.ssh_user();
        let binary_path = PathBuf::from(format!("/home/{user}/hydro-{unique_name}"));

        let channel = ProgressTracker::leaf(format!("launching binary {}", binary_path.display()),
            async {
                let mut channel = create_channel(&session).await?;

                let mut command = binary_path.to_str().unwrap().to_owned();
                for arg in args{
                    command.push(' ');
                    command.push_str(&shell_escape::unix::escape(Cow::Borrowed(arg)))
                }
                // Launch with perf if specified, also copy local binary to expected place for perf report to work
                if let Some(TracingOptions { frequency, .. }) = tracing.clone() {

                    // Install perf
                    let mut perf_install_channel = create_channel(&session).await?;
                    perf_install_channel
                        .exec("sudo sh -c 'apt update && apt install -y linux-perf binutils && echo -1 > /proc/sys/kernel/perf_event_paranoid && echo 0 > /proc/sys/kernel/kptr_restrict'")
                        .await?;

                    // log outputs
                    let mut perf_install_out = FuturesBufReader::new(perf_install_channel.stream(0)).lines();
                    while let Some(line) = perf_install_out.next().await {
                        ProgressTracker::eprintln(format!("[install perf] {}", line.unwrap()));
                    }
                    
                    perf_install_channel.wait_eof().await?;
                    let exit_code = perf_install_channel.exit_status()?;
                    perf_install_channel.wait_close().await?;
                    if exit_code != 0 {
                        anyhow::bail!("Failed to install perf on remote host");
                    }

                    // Attach perf to the command
                    command = format!(
                        "perf record -F {frequency} -e cycles:u --call-graph dwarf,65528 -o {PERF_OUTFILE} {command}",
                    );
                }
                channel.exec(&command).await?;
                anyhow::Ok(channel)
            }
        )
        .await?;

        let (stdin_sender, mut stdin_receiver) = mpsc::unbounded_channel::<String>();
        let mut stdin = channel.stream(0); // stream 0 is stdout/stdin, we use it for stdin
        tokio::spawn(async move {
            while let Some(line) = stdin_receiver.recv().await {
                if stdin.write_all(line.as_bytes()).await.is_err() {
                    break;
                }

                stdin.flush().await.unwrap();
            }
        });

        let id_clone = id.clone();
        let (stdout_deploy_receivers, stdout_receivers) =
            prioritized_broadcast(FuturesBufReader::new(channel.stream(0)).lines(), move |s| {
                ProgressTracker::println(format!("[{id_clone}] {s}"));
            });
        let (_, stderr_receivers) =
            prioritized_broadcast(FuturesBufReader::new(channel.stderr()).lines(), move |s| {
                ProgressTracker::println(format!("[{id} stderr] {s}"));
            });

        Ok(Box::new(LaunchedSshBinary {
            _resource_result: self.resource_result().clone(),
            session: Some(session),
            channel,
            stdin_sender,
            stdout_deploy_receivers,
            stdout_receivers,
            stderr_receivers,
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
                    .channel_direct_tcpip(&internal_ip, port, None)
                    .await
                    .unwrap();
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
