use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Result;
use async_channel::{Receiver, Sender};

use async_ssh2_lite::{AsyncChannel, AsyncSession, SessionConfiguration};
use async_trait::async_trait;
use futures::{AsyncWriteExt, Future, StreamExt};
use hydroflow::util::connection::BindType;
use serde_json::json;
use tokio::{net::TcpStream, sync::RwLock};

use super::{
    localhost::create_broadcast,
    terraform::{TerraformOutput, TerraformProvider},
    ConnectionType, Host, LaunchedBinary, LaunchedHost, ResourceBatch, ResourceResult,
};

struct LaunchedComputeEngineBinary {
    _resource_result: Arc<ResourceResult>,
    channel: AsyncChannel<TcpStream>,
    stdin_channel: Sender<String>,
    stdout_receivers: Arc<RwLock<Vec<Sender<String>>>>,
    stderr_receivers: Arc<RwLock<Vec<Sender<String>>>>,
}

#[async_trait]
impl LaunchedBinary for LaunchedComputeEngineBinary {
    async fn stdin(&self) -> Sender<String> {
        self.stdin_channel.clone()
    }

    async fn stdout(&self) -> Receiver<String> {
        let mut receivers = self.stdout_receivers.write().await;
        let (sender, receiver) = async_channel::unbounded::<String>();
        receivers.push(sender);
        receiver
    }

    async fn stderr(&self) -> Receiver<String> {
        let mut receivers = self.stderr_receivers.write().await;
        let (sender, receiver) = async_channel::unbounded::<String>();
        receivers.push(sender);
        receiver
    }

    async fn exit_code(&self) -> Option<i32> {
        if self.channel.eof() {
            self.channel.exit_status().ok()
        } else {
            None
        }
    }
}

struct LaunchedComputeEngine {
    resource_result: Arc<ResourceResult>,
    external_ip: String,
    binary_counter: RwLock<usize>,
}

async fn async_retry<T, F: Future<Output = Result<T>>>(
    thunk: impl Fn() -> F,
    count: usize,
) -> Result<T> {
    for _ in 1..count {
        let result = thunk().await;
        if result.is_ok() {
            return result;
        } else {
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    }

    thunk().await
}

#[async_trait]
impl LaunchedHost for LaunchedComputeEngine {
    async fn launch_binary(&self, binary: &Path) -> Result<Arc<RwLock<dyn LaunchedBinary>>> {
        let session = async_retry(
            || async {
                let mut config = SessionConfiguration::new();
                config.set_timeout(5000);

                let mut session = AsyncSession::<TcpStream>::connect(
                    SocketAddr::new(self.external_ip.parse().unwrap(), 22),
                    Some(config),
                )
                .await?;

                session.handshake().await?;

                session
                    .userauth_pubkey_file(
                        "hydro",
                        None,
                        self.resource_result
                            .terraform
                            .deployment_folder
                            .path()
                            .join(".ssh")
                            .join("vm_instance_ssh_key_pem")
                            .as_path(),
                        None,
                    )
                    .await?;

                Ok(session)
            },
            10,
        )
        .await?;

        let sftp = session.sftp().await?;

        let mut binary_counter_write = self.binary_counter.write().await;
        let my_binary_counter = *binary_counter_write;
        *binary_counter_write += 1;
        drop(binary_counter_write);

        let binary_path = PathBuf::from(format!("/home/hydro/hydro-{my_binary_counter}"));

        let mut created_file = sftp.create(&binary_path).await?;
        created_file
            .write_all(std::fs::read(binary).unwrap().as_slice())
            .await?;

        let mut orig_file_stat = sftp.stat(&binary_path).await?;
        orig_file_stat.perm = Some(0o755);
        created_file.setstat(orig_file_stat).await?;
        created_file.close().await?;
        drop(created_file);

        let mut channel = session.channel_session().await?;
        channel.exec(binary_path.to_str().unwrap()).await?;

        let (stdin_sender, mut stdin_receiver) = async_channel::unbounded::<String>();
        let mut stdin = channel.stream(0);
        tokio::spawn(async move {
            while let Some(line) = stdin_receiver.next().await {
                if stdin.write_all(line.as_bytes()).await.is_err() {
                    break;
                }
            }
        });

        let stdout_receivers = create_broadcast(channel.stream(0), |s| println!("{s}"));
        let stderr_receivers = create_broadcast(channel.stderr(), |s| eprintln!("{s}"));

        Ok(Arc::new(RwLock::new(LaunchedComputeEngineBinary {
            _resource_result: self.resource_result.clone(),
            channel,
            stdin_channel: stdin_sender,
            stdout_receivers,
            stderr_receivers,
        })))
    }
}

pub struct GCPComputeEngineHost {
    pub id: usize,
    pub project: String,
    pub machine_type: String,
    pub region: String,
    pub internal_ip: Option<String>,
    pub external_ip: Option<String>,
    pub launched: Option<Arc<dyn LaunchedHost>>,
}

impl GCPComputeEngineHost {
    pub fn new(id: usize, project: String, machine_type: String, region: String) -> Self {
        Self {
            id,
            project,
            machine_type,
            region,
            internal_ip: None,
            external_ip: None,
            launched: None,
        }
    }
}

#[async_trait]
impl Host for GCPComputeEngineHost {
    fn collect_resources(&self, resource_batch: &mut ResourceBatch) {
        let project = self.project.as_str();
        let id = self.id;

        resource_batch
            .terraform
            .terraform
            .required_providers
            .insert(
                "google".to_string(),
                TerraformProvider {
                    source: "hashicorp/google".to_string(),
                    version: "4.53.1".to_string(),
                },
            );

        resource_batch
            .terraform
            .terraform
            .required_providers
            .insert(
                "local".to_string(),
                TerraformProvider {
                    source: "hashicorp/local".to_string(),
                    version: "2.3.0".to_string(),
                },
            );

        resource_batch
            .terraform
            .terraform
            .required_providers
            .insert(
                "tls".to_string(),
                TerraformProvider {
                    source: "hashicorp/tls".to_string(),
                    version: "4.0.4".to_string(),
                },
            );

        resource_batch
            .terraform
            .resource
            .entry("tls_private_key".to_string())
            .or_default()
            .insert(
                "vm_instance_ssh_key".to_string(),
                json!({
                    "algorithm": "RSA",
                    "rsa_bits": 4096
                }),
            );

        resource_batch
            .terraform
            .resource
            .entry("local_file".to_string())
            .or_default()
            .insert(
                "vm_instance_ssh_key_pem".to_string(),
                json!({
                    "content": "${tls_private_key.vm_instance_ssh_key.private_key_pem}",
                    "filename": ".ssh/vm_instance_ssh_key_pem",
                    "file_permission": "0600"
                }),
            );

        let vpc_network = format!("vpc-network-{project}");
        resource_batch
            .terraform
            .resource
            .entry("google_compute_network".to_string())
            .or_default()
            .insert(
                vpc_network.clone(),
                json!({
                    "name": vpc_network,
                    "project": project,
                    "auto_create_subnetworks": true
                }),
            );

        let firewall_entries = resource_batch
            .terraform
            .resource
            .entry("google_compute_firewall".to_string())
            .or_default();

        firewall_entries.insert(
            format!("{vpc_network}-default-allow-internal"),
            json!({
                "name": format!("{vpc_network}-default-allow-internal"),
                "project": project,
                "network": format!("${{google_compute_network.{vpc_network}.name}}"),
                "source_ranges": ["10.128.0.0/9"],
                "allow": [
                    {
                        "protocol": "tcp",
                        "ports": ["0-65535"]
                    },
                    {
                        "protocol": "udp",
                        "ports": ["0-65535"]
                    },
                    {
                        "protocol": "icmp"
                    }
                ]
            }),
        );

        firewall_entries.insert(
            format!("{vpc_network}-default-allow-ping"),
            json!({
                "name": format!("{vpc_network}-default-allow-ping"),
                "project": project,
                "network": format!("${{google_compute_network.{vpc_network}.name}}"),
                "source_ranges": ["0.0.0.0/0"],
                "allow": [
                    {
                        "protocol": "icmp"
                    }
                ]
            }),
        );

        let allow_ssh_rule = format!("{vpc_network}-allow-ssh");
        firewall_entries.insert(
            allow_ssh_rule.clone(),
            json!({
                "name": allow_ssh_rule,
                "project": project,
                "network": format!("${{google_compute_network.{vpc_network}.name}}"),
                "target_tags": [allow_ssh_rule],
                "source_ranges": ["0.0.0.0/0"],
                "allow": [
                    {
                        "protocol": "tcp",
                        "ports": ["22"]
                    }
                ]
            }),
        );

        let vm_instance = format!("vm-instance-{project}-{id}");
        resource_batch.terraform.resource.entry("google_compute_instance".to_string())
            .or_default()
            .insert(vm_instance.clone(), json!({
                "name": vm_instance,
                "project": project,
                "machine_type": self.machine_type,
                "zone": self.region,
                "tags": [ allow_ssh_rule ],
                "metadata": {
                "ssh-keys": "hydro:${tls_private_key.vm_instance_ssh_key.public_key_openssh}"
                },
                "boot_disk": [
                    {
                        "initialize_params": [
                            {
                                "image": "debian-cloud/debian-11"
                            }
                        ]
                    }
                ],
                "network_interface": [
                    {
                        "network": format!("${{google_compute_network.{vpc_network}.self_link}}"),
                        "access_config": [
                            {
                                "network_tier": "STANDARD"
                            }
                        ]
                    }
                ]
            }));

        resource_batch.terraform.output.insert(
            format!("{vm_instance}-internal-ip"),
            TerraformOutput {
                value: format!(
                    "${{google_compute_instance.{vm_instance}.network_interface[0].network_ip}}"
                ),
            },
        );

        resource_batch.terraform.output.insert(
            format!("{vm_instance}-public-ip"),
            TerraformOutput {
                value: format!("${{google_compute_instance.{vm_instance}.network_interface[0].access_config[0].nat_ip}}")
            }
        );
    }

    async fn provision(&mut self, resource_result: &Arc<ResourceResult>) -> Arc<dyn LaunchedHost> {
        if self.launched.is_none() {
            let project = self.project.as_str();
            let id = self.id;

            let internal_ip = &resource_result
                .terraform
                .outputs
                .get(&format!("vm-instance-{project}-{id}-internal-ip"))
                .unwrap()
                .value;
            self.internal_ip = Some(internal_ip.clone());

            let external_ip = &resource_result
                .terraform
                .outputs
                .get(&format!("vm-instance-{project}-{id}-public-ip"))
                .unwrap()
                .value;
            self.external_ip = Some(external_ip.clone());

            self.launched = Some(Arc::new(LaunchedComputeEngine {
                resource_result: resource_result.clone(),
                external_ip: external_ip.clone(),
                binary_counter: RwLock::new(0),
            }))
        }

        self.launched.as_ref().unwrap().clone()
    }

    fn find_bind_type(&self, connection_from: &dyn Host) -> BindType {
        if connection_from.can_connect_to(ConnectionType::UnixSocket(self.id)) {
            BindType::UnixSocket
        } else if connection_from
            .can_connect_to(ConnectionType::InternalTcpPort(self.project.clone()))
        {
            BindType::TcpPort(self.internal_ip.as_ref().unwrap().clone())
        } else {
            todo!()
        }
    }

    fn can_connect_to(&self, typ: ConnectionType) -> bool {
        match typ {
            ConnectionType::UnixSocket(id) => {
                #[cfg(unix)]
                {
                    self.id == id
                }

                #[cfg(not(unix))]
                {
                    false
                }
            }
            ConnectionType::InternalTcpPort(id) => self.project == id,
        }
    }
}
