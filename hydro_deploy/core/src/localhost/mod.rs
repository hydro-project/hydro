use std::net::SocketAddr;
use std::sync::{Arc, OnceLock};

use anyhow::{Result, bail};
use async_process::{Command, Stdio};
use async_trait::async_trait;
use hydro_deploy_integration::ServerBindConfig;

use crate::progress::ProgressTracker;
use crate::rust_crate::build::BuildOutput;
use crate::rust_crate::tracing_options::TracingOptions;
use crate::{
    BaseServerStrategy, ClientStrategy, Host, HostStrategyGetter, HostTargetType, LaunchedBinary,
    LaunchedHost, PortNetworkHint, ResourceBatch, ResourceResult,
};

pub mod launched_binary;
pub use launched_binary::*;

#[cfg(feature = "tracing")]
#[cfg(any(target_os = "macos", target_family = "windows"))]
mod samply;

static LOCAL_LIBDIR: OnceLock<String> = OnceLock::new();

#[derive(Debug)]
pub struct LocalhostHost {
    pub id: usize,
    client_only: bool,
}

impl LocalhostHost {
    pub fn new(id: usize) -> LocalhostHost {
        LocalhostHost {
            id,
            client_only: false,
        }
    }

    pub fn client_only(&self) -> LocalhostHost {
        LocalhostHost {
            id: self.id,
            client_only: true,
        }
    }
}

impl Host for LocalhostHost {
    fn target_type(&self) -> HostTargetType {
        HostTargetType::Local
    }

    fn request_port_base(&self, _bind_type: &BaseServerStrategy) {}
    fn collect_resources(&self, _resource_batch: &mut ResourceBatch) {}
    fn request_custom_binary(&self) {}

    fn id(&self) -> usize {
        self.id
    }

    fn launched(&self) -> Option<Arc<dyn LaunchedHost>> {
        Some(Arc::new(LaunchedLocalhost))
    }

    fn provision(&self, _resource_result: &Arc<ResourceResult>) -> Arc<dyn LaunchedHost> {
        Arc::new(LaunchedLocalhost)
    }

    fn strategy_as_server<'a>(
        &'a self,
        connection_from: &dyn Host,
        network_hint: PortNetworkHint,
    ) -> Result<(ClientStrategy<'a>, HostStrategyGetter)> {
        if self.client_only {
            anyhow::bail!("Localhost cannot be a server if it is client only")
        }

        if matches!(network_hint, PortNetworkHint::Auto)
            && connection_from.can_connect_to(ClientStrategy::UnixSocket(self.id))
        {
            Ok((
                ClientStrategy::UnixSocket(self.id),
                Box::new(|_| BaseServerStrategy::UnixSocket),
            ))
        } else if matches!(
            network_hint,
            PortNetworkHint::Auto | PortNetworkHint::TcpPort(_)
        ) && connection_from.can_connect_to(ClientStrategy::InternalTcpPort(self))
        {
            Ok((
                ClientStrategy::InternalTcpPort(self),
                Box::new(move |_| {
                    BaseServerStrategy::InternalTcpPort(match network_hint {
                        PortNetworkHint::Auto => None,
                        PortNetworkHint::TcpPort(port) => port,
                    })
                }),
            ))
        } else {
            anyhow::bail!("Could not find a strategy to connect to localhost")
        }
    }

    fn can_connect_to(&self, typ: ClientStrategy) -> bool {
        match typ {
            ClientStrategy::UnixSocket(id) => {
                #[cfg(unix)]
                {
                    self.id == id
                }

                #[cfg(not(unix))]
                {
                    let _ = id;
                    false
                }
            }
            ClientStrategy::InternalTcpPort(target_host) => self.id == target_host.id(),
            ClientStrategy::ForwardedTcpPort(_) => true,
        }
    }
}

struct LaunchedLocalhost;

#[async_trait]
impl LaunchedHost for LaunchedLocalhost {
    fn base_server_config(&self, bind_type: &BaseServerStrategy) -> ServerBindConfig {
        match bind_type {
            BaseServerStrategy::UnixSocket => ServerBindConfig::UnixSocket,
            BaseServerStrategy::InternalTcpPort(port) => {
                ServerBindConfig::TcpPort("127.0.0.1".to_string(), *port)
            }
            BaseServerStrategy::ExternalTcpPort(_) => panic!("Cannot bind to external port"),
        }
    }

    async fn copy_binary(&self, _binary: &BuildOutput) -> Result<()> {
        Ok(())
    }

    async fn launch_binary(
        &self,
        id: String,
        binary: &BuildOutput,
        args: &[String],
        tracing: Option<TracingOptions>,
    ) -> Result<Box<dyn LaunchedBinary>> {
        let (maybe_perf_outfile, mut command) = if let Some(tracing) = tracing.as_ref() {
            if cfg!(any(target_os = "macos", target_family = "windows")) {
                // samply
                ProgressTracker::println(
                    format!("[{id} tracing] Profiling binary with `samply`.",),
                );
                let samply_outfile = tempfile::NamedTempFile::new()?;

                let mut command = Command::new("samply");
                command
                    .arg("record")
                    .arg("--save-only")
                    .arg("--output")
                    .arg(samply_outfile.as_ref())
                    .arg(&binary.bin_path)
                    .args(args);
                (Some(samply_outfile), command)
            } else if cfg!(target_family = "unix") {
                // perf
                ProgressTracker::println(format!("[{} tracing] Tracing binary with `perf`.", id));
                let perf_outfile = tempfile::NamedTempFile::new()?;

                let mut command = Command::new("perf");
                command
                    .args([
                        "record",
                        "-F",
                        &tracing.frequency.to_string(),
                        "-e",
                        "cycles:u",
                        "--call-graph",
                        "dwarf,65528",
                        "-o",
                    ])
                    .arg(perf_outfile.as_ref())
                    .arg(&binary.bin_path)
                    .args(args);

                (Some(perf_outfile), command)
            } else {
                bail!(
                    "Unknown OS for samply/perf tracing: {}",
                    std::env::consts::OS
                );
            }
        } else {
            let mut command = Command::new(&binary.bin_path);
            command.args(args);
            (None, command)
        };

        // from cargo (https://github.com/rust-lang/cargo/blob/master/crates/cargo-util/src/paths.rs#L38)
        let dylib_path_var = if cfg!(windows) {
            "PATH"
        } else if cfg!(target_os = "macos") {
            "DYLD_FALLBACK_LIBRARY_PATH"
        } else if cfg!(target_os = "aix") {
            "LIBPATH"
        } else {
            "LD_LIBRARY_PATH"
        };

        let local_libdir = LOCAL_LIBDIR.get_or_init(|| {
            std::process::Command::new("rustc")
                .arg("--print")
                .arg("target-libdir")
                .output()
                .map(|output| String::from_utf8(output.stdout).unwrap().trim().to_string())
                .unwrap()
        });

        command.env(
            dylib_path_var,
            std::env::var_os(dylib_path_var).map_or_else(
                || {
                    std::env::join_paths(
                        [
                            binary.shared_library_path.as_ref(),
                            Some(&std::path::PathBuf::from(local_libdir)),
                        ]
                        .into_iter()
                        .flatten(),
                    )
                    .unwrap()
                },
                |paths| {
                    let mut paths = std::env::split_paths(&paths).collect::<Vec<_>>();
                    paths.insert(0, std::path::PathBuf::from(local_libdir));
                    if let Some(shared_path) = &binary.shared_library_path {
                        paths.insert(0, shared_path.to_path_buf());
                    }
                    std::env::join_paths(paths).unwrap()
                },
            ),
        );

        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(not(target_family = "unix"))]
        command.kill_on_drop(true);

        ProgressTracker::println(format!("[{}] running command: `{:?}`", id, command));

        let child = command.spawn().map_err(|e| {
            let msg = if maybe_perf_outfile.is_some() && std::io::ErrorKind::NotFound == e.kind() {
                "Tracing executable not found, ensure it is installed"
            } else {
                "Failed to execute command"
            };
            anyhow::Error::new(e).context(format!("{}: {:?}", msg, command))
        })?;

        Ok(Box::new(LaunchedLocalhostBinary::new(
            child,
            id,
            tracing,
            maybe_perf_outfile.map(|f| TracingDataLocal { outfile: f }),
        )))
    }

    async fn forward_port(&self, addr: &SocketAddr) -> Result<SocketAddr> {
        Ok(*addr)
    }
}
