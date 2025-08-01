use std::any::Any;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, Mutex, OnceLock};

use anyhow::Result;
use async_trait::async_trait;
use nanoid::nanoid;
use serde_json::json;

use super::terraform::{TERRAFORM_ALPHABET, TerraformOutput, TerraformProvider};
use super::{ClientStrategy, Host, HostTargetType, LaunchedHost, ResourceBatch, ResourceResult};
use crate::ssh::LaunchedSshHost;
use crate::{BaseServerStrategy, HostStrategyGetter, PortNetworkHint};

pub struct LaunchedVirtualMachine {
    resource_result: Arc<ResourceResult>,
    user: String,
    pub internal_ip: String,
    pub external_ip: Option<String>,
}

impl LaunchedSshHost for LaunchedVirtualMachine {
    fn get_external_ip(&self) -> Option<String> {
        self.external_ip.clone()
    }

    fn get_internal_ip(&self) -> String {
        self.internal_ip.clone()
    }

    fn get_cloud_provider(&self) -> String {
        "Azure".to_string()
    }

    fn resource_result(&self) -> &Arc<ResourceResult> {
        &self.resource_result
    }

    fn ssh_user(&self) -> &str {
        self.user.as_str()
    }
}

pub struct AzureHost {
    /// ID from [`crate::Deployment::add_host`].
    id: usize,

    project: String,
    os_type: String, // linux or windows
    machine_size: String,
    image: Option<HashMap<String, String>>,
    region: String,
    user: Option<String>,
    pub launched: OnceLock<Arc<LaunchedVirtualMachine>>, // TODO(mingwei): fix pub
    external_ports: Mutex<Vec<u16>>,
}

impl AzureHost {
    pub fn new(
        id: usize,
        project: String,
        os_type: String, // linux or windows
        machine_size: String,
        image: Option<HashMap<String, String>>,
        region: String,
        user: Option<String>,
    ) -> Self {
        Self {
            id,
            project,
            os_type,
            machine_size,
            image,
            region,
            user,
            launched: OnceLock::new(),
            external_ports: Mutex::new(Vec::new()),
        }
    }
}

impl Debug for AzureHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("AzureHost({})", self.id))
    }
}

#[async_trait]
impl Host for AzureHost {
    fn target_type(&self) -> HostTargetType {
        HostTargetType::Linux
    }

    fn request_port_base(&self, bind_type: &BaseServerStrategy) {
        match bind_type {
            BaseServerStrategy::UnixSocket => {}
            BaseServerStrategy::InternalTcpPort(_) => {}
            BaseServerStrategy::ExternalTcpPort(port) => {
                let mut external_ports = self.external_ports.lock().unwrap();
                if !external_ports.contains(port) {
                    if self.launched.get().is_some() {
                        todo!("Cannot adjust firewall after host has been launched");
                    }
                    external_ports.push(*port);
                }
            }
        }
    }

    fn request_custom_binary(&self) {
        self.request_port_base(&BaseServerStrategy::ExternalTcpPort(22));
    }

    fn id(&self) -> usize {
        self.id
    }

    fn collect_resources(&self, resource_batch: &mut ResourceBatch) {
        if self.launched.get().is_some() {
            return;
        }

        let project = self.project.as_str();

        // first, we import the providers we need
        resource_batch
            .terraform
            .terraform
            .required_providers
            .insert(
                "azurerm".to_string(),
                TerraformProvider {
                    source: "hashicorp/azurerm".to_string(),
                    version: "3.67.0".to_string(),
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

        // Handle provider configuration
        resource_batch.terraform.provider.insert(
            "azurerm".to_string(),
            json!({
                "skip_provider_registration": "true",
                "features": {},
            }),
        );

        // Handle resources
        resource_batch
            .terraform
            .resource
            .entry("azurerm_resource_group".to_string())
            .or_default()
            .insert(
                vm_key.to_string(),
                json!({
                    "name": project,
                    "location": self.region.clone(),
                }),
            );

        resource_batch
            .terraform
            .resource
            .entry("azurerm_virtual_network".to_string())
            .or_default()
            .insert(
                vm_key.to_string(),
                json!({
                    "name": format!("{vm_key}-network"),
                    "address_space": ["10.0.0.0/16"],
                    "location": self.region.clone(),
                    "resource_group_name": format!("${{azurerm_resource_group.{vm_key}.name}}")
                }),
            );

        resource_batch
            .terraform
            .resource
            .entry("azurerm_subnet".to_string())
            .or_default()
            .insert(
                vm_key.to_string(),
                json!({
                    "name": "internal",
                    "resource_group_name": format!("${{azurerm_resource_group.{vm_key}.name}}"),
                    "virtual_network_name": format!("${{azurerm_virtual_network.{vm_key}.name}}"),
                    "address_prefixes": ["10.0.2.0/24"]
                }),
            );

        resource_batch
            .terraform
            .resource
            .entry("azurerm_public_ip".to_string())
            .or_default()
            .insert(
                vm_key.to_string(),
                json!({
                    "name": "hydropubip",
                    "resource_group_name": format!("${{azurerm_resource_group.{vm_key}.name}}"),
                    "location": format!("${{azurerm_resource_group.{vm_key}.location}}"),
                    "allocation_method": "Static",
                }),
            );

        resource_batch
            .terraform
            .resource
            .entry("azurerm_network_interface".to_string())
            .or_default()
            .insert(
                vm_key.to_string(),
                json!({
                    "name": format!("{vm_key}-nic"),
                    "location": format!("${{azurerm_resource_group.{vm_key}.location}}"),
                    "resource_group_name": format!("${{azurerm_resource_group.{vm_key}.name}}"),
                    "ip_configuration": {
                        "name": "internal",
                        "subnet_id": format!("${{azurerm_subnet.{vm_key}.id}}"),
                        "private_ip_address_allocation": "Dynamic",
                        "public_ip_address_id": format!("${{azurerm_public_ip.{vm_key}.id}}"),
                    }
                }),
            );

        // Define network security rules - for now, accept all connections
        resource_batch
            .terraform
            .resource
            .entry("azurerm_network_security_group".to_string())
            .or_default()
            .insert(
                vm_key.to_string(),
                json!({
                    "name": "primary_security_group",
                    "location": format!("${{azurerm_resource_group.{vm_key}.location}}"),
                    "resource_group_name": format!("${{azurerm_resource_group.{vm_key}.name}}"),
                }),
            );

        resource_batch
            .terraform
            .resource
            .entry("azurerm_network_security_rule".to_string())
            .or_default()
            .insert(
                vm_key.to_string(),
                json!({
                    "name": "allowall",
                    "priority": 100,
                    "direction": "Inbound",
                    "access": "Allow",
                    "protocol": "Tcp",
                    "source_port_range": "*",
                    "destination_port_range": "*",
                    "source_address_prefix": "*",
                    "destination_address_prefix": "*",
                    "resource_group_name": format!("${{azurerm_resource_group.{vm_key}.name}}"),
                    "network_security_group_name": format!("${{azurerm_network_security_group.{vm_key}.name}}"),
                })
            );

        resource_batch
            .terraform
            .resource
            .entry("azurerm_subnet_network_security_group_association".to_string())
            .or_default()
            .insert(
                vm_key.to_string(),
                json!({
                    "subnet_id": format!("${{azurerm_subnet.{vm_key}.id}}"),
                    "network_security_group_id": format!("${{azurerm_network_security_group.{vm_key}.id}}"),
                })
            );

        let user = self.user.as_ref().cloned().unwrap_or("hydro".to_string());
        let os_type = format!("azurerm_{}_virtual_machine", self.os_type.clone());
        let image = self.image.as_ref().cloned().unwrap_or(HashMap::from([
            ("publisher".to_string(), "Canonical".to_string()),
            (
                "offer".to_string(),
                "0001-com-ubuntu-server-jammy".to_string(),
            ),
            ("sku".to_string(), "22_04-lts".to_string()),
            ("version".to_string(), "latest".to_string()),
        ]));

        resource_batch
            .terraform
            .resource
            .entry(os_type.clone())
            .or_default()
            .insert(
                vm_key.clone(),
                json!({
                    "name": vm_name,
                    "resource_group_name": format!("${{azurerm_resource_group.{vm_key}.name}}"),
                    "location": format!("${{azurerm_resource_group.{vm_key}.location}}"),
                    "size": self.machine_size.clone(),
                    "network_interface_ids": [format!("${{azurerm_network_interface.{vm_key}.id}}")],
                    "admin_ssh_key": {
                        "username": user,
                        "public_key": "${tls_private_key.vm_instance_ssh_key.public_key_openssh}",
                    },
                    "admin_username": user,
                    "os_disk": {
                        "caching": "ReadWrite",
                        "storage_account_type": "Standard_LRS",
                    },
                    "source_image_reference": image,
                }),
            );

        resource_batch.terraform.output.insert(
            format!("{vm_key}-public-ip"),
            TerraformOutput {
                value: format!("${{azurerm_public_ip.{vm_key}.ip_address}}"),
            },
        );

        resource_batch.terraform.output.insert(
            format!("{vm_key}-internal-ip"),
            TerraformOutput {
                value: format!("${{azurerm_network_interface.{vm_key}.private_ip_address}}"),
            },
        );
    }

    fn launched(&self) -> Option<Arc<dyn LaunchedHost>> {
        self.launched
            .get()
            .map(|a| a.clone() as Arc<dyn LaunchedHost>)
    }

    fn provision(&self, resource_result: &Arc<ResourceResult>) -> Arc<dyn LaunchedHost> {
        self.launched
            .get_or_init(|| {
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

                Arc::new(LaunchedVirtualMachine {
                    resource_result: resource_result.clone(),
                    user: self.user.as_ref().cloned().unwrap_or("hydro".to_string()),
                    internal_ip,
                    external_ip,
                })
            })
            .clone()
    }

    fn strategy_as_server<'a>(
        &'a self,
        client_host: &dyn Host,
        network_hint: PortNetworkHint,
    ) -> Result<(ClientStrategy<'a>, HostStrategyGetter)> {
        if matches!(network_hint, PortNetworkHint::Auto)
            && client_host.can_connect_to(ClientStrategy::UnixSocket(self.id))
        {
            Ok((
                ClientStrategy::UnixSocket(self.id),
                Box::new(|_| BaseServerStrategy::UnixSocket),
            ))
        } else if matches!(
            network_hint,
            PortNetworkHint::Auto | PortNetworkHint::TcpPort(_)
        ) && client_host.can_connect_to(ClientStrategy::InternalTcpPort(self))
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
        } else if matches!(network_hint, PortNetworkHint::Auto)
            && client_host.can_connect_to(ClientStrategy::ForwardedTcpPort(self))
        {
            Ok((
                ClientStrategy::ForwardedTcpPort(self),
                Box::new(|me| {
                    me.downcast_ref::<AzureHost>()
                        .unwrap()
                        .request_port_base(&BaseServerStrategy::ExternalTcpPort(22)); // needed to forward
                    BaseServerStrategy::InternalTcpPort(None)
                }),
            ))
        } else {
            anyhow::bail!("Could not find a strategy to connect to Azure instance")
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
                if let Some(provider_target) = <dyn Any>::downcast_ref::<AzureHost>(target_host) {
                    self.project == provider_target.project
                } else {
                    false
                }
            }
            ClientStrategy::ForwardedTcpPort(_) => false,
        }
    }
}
