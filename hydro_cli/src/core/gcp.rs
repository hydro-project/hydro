use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use async_ssh2_lite::{AsyncSession, SessionConfiguration};
use async_trait::async_trait;
use hydroflow_cli_integration::ServerBindConfig;
use nanoid::nanoid;
use serde_json::json;
use tokio::net::TcpStream;
use tokio::sync::RwLock;

use super::progress::ProgressTracker;
use super::ssh::LaunchedSSHHost;
use super::terraform::{TerraformOutput, TerraformProvider, TERRAFORM_ALPHABET};
use super::util::async_retry;
use super::{
    ClientStrategy, Host, HostTargetType, LaunchedHost, ResourceBatch, ResourceResult,
    ServerStrategy,
};

// #[derive(Debug)]
pub struct LaunchedComputeEngine {
    resource_result: Arc<ResourceResult>,
    user: String,
    pub internal_ip: String,
    pub external_ip: Option<String>,
}

impl LaunchedComputeEngine {
    pub fn ssh_key_path(&self) -> PathBuf {
        self.resource_result
            .terraform
            .deployment_folder
            .as_ref()
            .unwrap()
            .path()
            .join(".ssh")
            .join("vm_instance_ssh_key_pem")
    }
}

#[async_trait]
impl LaunchedSSHHost for LaunchedComputeEngine {
    fn server_config(&self, bind_type: &ServerStrategy) -> ServerBindConfig {
        match bind_type {
            ServerStrategy::UnixSocket => ServerBindConfig::UnixSocket,
            ServerStrategy::InternalTcpPort => ServerBindConfig::TcpPort(self.internal_ip.clone()),
            ServerStrategy::ExternalTcpPort(_) => todo!(),
            ServerStrategy::Demux(demux) => {
                let mut config_map = HashMap::new();
                for (key, underlying) in demux {
                    config_map.insert(*key, LaunchedSSHHost::server_config(self, underlying));
                }

                ServerBindConfig::Demux(config_map)
            }
            ServerStrategy::Merge(merge) => {
                let mut configs = vec![];
                for underlying in merge {
                    configs.push(LaunchedSSHHost::server_config(self, underlying));
                }

                ServerBindConfig::Merge(configs)
            }
            ServerStrategy::Tagged(underlying, id) => ServerBindConfig::Tagged(
                Box::new(LaunchedSSHHost::server_config(self, underlying)),
                *id,
            ),
            ServerStrategy::Null => ServerBindConfig::Null,
        }
    }

    fn resource_result(&self) -> &Arc<ResourceResult> {
        &self.resource_result
    }

    fn ssh_user(&self) -> &str {
        self.user.as_str()
    }

    async fn open_ssh_session(&self) -> Result<AsyncSession<TcpStream>> {
        let target_addr = SocketAddr::new(
            self.external_ip
                .as_ref()
                .context("GCP host must be configured with an external IP to launch binaries")?
                .parse()
                .unwrap(),
            22,
        );

        let res = ProgressTracker::leaf(
            format!(
                "connecting to host @ {}",
                self.external_ip.as_ref().unwrap()
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
                            self.user.as_str(),
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

#[derive(Debug)]
pub struct GCPNetwork {
    pub project: String,
    pub existing_vpc: Option<String>,
    id: String,
}

impl GCPNetwork {
    pub fn new(project: String, existing_vpc: Option<String>) -> Self {
        Self {
            project,
            existing_vpc,
            id: nanoid!(8, &TERRAFORM_ALPHABET),
        }
    }

    fn collect_resources(&mut self, resource_batch: &mut ResourceBatch) -> String {
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

        let vpc_network = format!("hydro-vpc-network-{}", self.id);

        if let Some(existing) = self.existing_vpc.as_ref() {
            if resource_batch
                .terraform
                .resource
                .get(&"google_compute_network".to_string())
                .unwrap_or(&HashMap::new())
                .contains_key(existing)
            {
                format!("google_compute_network.{existing}")
            } else {
                resource_batch
                    .terraform
                    .data
                    .entry("google_compute_network".to_string())
                    .or_default()
                    .insert(
                        vpc_network.clone(),
                        json!({
                            "name": existing,
                            "project": self.project,
                        }),
                    );

                format!("data.google_compute_network.{vpc_network}")
            }
        } else {
            resource_batch
                .terraform
                .resource
                .entry("google_compute_network".to_string())
                .or_default()
                .insert(
                    vpc_network.clone(),
                    json!({
                        "name": vpc_network,
                        "project": self.project,
                        "auto_create_subnetworks": true
                    }),
                );

            let firewall_entries = resource_batch
                .terraform
                .resource
                .entry("google_compute_firewall".to_string())
                .or_default();

            // allow all VMs to communicate with each other over internal IPs
            firewall_entries.insert(
                format!("{vpc_network}-default-allow-internal"),
                json!({
                    "name": format!("{vpc_network}-default-allow-internal"),
                    "project": self.project,
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

            // allow external pings to all VMs
            firewall_entries.insert(
                format!("{vpc_network}-default-allow-ping"),
                json!({
                    "name": format!("{vpc_network}-default-allow-ping"),
                    "project": self.project,
                    "network": format!("${{google_compute_network.{vpc_network}.name}}"),
                    "source_ranges": ["0.0.0.0/0"],
                    "allow": [
                        {
                            "protocol": "icmp"
                        }
                    ]
                }),
            );

            self.existing_vpc = Some(vpc_network.clone());

            format!("google_compute_network.{vpc_network}")
        }
    }
}

pub struct GCPComputeEngineHost {
    pub id: usize,
    pub project: String,
    pub machine_type: String,
    pub image: String,
    pub region: String,
    pub network: Arc<RwLock<GCPNetwork>>,
    pub user: Option<String>,
    pub launched: Option<Arc<LaunchedComputeEngine>>,
    external_ports: Vec<u16>,
}

impl GCPComputeEngineHost {
    pub fn new(
        id: usize,
        project: String,
        machine_type: String,
        image: String,
        region: String,
        network: Arc<RwLock<GCPNetwork>>,
        user: Option<String>,
    ) -> Self {
        Self {
            id,
            project,
            machine_type,
            image,
            region,
            network,
            user,
            launched: None,
            external_ports: vec![],
        }
    }
}

#[async_trait]
impl Host for GCPComputeEngineHost {
    fn target_type(&self) -> HostTargetType {
        HostTargetType::Linux
    }

    fn request_port(&mut self, bind_type: &ServerStrategy) {
        match bind_type {
            ServerStrategy::UnixSocket => {}
            ServerStrategy::InternalTcpPort => {}
            ServerStrategy::ExternalTcpPort(port) => {
                if !self.external_ports.contains(port) {
                    if self.launched.is_some() {
                        todo!("Cannot adjust firewall after host has been launched");
                    }

                    self.external_ports.push(*port);
                }
            }
            ServerStrategy::Demux(demux) => {
                for bind_type in demux.values() {
                    self.request_port(bind_type);
                }
            }
            ServerStrategy::Merge(merge) => {
                for bind_type in merge {
                    self.request_port(bind_type);
                }
            }
            ServerStrategy::Tagged(underlying, _) => {
                self.request_port(underlying);
            }
            ServerStrategy::Null => {}
        }
    }

    fn request_custom_binary(&mut self) {
        self.request_port(&ServerStrategy::ExternalTcpPort(22));
    }

    fn id(&self) -> usize {
        self.id
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn collect_resources(&self, resource_batch: &mut ResourceBatch) {
        if self.launched.is_some() {
            return;
        }

        let vpc_path = self
            .network
            .try_write()
            .unwrap()
            .collect_resources(resource_batch);

        let project = self.project.as_str();

        // first, we import the providers we need
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

        // we use a single SSH key for all VMs
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

        let vm_key = format!("vm-instance-{}", self.id);
        let vm_name = format!("hydro-vm-instance-{}", nanoid!(8, &TERRAFORM_ALPHABET));

        let mut tags = vec![];
        let mut external_interfaces = vec![];

        if self.external_ports.is_empty() {
            external_interfaces.push(json!({ "network": format!("${{{vpc_path}.self_link}}") }));
        } else {
            external_interfaces.push(json!({
                "network": format!("${{{vpc_path}.self_link}}"),
                "access_config": [
                    {
                        "network_tier": "STANDARD"
                    }
                ]
            }));

            // open the external ports that were requested
            let my_external_tags = self.external_ports.iter().map(|port| {
                let rule_id = nanoid!(8, &TERRAFORM_ALPHABET);
                let firewall_rule = resource_batch
                    .terraform
                    .resource
                    .entry("google_compute_firewall".to_string())
                    .or_default()
                    .entry(format!("open-external-port-{}", port))
                    .or_insert(json!({
                        "name": format!("open-external-port-{}-{}", port, rule_id),
                        "project": project,
                        "network": format!("${{{vpc_path}.name}}"),
                        "target_tags": [format!("open-external-port-tag-{}-{}", port, rule_id)],
                        "source_ranges": ["0.0.0.0/0"],
                        "allow": [
                            {
                                "protocol": "tcp",
                                "ports": vec![port.to_string()]
                            }
                        ]
                    }));

                firewall_rule["target_tags"].as_array().unwrap()[0].clone()
            });

            tags.extend(my_external_tags);

            resource_batch.terraform.output.insert(
                format!("{vm_key}-public-ip"),
                TerraformOutput {
                    value: format!("${{google_compute_instance.{vm_key}.network_interface[0].access_config[0].nat_ip}}")
                }
            );
        }

        let user = self.user.as_ref().cloned().unwrap_or("hydro".to_string());
        resource_batch
            .terraform
            .resource
            .entry("google_compute_instance".to_string())
            .or_default()
            .insert(
                vm_key.clone(),
                json!({
                    "name": vm_name,
                    "project": project,
                    "machine_type": self.machine_type,
                    "zone": self.region,
                    "tags": tags,
                    "metadata": {
                        "ssh-keys": format!("{user}:${{tls_private_key.vm_instance_ssh_key.public_key_openssh}}")
                    },
                    "boot_disk": [
                        {
                            "initialize_params": [
                                {
                                    "image": self.image
                                }
                            ]
                        }
                    ],
                    "network_interface": external_interfaces
                }),
            );

        resource_batch.terraform.output.insert(
            format!("{vm_key}-internal-ip"),
            TerraformOutput {
                value: format!(
                    "${{google_compute_instance.{vm_key}.network_interface[0].network_ip}}"
                ),
            },
        );
    }

    fn launched(&self) -> Option<Arc<dyn LaunchedHost>> {
        self.launched
            .as_ref()
            .map(|a| a.clone() as Arc<dyn LaunchedHost>)
    }

    async fn provision(&mut self, resource_result: &Arc<ResourceResult>) -> Arc<dyn LaunchedHost> {
        if self.launched.is_none() {
            let id = self.id;

            let internal_ip = resource_result
                .terraform
                .outputs
                .get(&format!("vm-instance-{id}-internal-ip"))
                .unwrap()
                .value
                .clone();

            let external_ip = resource_result
                .terraform
                .outputs
                .get(&format!("vm-instance-{id}-public-ip"))
                .map(|v| v.value.clone());

            self.launched = Some(Arc::new(LaunchedComputeEngine {
                resource_result: resource_result.clone(),
                user: self.user.as_ref().cloned().unwrap_or("hydro".to_string()),
                internal_ip,
                external_ip,
            }))
        }

        self.launched.as_ref().unwrap().clone()
    }

    fn strategy_as_server<'a>(
        &'a self,
        client_host: &dyn Host,
    ) -> Result<(
        ClientStrategy<'a>,
        Box<dyn FnOnce(&mut dyn std::any::Any) -> ServerStrategy>,
    )> {
        if client_host.can_connect_to(ClientStrategy::UnixSocket(self.id)) {
            Ok((
                ClientStrategy::UnixSocket(self.id),
                Box::new(|_| ServerStrategy::UnixSocket),
            ))
        } else if client_host.can_connect_to(ClientStrategy::InternalTcpPort(self)) {
            Ok((
                ClientStrategy::InternalTcpPort(self),
                Box::new(|_| ServerStrategy::InternalTcpPort),
            ))
        } else if client_host.can_connect_to(ClientStrategy::ForwardedTcpPort(self)) {
            Ok((
                ClientStrategy::ForwardedTcpPort(self),
                Box::new(|me| {
                    me.downcast_mut::<GCPComputeEngineHost>()
                        .unwrap()
                        .request_port(&ServerStrategy::ExternalTcpPort(22)); // needed to forward
                    ServerStrategy::InternalTcpPort
                }),
            ))
        } else {
            anyhow::bail!("Could not find a strategy to connect to GCP instance")
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
            ClientStrategy::InternalTcpPort(target_host) => {
                if let Some(gcp_target) =
                    target_host.as_any().downcast_ref::<GCPComputeEngineHost>()
                {
                    self.project == gcp_target.project
                } else {
                    false
                }
            }
            ClientStrategy::ForwardedTcpPort(_) => false,
        }
    }
}
