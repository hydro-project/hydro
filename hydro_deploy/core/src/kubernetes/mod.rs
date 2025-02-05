use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, OnceLock};

use anyhow::Result;
use async_trait::async_trait;
use futures::{StreamExt, TryStreamExt};
use hydroflow_deploy_integration::ServerBindConfig;
use k8s_openapi::api::core::v1::Pod;
use kube::api::{Api, AttachParams, ListParams, PostParams, ResourceExt, WatchEvent, WatchParams};
use kube::Client;
use nanoid::nanoid;
use tokio::io::AsyncWriteExt;

use super::progress::ProgressTracker;
use super::{
    ClientStrategy, Host, HostTargetType, LaunchedBinary, LaunchedHost, ResourceBatch,
    ResourceResult, ServerStrategy,
};
use crate::hydroflow_crate::build::BuildOutput;
use crate::hydroflow_crate::tracing_options::TracingOptions;

pub mod launched_binary;
pub use launched_binary::*;

const ALPHABET: [char; 36] = [
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z', '1', '2', '3', '4', '5', '6', '7', '8', '9', '0',
];

#[derive(Debug)]
pub struct PodHost {
    pub id: usize,
    client_only: bool,
    pub launched: OnceLock<Arc<LaunchedPod>>,
}

impl PodHost {
    pub fn new(id: usize) -> PodHost {
        PodHost {
            id,
            client_only: false,
            launched: OnceLock::new(),
        }
    }

    pub fn client_only(&self) -> PodHost {
        PodHost {
            id: self.id,
            client_only: true,
            launched: OnceLock::new(),
        }
    }
}

#[async_trait]
impl Host for PodHost {
    fn target_type(&self) -> HostTargetType {
        HostTargetType::Linux(crate::LinuxArchitecture::AARCH64)
    }

    fn request_port(&self, _bind_type: &ServerStrategy) {}
    fn collect_resources(&self, _resource_batch: &mut ResourceBatch) {}
    fn request_custom_binary(&self) {}

    fn id(&self) -> usize {
        self.id
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn launched(&self) -> Option<Arc<dyn LaunchedHost>> {
        self.launched
            .get()
            .map(|a| a.clone() as Arc<dyn LaunchedHost>)
    }

    async fn provision(&self, _resource_result: &Arc<ResourceResult>) -> Arc<dyn LaunchedHost> {
        if self.launched.get().is_none() {
            let client = Client::try_default().await.unwrap();
            let pod_id = nanoid!(10, &ALPHABET); // randomly create a pod name, kubernetes restricts names to alphanumeric characters
            let pod_name = format!("hydro-{}", pod_id);

            // Blank template for a new pod
            let p: Pod = serde_json::from_value(serde_json::json!({
                "apiVersion": "v1",
                "kind": "Pod",
                "metadata": { "name": pod_name },
                "spec": {
                    "containers": [{
                        "name": pod_name,
                        "image": "ubuntu:20.04",
                        // Do nothing
                        "command": ["tail", "-f", "/dev/null"],
                    }],
                }
            }))
            .unwrap();

            let pods: Api<Pod> = Api::default_namespaced(client);

            // Check if pod_name has already been created. If not, create it.
            let lp = ListParams::default().fields(&format!("metadata.name={}", pod_name)); // only want results for our pod
            let mut found_existing_pod = false;
            match pods.list(&lp).await {
                Ok(pod_list) => {
                    for p in pod_list {
                        if p.name_any() == pod_name {
                            found_existing_pod = true;
                        }
                    }
                }
                Err(e) => {
                    ProgressTracker::println(
                        format!(
                            "Error listing pods: {:?}. Maybe your kubernetes cluster is not up?",
                            e
                        )
                        .as_str(),
                    );
                }
            }
            if !found_existing_pod {
                let res = pods.create(&PostParams::default(), &p).await;
                match res {
                    Err(e) => ProgressTracker::println(format!("{:?}", e).as_str()),
                    Ok(_) => (),
                }
            }

            // Wait until the pod is running, otherwise we get 500 error.
            let wp = WatchParams::default()
                .fields(format!("metadata.name={}", pod_name).as_str())
                .timeout(10);
            let mut stream = pods.watch(&wp, "0").await.unwrap().boxed();
            loop {
                let status_result = stream.try_next().await;
                match status_result {
                    Ok(Some(status)) => match status {
                        WatchEvent::Added(o) | WatchEvent::Modified(o) => {
                            let s = o.status.as_ref().expect("status exists on pod");
                            if s.phase.clone().unwrap_or_default() == "Running" {
                                break;
                            }
                        }
                        _ => {}
                    },
                    Ok(None) => {
                        // Pod still being created, likely -- restart the watch stream
                        stream = pods.watch(&wp, "0").await.unwrap().boxed();
                    }
                    Err(e) => {
                        ProgressTracker::println(&format!("Error watching pod events: {:?}", e));
                        break;
                    }
                }
            }

            // Use the internal IP for communication between pods
            let internal_ip = pods
                .get_status(pod_name.clone().as_str())
                .await
                .unwrap()
                .status
                .unwrap()
                .pod_ip
                .unwrap();

            self.launched
                .set(Arc::new(LaunchedPod {
                    internal_ip,
                    pod_name: pod_name.clone(),
                }))
                .unwrap();

            // We use lsof later in order to verify that the hydroflow binary has been written successfully to the pod.
            // First, update apt-get in the pod
            let ap = AttachParams::default()
                .stdin(false)
                .stdout(false)
                .stderr(true);
            let mut update_apt = pods
                .exec(
                    pod_name.clone().as_str(),
                    vec![&format!("apt-get"), "update"],
                    &ap,
                )
                .await
                .unwrap();
            let update_apt_status = update_apt.take_status().unwrap();

            // Verify that no errors occurred with command
            match update_apt_status.await {
                None => {
                    ProgressTracker::println("Warning: Command 'apt-get update' failed in pod");
                }
                _ => {}
            }

            // Install lsof in the pod to track open files
            let mut install_lsof = pods
                .exec(
                    pod_name.clone().as_str(),
                    vec![&format!("apt-get"), "install", "-y", "lsof"],
                    &ap,
                )
                .await
                .unwrap();
            let install_lsof_status = install_lsof.take_status().unwrap();

            match install_lsof_status.await {
                None => {
                    ProgressTracker::println(
                        "Warning: Command 'apt-get install -y lsof' failed in pod",
                    );
                }
                _ => {}
            }
        }

        self.launched.get().unwrap().clone()
    }

    fn strategy_as_server<'a>(
        &'a self,
        connection_from: &dyn Host,
    ) -> Result<(
        ClientStrategy<'a>,
        Box<dyn FnOnce(&dyn std::any::Any) -> ServerStrategy>,
    )> {
        if self.client_only {
            anyhow::bail!("Pod cannot be a server if it is client only")
        }

        if connection_from.can_connect_to(ClientStrategy::UnixSocket(self.id)) {
            Ok((
                ClientStrategy::UnixSocket(self.id),
                Box::new(|_| ServerStrategy::UnixSocket),
            ))
        } else if connection_from.can_connect_to(ClientStrategy::InternalTcpPort(self)) {
            Ok((
                ClientStrategy::InternalTcpPort(self),
                Box::new(|_| ServerStrategy::InternalTcpPort),
            ))
        } else {
            anyhow::bail!("Could not find a strategy to connect to Pod")
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
            ClientStrategy::InternalTcpPort(_target_host) => true, /* TODO: if I'm on the same cluster, can just return true first */
            ClientStrategy::ForwardedTcpPort(_) => true,
        }
    }
}

#[derive(Debug)]
pub struct LaunchedPod {
    pub internal_ip: String, // internal IP address for inter-pod communication in a node
    pub pod_name: String,
}

#[async_trait]
impl LaunchedHost for LaunchedPod {
    fn server_config(&self, bind_type: &ServerStrategy) -> ServerBindConfig {
        match bind_type {
            ServerStrategy::UnixSocket => ServerBindConfig::UnixSocket,
            ServerStrategy::InternalTcpPort => ServerBindConfig::TcpPort(self.internal_ip.clone()),
            ServerStrategy::ExternalTcpPort(_) => panic!("Cannot bind to external port"),
            ServerStrategy::Demux(demux) => {
                let mut config_map = HashMap::new();
                for (key, underlying) in demux {
                    config_map.insert(*key, self.server_config(underlying));
                }

                ServerBindConfig::Demux(config_map)
            }
            ServerStrategy::Merge(merge) => {
                let mut configs = vec![];
                for underlying in merge {
                    configs.push(self.server_config(underlying));
                }

                ServerBindConfig::Merge(configs)
            }
            ServerStrategy::Tagged(underlying, id) => {
                ServerBindConfig::Tagged(Box::new(self.server_config(underlying)), *id)
            }
            ServerStrategy::Null => ServerBindConfig::Null,
        }
    }

    async fn copy_binary(&self, binary: &BuildOutput) -> Result<()> {
        // Create a new pod in the running kubernetes cluster (we assume the user already has one up)
        let client = Client::try_default().await?;
        let pods: Api<Pod> = Api::default_namespaced(client);

        // Initialize the hydroflow binary with executable permissions
        let file_name = format!("hydro-{}-binary", binary.unique_id);
        let binary_data = binary.bin_data.clone();
        let mut header = tar::Header::new_gnu();
        header.set_path(file_name).unwrap();
        header.set_size(binary_data.len() as u64);
        header.set_mode(0o755); // give the binary executable permissions
        header.set_cksum();

        let mut ar = tar::Builder::new(Vec::new());
        ar.append(&header, &mut binary_data.as_slice()).unwrap();
        let data = ar.into_inner().unwrap();

        // Open up a tar extraction listening to stdin
        let ap = AttachParams::default().stdin(true).stderr(false);
        let mut tar = pods
            .exec(
                self.pod_name.as_str(),
                vec!["tar", "xf", "-", "-C", "/"],
                &ap,
            )
            .await?;
        let mut tar_stdin = tar.stdin().unwrap();

        // Write file contents through stdin
        if let Err(e) = tar_stdin.write_all(&data).await {
            ProgressTracker::println(&format!("Error writing to stdin: {:?}", e));
            return Err(e.into());
        }

        // Flush the stdin to finish sending the file through
        if let Err(e) = tar_stdin.flush().await {
            ProgressTracker::println(&format!("Error flushing stdin: {:?}", e));
            return Err(e.into());
        }

        // Shut down stdin to end writing process
        if let Err(e) = tar_stdin.shutdown().await {
            ProgressTracker::println(&format!("Error shutting down stdin: {:?}", e));
            return Err(e.into());
        }

        // Wait until the binary file has finished extracting and is executable. Why can we not just use tar.join() to wait until the tar
        // extraction has finished? That is because tar.join() only applies to the message loop that manages the streams (ex. stdin, stdout)
        // used for the underlying HTTP request which actually executes tar. When we close the stdin sender, tar.join() automatically returns, but the
        // underlying tar process may still be running for a bit longer. Thus, we check if our binary file is still open and only proceed when
        // it is not.
        loop {
            let ap_stdout = AttachParams::default().stdin(false).stderr(false);

            let mut lsof_file_process = pods
                .exec(
                    self.pod_name.as_str(),
                    vec!["lsof", &format!("./hydro-{}-binary", binary.unique_id)],
                    &ap_stdout,
                )
                .await?;

            let lsof_stdout =
                tokio_util::io::ReaderStream::new(lsof_file_process.stdout().unwrap());
            let lsof_out = lsof_stdout
                .filter_map(|r| async { r.ok().and_then(|v| String::from_utf8(v.to_vec()).ok()) })
                .collect::<Vec<_>>()
                .await
                .join("");

            if !lsof_out.is_empty() {
                // Binary is currently still open and being written to
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            } else {
                // Binary is not in use, ready to proceed
                break;
            }
        }

        Ok(())
    }

    async fn launch_binary(
        &self,
        id: String,
        binary: &BuildOutput,
        args: &[String],
        _perf: Option<TracingOptions>,
    ) -> Result<Box<dyn LaunchedBinary>> {

        let client = Client::try_default().await?;
        let pods: Api<Pod> = Api::default_namespaced(client);
        let pod_name = &self.pod_name;
        let file_name = format!("hydro-{}-binary", binary.unique_id);

        // Construct arguments
        let mut args_list = vec![format!("./{file_name}")];
        args_list.extend(args.iter().cloned());

        // Execute binary inside the new pod
        let ap = AttachParams::default()
            .stdin(true)
            .stdout(true)
            .stderr(true);

        // Execute binary inside the new pod
        let launch_binary = match pods.exec(pod_name, args_list, &ap).await {
            Ok(exec) => exec,
            Err(e) => {
                ProgressTracker::println(&format!("Failed to launch binary in Pod: {:?}", e));
                return Err(e.into());
            }
        };

        Ok(Box::new(LaunchedPodBinary::new(
            launch_binary,
            id,
            self.pod_name.clone(),
        )))
    }

    async fn forward_port(&self, addr: &SocketAddr) -> Result<SocketAddr> {
        Ok(*addr)
    }
}
