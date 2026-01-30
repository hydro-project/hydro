use std::any::Any;
use std::fmt::Debug;
use std::sync::{Arc, Mutex, OnceLock};

use anyhow::Result;
use nanoid::nanoid;
use serde_json::json;

use super::terraform::{TERRAFORM_ALPHABET, TerraformOutput, TerraformProvider};
use super::{ClientStrategy, Host, HostTargetType, LaunchedHost, ResourceBatch, ResourceResult};
use crate::ssh::LaunchedSshHost;
use crate::{BaseServerStrategy, HostStrategyGetter, PortNetworkHint};

pub struct LaunchedEc2Instance {
    resource_result: Arc<ResourceResult>,
    user: String,
    pub internal_ip: String,
    pub external_ip: Option<String>,
}

impl LaunchedSshHost for LaunchedEc2Instance {
    fn get_external_ip(&self) -> Option<String> {
        self.external_ip.clone()
    }

    fn get_internal_ip(&self) -> String {
        self.internal_ip.clone()
    }

    fn get_cloud_provider(&self) -> String {
        "AWS".to_string()
    }

    fn resource_result(&self) -> &Arc<ResourceResult> {
        &self.resource_result
    }

    fn ssh_user(&self) -> &str {
        self.user.as_str()
    }
}

#[derive(Debug)]
pub struct AwsNetwork {
    pub region: String,
    pub existing_vpc: OnceLock<String>,
    id: String,
}

impl AwsNetwork {
    pub fn new(region: impl Into<String>, existing_vpc: Option<String>) -> Arc<Self> {
        Arc::new(Self {
            region: region.into(),
            existing_vpc: existing_vpc.map(From::from).unwrap_or_default(),
            id: nanoid!(8, &TERRAFORM_ALPHABET),
        })
    }

    fn collect_resources(&self, resource_batch: &mut ResourceBatch) -> String {
        resource_batch
            .terraform
            .terraform
            .required_providers
            .insert(
                "aws".to_string(),
                TerraformProvider {
                    source: "hashicorp/aws".to_string(),
                    version: "5.0.0".to_string(),
                },
            );

        resource_batch.terraform.provider.insert(
            "aws".to_string(),
            json!({
                "region": self.region
            }),
        );

        let vpc_network = format!("hydro-vpc-network-{}", self.id);

        if let Some(existing) = self.existing_vpc.get() {
            if resource_batch
                .terraform
                .resource
                .get("aws_vpc")
                .is_some_and(|map| map.contains_key(existing))
            {
                format!("aws_vpc.{existing}")
            } else {
                resource_batch
                    .terraform
                    .data
                    .entry("aws_vpc".to_string())
                    .or_default()
                    .insert(
                        vpc_network.clone(),
                        json!({
                            "id": existing,
                        }),
                    );

                format!("data.aws_vpc.{vpc_network}")
            }
        } else {
            resource_batch
                .terraform
                .resource
                .entry("aws_vpc".to_string())
                .or_default()
                .insert(
                    vpc_network.clone(),
                    json!({
                        "cidr_block": "10.0.0.0/16",
                        "enable_dns_hostnames": true,
                        "enable_dns_support": true,
                        "tags": {
                            "Name": vpc_network
                        }
                    }),
                );

            // Create internet gateway
            let igw_key = format!("{vpc_network}-igw");
            resource_batch
                .terraform
                .resource
                .entry("aws_internet_gateway".to_string())
                .or_default()
                .insert(
                    igw_key.clone(),
                    json!({
                        "vpc_id": format!("${{aws_vpc.{}.id}}", vpc_network),
                        "tags": {
                            "Name": igw_key
                        }
                    }),
                );

            // Create subnet
            let subnet_key = format!("{vpc_network}-subnet");
            resource_batch
                .terraform
                .resource
                .entry("aws_subnet".to_string())
                .or_default()
                .insert(
                    subnet_key.clone(),
                    json!({
                        "vpc_id": format!("${{aws_vpc.{}.id}}", vpc_network),
                        "cidr_block": "10.0.1.0/24",
                        "availability_zone": format!("{}a", self.region),
                        "map_public_ip_on_launch": true,
                        "tags": {
                            "Name": subnet_key
                        }
                    }),
                );

            // Create route table
            let rt_key = format!("{vpc_network}-rt");
            resource_batch
                .terraform
                .resource
                .entry("aws_route_table".to_string())
                .or_default()
                .insert(
                    rt_key.clone(),
                    json!({
                        "vpc_id": format!("${{aws_vpc.{}.id}}", vpc_network),
                        "tags": {
                            "Name": rt_key
                        }
                    }),
                );

            // Create route
            resource_batch
                .terraform
                .resource
                .entry("aws_route".to_string())
                .or_default()
                .insert(
                    format!("{vpc_network}-route"),
                    json!({
                        "route_table_id": format!("${{aws_route_table.{}.id}}", rt_key),
                        "destination_cidr_block": "0.0.0.0/0",
                        "gateway_id": format!("${{aws_internet_gateway.{}.id}}", igw_key)
                    }),
                );

            resource_batch
                .terraform
                .resource
                .entry("aws_route_table_association".to_string())
                .or_default()
                .insert(
                    format!("{vpc_network}-rta"),
                    json!({
                        "subnet_id": format!("${{aws_subnet.{}.id}}", subnet_key),
                        "route_table_id": format!("${{aws_route_table.{}.id}}", rt_key)
                    }),
                );

            // Create security group that allows internal communication
            let sg_key = format!("{vpc_network}-default-sg");
            resource_batch
                .terraform
                .resource
                .entry("aws_security_group".to_string())
                .or_default()
                .insert(
                    sg_key.clone(),
                    json!({
                        "name": format!("{vpc_network}-default-allow-internal"),
                        "description": "Allow internal communication between instances",
                        "vpc_id": format!("${{aws_vpc.{}.id}}", vpc_network),
                        "ingress": [
                            {
                                "from_port": 0,
                                "to_port": 65535,
                                "protocol": "tcp",
                                "cidr_blocks": ["10.0.0.0/16"],
                                "description": "Allow all TCP traffic within VPC",
                                "ipv6_cidr_blocks": [],
                                "prefix_list_ids": [],
                                "security_groups": [],
                                "self": false
                            },
                            {
                                "from_port": 0,
                                "to_port": 65535,
                                "protocol": "udp",
                                "cidr_blocks": ["10.0.0.0/16"],
                                "description": "Allow all UDP traffic within VPC",
                                "ipv6_cidr_blocks": [],
                                "prefix_list_ids": [],
                                "security_groups": [],
                                "self": false
                            },
                            {
                                "from_port": -1,
                                "to_port": -1,
                                "protocol": "icmp",
                                "cidr_blocks": ["10.0.0.0/16"],
                                "description": "Allow ICMP within VPC",
                                "ipv6_cidr_blocks": [],
                                "prefix_list_ids": [],
                                "security_groups": [],
                                "self": false
                            }
                        ],
                        "egress": [
                            {
                                "from_port": 0,
                                "to_port": 0,
                                "protocol": "-1",
                                "cidr_blocks": ["0.0.0.0/0"],
                                "description": "Allow all outbound traffic",
                                "ipv6_cidr_blocks": [],
                                "prefix_list_ids": [],
                                "security_groups": [],
                                "self": false
                            }
                        ]
                    }),
                );

            let out = format!("aws_vpc.{vpc_network}");
            self.existing_vpc.set(vpc_network).unwrap();
            out
        }
    }
}

/// Represents a IAM role, IAM policy attachments, and instance profile for one or multiple EC2 instances.
#[derive(Debug)]
pub struct AwsEc2IamInstanceProfile {
    pub region: String,
    pub existing_instance_profile_key_or_name: Option<String>,
    pub policy_arns: Vec<String>,
    id: String,
}

impl AwsEc2IamInstanceProfile {
    /// Creates a new instance. If `existing_instance_profile_name` is `Some`, that will be used as the instance
    /// profile name which must already exist in the AWS account.
    pub fn new(region: impl Into<String>, existing_instance_profile_name: Option<String>) -> Self {
        Self {
            region: region.into(),
            existing_instance_profile_key_or_name: existing_instance_profile_name,
            policy_arns: Default::default(),
            id: nanoid!(8, &TERRAFORM_ALPHABET),
        }
    }

    /// Permits the given ARN.
    pub fn add_policy_arn(mut self, policy_arn: impl Into<String>) -> Self {
        if self.existing_instance_profile_key_or_name.is_some() {
            panic!("Adding an ARN to an existing instance profile is not supported.");
        }
        self.policy_arns.push(policy_arn.into());
        self
    }

    /// Enables running and emitting telemetry via the CloudWatch agent.
    pub fn add_cloudwatch_agent_server_policy_arn(self) -> Self {
        self.add_policy_arn("arn:aws:iam::aws:policy/CloudWatchAgentServerPolicy")
    }

    fn collect_resources(&mut self, resource_batch: &mut ResourceBatch) -> String {
        const RESOURCE_AWS_IAM_INSTANCE_PROFILE: &str = "aws_iam_instance_profile";
        const RESOURCE_AWS_IAM_ROLE_POLICY_ATTACHMENT: &str = "aws_iam_role_policy_attachment";
        const RESOURCE_AWS_IAM_ROLE: &str = "aws_iam_role";

        resource_batch
            .terraform
            .terraform
            .required_providers
            .insert(
                "aws".to_string(),
                TerraformProvider {
                    source: "hashicorp/aws".to_string(),
                    version: "5.0.0".to_string(),
                },
            );

        resource_batch.terraform.provider.insert(
            "aws".to_string(),
            json!({
                "region": self.region
            }),
        );

        let instance_profile_key = format!("hydro-instance-profile-{}", self.id);

        if let Some(existing) = self.existing_instance_profile_key_or_name.as_ref() {
            if resource_batch
                .terraform
                .resource
                .get(RESOURCE_AWS_IAM_INSTANCE_PROFILE)
                .is_some_and(|map| map.contains_key(existing))
            {
                // `existing` is a key.
                format!("{RESOURCE_AWS_IAM_INSTANCE_PROFILE}.{existing}")
            } else {
                // `existing` is a name of an existing resource, supplied when constructed.
                resource_batch
                    .terraform
                    .data
                    .entry(RESOURCE_AWS_IAM_INSTANCE_PROFILE.to_string())
                    .or_default()
                    .insert(
                        instance_profile_key.clone(),
                        json!({
                            "id": existing,
                        }),
                    );

                format!("data.{RESOURCE_AWS_IAM_INSTANCE_PROFILE}.{instance_profile_key}")
            }
        } else {
            // Create the role (permissions set after).
            let iam_role_key = format!("{instance_profile_key}-iam-role");
            resource_batch
                .terraform
                .resource
                .entry(RESOURCE_AWS_IAM_ROLE.to_string())
                .or_default()
                .insert(
                    iam_role_key.clone(),
                    json!({
                        "name": format!("hydro-iam-role-{}", self.id),
                        "assume_role_policy": json!({
                            "Version": "2012-10-17",
                            "Statement": [
                                {
                                    "Action": "sts:AssumeRole",
                                    "Effect": "Allow",
                                    "Principal": {
                                        "Service": "ec2.amazonaws.com"
                                    }
                                }
                            ]
                        }).to_string(),
                    }),
                );

            // Attach permissions
            for (i, policy_arn) in self.policy_arns.iter().enumerate() {
                let policy_attachment_key = format!("{iam_role_key}-policy-attachment-{i}");
                resource_batch
                    .terraform
                    .resource
                    .entry(RESOURCE_AWS_IAM_ROLE_POLICY_ATTACHMENT.to_string())
                    .or_default()
                    .insert(
                        policy_attachment_key,
                        json!({
                            "policy_arn": policy_arn,
                            "role": format!("${{{RESOURCE_AWS_IAM_ROLE}.{iam_role_key}.name}}"),
                        }),
                    );
            }

            // Create instance profile. This is what attaches to EC2 instances.
            resource_batch
                .terraform
                .resource
                .entry(RESOURCE_AWS_IAM_INSTANCE_PROFILE.to_string())
                .or_default()
                .insert(
                    instance_profile_key.clone(),
                    json!({
                        "name": format!("hydro-instance-profile-{}", self.id),
                        "role": format!("${{{RESOURCE_AWS_IAM_ROLE}.{iam_role_key}.name}}"),
                    }),
                );

            // Set key
            self.existing_instance_profile_key_or_name = Some(instance_profile_key.clone());

            format!("{RESOURCE_AWS_IAM_INSTANCE_PROFILE}.{instance_profile_key}")
        }
    }
}

/// Represents a CloudWatch log group.
#[derive(Debug)]
pub struct AwsCloudwatchLogGroup {
    pub region: String,
    pub existing_cloudwatch_log_group_key_or_name: Option<String>,
    id: String,
}

impl AwsCloudwatchLogGroup {
    /// Creates a new instance. If `existing_cloudwatch_log_group_name` is `Some`, that will be used as the CloudWatch
    /// log group name which must already exist in the AWS account and region.
    pub fn new(
        region: impl Into<String>,
        existing_cloudwatch_log_group_name: Option<String>,
    ) -> Self {
        Self {
            region: region.into(),
            existing_cloudwatch_log_group_key_or_name: existing_cloudwatch_log_group_name,
            id: nanoid!(8, &TERRAFORM_ALPHABET),
        }
    }

    fn collect_resources(&mut self, resource_batch: &mut ResourceBatch) -> String {
        const RESOURCE_AWS_CLOUDWATCH_LOG_GROUP: &str = "aws_cloudwatch_log_group";

        resource_batch
            .terraform
            .terraform
            .required_providers
            .insert(
                "aws".to_string(),
                TerraformProvider {
                    source: "hashicorp/aws".to_string(),
                    version: "5.0.0".to_string(),
                },
            );

        resource_batch.terraform.provider.insert(
            "aws".to_string(),
            json!({
                "region": self.region
            }),
        );

        let cloudwatch_log_group_key = format!("hydro-cloudwatch-log-group-{}", self.id);

        if let Some(existing) = self.existing_cloudwatch_log_group_key_or_name.as_ref() {
            if resource_batch
                .terraform
                .resource
                .get(RESOURCE_AWS_CLOUDWATCH_LOG_GROUP)
                .is_some_and(|map| map.contains_key(existing))
            {
                // `existing` is a key.
                format!("{RESOURCE_AWS_CLOUDWATCH_LOG_GROUP}.{existing}")
            } else {
                // `existing` is a name of an existing resource, supplied when constructed.
                resource_batch
                    .terraform
                    .data
                    .entry(RESOURCE_AWS_CLOUDWATCH_LOG_GROUP.to_string())
                    .or_default()
                    .insert(
                        cloudwatch_log_group_key.clone(),
                        json!({
                            "id": existing,
                        }),
                    );

                format!("data.{RESOURCE_AWS_CLOUDWATCH_LOG_GROUP}.{cloudwatch_log_group_key}")
            }
        } else {
            // Create the log group.
            resource_batch
                .terraform
                .resource
                .entry(RESOURCE_AWS_CLOUDWATCH_LOG_GROUP.to_string())
                .or_default()
                .insert(
                    cloudwatch_log_group_key.clone(),
                    json!({
                        "name": format!("hydro-cloudwatch-log-group-{}", self.id),
                        "retention_in_days": 1,
                    }),
                );

            // Set key
            self.existing_cloudwatch_log_group_key_or_name = Some(cloudwatch_log_group_key.clone());

            format!("{RESOURCE_AWS_CLOUDWATCH_LOG_GROUP}.{cloudwatch_log_group_key}")
        }
    }
}

pub struct AwsEc2Host {
    /// ID from [`crate::Deployment::add_host`].
    id: usize,

    region: String,
    instance_type: String,
    target_type: HostTargetType,
    ami: String,
    network: Arc<AwsNetwork>,
    iam_instance_profile: Option<Arc<Mutex<AwsEc2IamInstanceProfile>>>,
    cloudwatch_log_group: Option<Arc<Mutex<AwsCloudwatchLogGroup>>>,
    cwa_metrics_collected: Option<serde_json::Value>,
    user: Option<String>,
    display_name: Option<String>,
    pub launched: OnceLock<Arc<LaunchedEc2Instance>>,
    external_ports: Mutex<Vec<u16>>,
}

impl Debug for AwsEc2Host {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "AwsEc2Host({} ({:?}))",
            self.id, &self.display_name
        ))
    }
}

impl AwsEc2Host {
    #[expect(clippy::too_many_arguments, reason = "used via builder pattern")]
    pub fn new(
        id: usize,
        region: impl Into<String>,
        instance_type: impl Into<String>,
        target_type: HostTargetType,
        ami: impl Into<String>,
        network: Arc<AwsNetwork>,
        iam_instance_profile: Option<Arc<Mutex<AwsEc2IamInstanceProfile>>>,
        cloudwatch_log_group: Option<Arc<Mutex<AwsCloudwatchLogGroup>>>,
        cwa_metrics_collected: Option<serde_json::Value>,
        user: Option<String>,
        display_name: Option<String>,
    ) -> Self {
        Self {
            id,
            region: region.into(),
            instance_type: instance_type.into(),
            target_type,
            ami: ami.into(),
            network,
            iam_instance_profile,
            cloudwatch_log_group,
            cwa_metrics_collected,
            user,
            display_name,
            launched: OnceLock::new(),
            external_ports: Mutex::new(Vec::new()),
        }
    }
}

impl Host for AwsEc2Host {
    fn target_type(&self) -> HostTargetType {
        self.target_type
    }

    fn request_port_base(&self, bind_type: &BaseServerStrategy) {
        match bind_type {
            BaseServerStrategy::UnixSocket => {}
            BaseServerStrategy::InternalTcpPort(_) => {}
            BaseServerStrategy::ExternalTcpPort(port) => {
                let mut external_ports = self.external_ports.lock().unwrap();
                if !external_ports.contains(port) {
                    if self.launched.get().is_some() {
                        todo!("Cannot adjust security group after host has been launched");
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

        let vpc_path = self.network.collect_resources(resource_batch);

        let iam_instance_profile = self
            .iam_instance_profile
            .as_deref()
            .map(|irip| irip.lock().unwrap().collect_resources(resource_batch));

        let cloudwatch_log_group = self
            .cloudwatch_log_group
            .as_deref()
            .map(|cwlg| cwlg.lock().unwrap().collect_resources(resource_batch));

        // Add additional providers
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

        // Generate SSH key pair
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
                    "file_permission": "0600",
                    "directory_permission": "0700"
                }),
            );

        resource_batch
            .terraform
            .resource
            .entry("aws_key_pair".to_string())
            .or_default()
            .insert(
                "ec2_key_pair".to_string(),
                json!({
                    "key_name": format!("hydro-key-{}", nanoid!(8, &TERRAFORM_ALPHABET)),
                    "public_key": "${tls_private_key.vm_instance_ssh_key.public_key_openssh}"
                }),
            );

        let instance_key = format!("ec2-instance-{}", self.id);
        let mut instance_name = format!("hydro-ec2-instance-{}", nanoid!(8, &TERRAFORM_ALPHABET));

        if let Some(mut display_name) = self.display_name.clone() {
            instance_name.push('-');
            display_name = display_name.replace("_", "-").to_lowercase();

            let num_chars_to_cut = instance_name.len() + display_name.len() - 63;
            if num_chars_to_cut > 0 {
                display_name.drain(0..num_chars_to_cut);
            }
            instance_name.push_str(&display_name);
        }

        let network_id = self.network.id.clone();
        let vpc_ref = format!("${{{}.id}}", vpc_path);
        let default_sg_ref = format!(
            "${{aws_security_group.hydro-vpc-network-{}-default-sg.id}}",
            network_id
        );

        // Create additional security group for external ports if needed
        let mut security_groups = vec![default_sg_ref.clone()];
        let external_ports = self.external_ports.lock().unwrap();

        if !external_ports.is_empty() {
            let sg_key = format!("sg-{}", self.id);
            let mut sg_rules = vec![];

            for port in external_ports.iter() {
                sg_rules.push(json!({
                    "from_port": port,
                    "to_port": port,
                    "protocol": "tcp",
                    "cidr_blocks": ["0.0.0.0/0"],
                    "description": format!("External port {}", port),
                    "ipv6_cidr_blocks": [],
                    "prefix_list_ids": [],
                    "security_groups": [],
                    "self": false
                }));
            }

            resource_batch
                .terraform
                .resource
                .entry("aws_security_group".to_string())
                .or_default()
                .insert(
                    sg_key.clone(),
                    json!({
                        "name": format!("hydro-sg-{}", nanoid!(8, &TERRAFORM_ALPHABET)),
                        "description": "Hydro external ports security group",
                        "vpc_id": vpc_ref,
                        "ingress": sg_rules,
                        "egress": [{
                            "from_port": 0,
                            "to_port": 0,
                            "protocol": "-1",
                            "cidr_blocks": ["0.0.0.0/0"],
                            "description": "All outbound traffic",
                            "ipv6_cidr_blocks": [],
                            "prefix_list_ids": [],
                            "security_groups": [],
                            "self": false
                        }]
                    }),
                );

            security_groups.push(format!("${{aws_security_group.{}.id}}", sg_key));
        }
        drop(external_ports);

        let subnet_ref = format!("${{aws_subnet.hydro-vpc-network-{}-subnet.id}}", network_id);
        let iam_instance_profile_ref = iam_instance_profile.map(|key| format!("${{{key}.name}}"));

        // Write the CloudWatch Agent config file.
        // https://docs.aws.amazon.com/AmazonCloudWatch/latest/monitoring/CloudWatch-Agent-Configuration-File-Details.html
        let cloudwatch_agent_config = cloudwatch_log_group.map(|cwlg| {
            json!({
                "logs": {
                    "logs_collected": {
                        "files": {
                            "collect_list": [
                                {
                                    "file_path": "/var/log/hydro/metrics.log",
                                    "log_group_name": format!("${{{cwlg}.name}}"), // This `$` is interpreted by terraform
                                    "log_stream_name": "{{instance_id}}"
                                }
                            ]
                        }
                    }
                },
                "metrics": {
                    // "namespace": todo!(), // TODO(mingwei): use flow_name here somehow
                    "metrics_collected": self.cwa_metrics_collected.as_ref().unwrap_or(&json!({
                        "cpu": {
                            "resources": [
                                "*"
                            ],
                            "measurement": [
                                "usage_active"
                            ],
                            "totalcpu": true
                        },
                        "mem": {
                            "measurement": [
                                "used_percent"
                            ]
                        }
                    })),
                    // See special escape handling below.
                    "append_dimensions": {
                        "InstanceId": "${aws:InstanceId}"
                    }
                }
            })
            .to_string()
        });

        // TODO(mingwei): Run this in SSH instead of `user_data` to avoid racing and capture errors.
        let user_data_script = cloudwatch_agent_config.map(|cwa_config| {
            let cwa_config_esc = cwa_config
                .replace("\\", r"\\") // escape backslashes
                .replace("\"", r#"\""#) // escape quotes
                .replace("\n", r"\n") // escape newlines
                // Special handling of AWS `append_dimensions` fields:
                // `$$` to escape for terraform, becomes `\$` in bash, becomes `$` in echo output.
                .replace("${aws:", r"\$${aws:");
            format!(
                r##"
#!/bin/bash
set -euxo pipefail

mkdir -p /var/log/hydro/
chmod +777 /var/log/hydro
touch /var/log/hydro/metrics.log
chmod +666 /var/log/hydro/metrics.log

# Install the CloudWatch Agent
yum install -y amazon-cloudwatch-agent

mkdir -p /opt/aws/amazon-cloudwatch-agent/etc
echo -e "{cwa_config_esc}" > /opt/aws/amazon-cloudwatch-agent/etc/amazon-cloudwatch-agent.json

# Start or restart the agent
/opt/aws/amazon-cloudwatch-agent/bin/amazon-cloudwatch-agent-ctl \
    -a fetch-config -m ec2 \
    -c file:/opt/aws/amazon-cloudwatch-agent/etc/amazon-cloudwatch-agent.json \
    -s
"##
            )
        });

        // Create EC2 instance
        resource_batch
            .terraform
            .resource
            .entry("aws_instance".to_string())
            .or_default()
            .insert(
                instance_key.clone(),
                json!({
                    "ami": self.ami,
                    "instance_type": self.instance_type,
                    "key_name": "${aws_key_pair.ec2_key_pair.key_name}",
                    "vpc_security_group_ids": security_groups,
                    "subnet_id": subnet_ref,
                    "associate_public_ip_address": true,
                    "iam_instance_profile": iam_instance_profile_ref, // May be `None`.
                    "user_data": user_data_script, // May be `None`.
                    "tags": {
                        "Name": instance_name
                    }
                }),
            );

        resource_batch.terraform.output.insert(
            format!("{}-private-ip", instance_key),
            TerraformOutput {
                value: format!("${{aws_instance.{}.private_ip}}", instance_key),
            },
        );

        resource_batch.terraform.output.insert(
            format!("{}-public-ip", instance_key),
            TerraformOutput {
                value: format!("${{aws_instance.{}.public_ip}}", instance_key),
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
                    .get(&format!("ec2-instance-{id}-private-ip"))
                    .unwrap()
                    .value
                    .clone();

                let external_ip = resource_result
                    .terraform
                    .outputs
                    .get(&format!("ec2-instance-{id}-public-ip"))
                    .map(|v| v.value.clone());

                Arc::new(LaunchedEc2Instance {
                    resource_result: resource_result.clone(),
                    user: self
                        .user
                        .as_ref()
                        .cloned()
                        .unwrap_or("ec2-user".to_string()),
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
                    me.downcast_ref::<AwsEc2Host>()
                        .unwrap()
                        .request_port_base(&BaseServerStrategy::ExternalTcpPort(22));
                    BaseServerStrategy::InternalTcpPort(None)
                }),
            ))
        } else {
            anyhow::bail!("Could not find a strategy to connect to AWS EC2 instance")
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
                if let Some(aws_target) = <dyn Any>::downcast_ref::<AwsEc2Host>(target_host) {
                    self.region == aws_target.region
                        && Arc::ptr_eq(&self.network, &aws_target.network)
                } else {
                    false
                }
            }
            ClientStrategy::ForwardedTcpPort(_) => false,
        }
    }
}
