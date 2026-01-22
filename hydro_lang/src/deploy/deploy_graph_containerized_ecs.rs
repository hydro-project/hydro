//! Deployment backend for Hydro that uses Docker to provision and launch services.

use std::cell::RefCell;
use std::collections::HashMap;
use std::pin::Pin;
use std::rc::Rc;

use bollard::Docker;
use bollard::query_parameters::{BuildImageOptions, TagImageOptions};
use bytes::Bytes;
use dfir_lang::graph::DfirGraph;
use futures::{Sink, SinkExt, Stream, StreamExt};
use http_body_util::Full;
use hydro_deploy::rust_crate::build::{BuildError, build_crate_memoized};
use hydro_deploy::{LinuxCompileType, RustCrate};
use nanoid::nanoid;
use proc_macro2::Span;
use serde_json::{Map, Value, json};
use sinktools::lazy::LazySink;
use stageleft::QuotedWithContext;
use syn::parse_quote;
use tar::{Builder, Header};
use tokio::net::TcpStream;
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};
use tracing::{Instrument, instrument, trace};

use super::deploy_runtime_containerized_ecs::*;

/// Task configuration for CloudFormation template generation
struct TaskConfig {
    task_name: String,
    image_uri: String,
    deployment_instance: String,
    exposed_ports: Vec<u16>,
    region: String,
}

/// Generate a complete CloudFormation template with all infrastructure and tasks
fn generate_cloudformation_template(
    deployment_instance: &str,
    tasks: &[TaskConfig],
) -> anyhow::Result<String> {
    let mut resources: Map<String, Value> = Map::new();

    // Base infrastructure resources
    resources.insert(
        "EcsTaskExecutionRole".to_string(),
        json!({
            "Type": "AWS::IAM::Role",
            "Properties": {
                "RoleName": format!("hydro-exec-{}", deployment_instance),
                "AssumeRolePolicyDocument": {
                    "Version": "2012-10-17",
                    "Statement": [{
                        "Effect": "Allow",
                        "Principal": { "Service": "ecs-tasks.amazonaws.com" },
                        "Action": "sts:AssumeRole"
                    }]
                },
                "ManagedPolicyArns": [
                    "arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy"
                ],
                "Policies": [{
                    "PolicyName": "CloudWatchLogsCreateLogGroup",
                    "PolicyDocument": {
                        "Version": "2012-10-17",
                        "Statement": [{
                            "Effect": "Allow",
                            "Action": "logs:CreateLogGroup",
                            "Resource": "*"
                        }]
                    }
                }]
            }
        }),
    );

    resources.insert(
        "EcsTaskRole".to_string(),
        json!({
            "Type": "AWS::IAM::Role",
            "Properties": {
                "RoleName": format!("hydro-task-{}", deployment_instance),
                "AssumeRolePolicyDocument": {
                    "Version": "2012-10-17",
                    "Statement": [{
                        "Effect": "Allow",
                        "Principal": { "Service": "ecs-tasks.amazonaws.com" },
                        "Action": "sts:AssumeRole"
                    }]
                },
                "Policies": [{
                    "PolicyName": "HydroEcsAccess",
                    "PolicyDocument": {
                        "Version": "2012-10-17",
                        "Statement": [{
                            "Effect": "Allow",
                            "Action": [
                                "ecs:ListTasks",
                                "ecs:DescribeTasks",
                                "ec2:DescribeNetworkInterfaces"
                            ],
                            "Resource": "*"
                        }]
                    }
                }]
            }
        }),
    );

    resources.insert(
        "VPC".to_string(),
        json!({
            "Type": "AWS::EC2::VPC",
            "Properties": {
                "CidrBlock": "10.0.0.0/16",
                "EnableDnsSupport": true,
                "EnableDnsHostnames": true,
                "Tags": [{ "Key": "Name", "Value": format!("hydro-vpc-{}", deployment_instance) }]
            }
        }),
    );

    resources.insert("Subnet".to_string(), json!({
        "Type": "AWS::EC2::Subnet",
        "Properties": {
            "VpcId": { "Ref": "VPC" },
            "CidrBlock": "10.0.1.0/24",
            "MapPublicIpOnLaunch": true,
            "Tags": [{ "Key": "Name", "Value": format!("hydro-subnet-{}", deployment_instance) }]
        }
    }));

    resources.insert(
        "InternetGateway".to_string(),
        json!({
            "Type": "AWS::EC2::InternetGateway",
            "Properties": {
                "Tags": [{ "Key": "Name", "Value": format!("hydro-igw-{}", deployment_instance) }]
            }
        }),
    );

    resources.insert(
        "VPCGatewayAttachment".to_string(),
        json!({
            "Type": "AWS::EC2::VPCGatewayAttachment",
            "Properties": {
                "VpcId": { "Ref": "VPC" },
                "InternetGatewayId": { "Ref": "InternetGateway" }
            }
        }),
    );

    resources.insert(
        "RouteTable".to_string(),
        json!({
            "Type": "AWS::EC2::RouteTable",
            "Properties": {
                "VpcId": { "Ref": "VPC" },
                "Tags": [{ "Key": "Name", "Value": format!("hydro-rt-{}", deployment_instance) }]
            }
        }),
    );

    resources.insert(
        "Route".to_string(),
        json!({
            "Type": "AWS::EC2::Route",
            "DependsOn": "VPCGatewayAttachment",
            "Properties": {
                "RouteTableId": { "Ref": "RouteTable" },
                "DestinationCidrBlock": "0.0.0.0/0",
                "GatewayId": { "Ref": "InternetGateway" }
            }
        }),
    );

    resources.insert(
        "SubnetRouteTableAssociation".to_string(),
        json!({
            "Type": "AWS::EC2::SubnetRouteTableAssociation",
            "Properties": {
                "SubnetId": { "Ref": "Subnet" },
                "RouteTableId": { "Ref": "RouteTable" }
            }
        }),
    );

    resources.insert(
        "SecurityGroup".to_string(),
        json!({
            "Type": "AWS::EC2::SecurityGroup",
            "Properties": {
                "GroupDescription": "Security group for Hydro ECS tasks",
                "VpcId": { "Ref": "VPC" },
                "SecurityGroupIngress": [{
                    "IpProtocol": "-1",
                    "CidrIp": "0.0.0.0/0"
                }],
                "Tags": [{ "Key": "Name", "Value": format!("hydro-sg-{}", deployment_instance) }]
            }
        }),
    );

    resources.insert(
        "EcsCluster".to_string(),
        json!({
            "Type": "AWS::ECS::Cluster",
            "Properties": {
                "ClusterName": format!("hydro-{}", deployment_instance)
            }
        }),
    );

    // Generate task definitions and ECS services for each task
    for task in tasks {
        let safe_name = task.task_name.replace('-', "");

        // Port mappings
        let port_mappings: Vec<Value> = task
            .exposed_ports
            .iter()
            .map(|p| {
                json!({
                    "ContainerPort": *p as i32,
                    "Protocol": "tcp"
                })
            })
            .collect();

        // Task Definition
        resources.insert(format!("TaskDef{}", safe_name), json!({
            "Type": "AWS::ECS::TaskDefinition",
            "Properties": {
                "Family": task.task_name,
                "RequiresCompatibilities": ["FARGATE"],
                "NetworkMode": "awsvpc",
                "Cpu": "256",
                "Memory": "512",
                "ExecutionRoleArn": { "Fn::GetAtt": ["EcsTaskExecutionRole", "Arn"] },
                "TaskRoleArn": { "Fn::GetAtt": ["EcsTaskRole", "Arn"] },
                "ContainerDefinitions": [{
                    "Name": task.task_name,
                    "Image": task.image_uri,
                    "PortMappings": port_mappings,
                    "LogConfiguration": {
                        "LogDriver": "awslogs",
                        "Options": {
                            "awslogs-group": "/ecs/hydro",
                            "awslogs-region": task.region,
                            "awslogs-stream-prefix": "ecs",
                            "awslogs-create-group": "true"
                        }
                    },
                    "Environment": [
                        { "Name": "CONTAINER_NAME", "Value": task.task_name },
                        { "Name": "DEPLOYMENT_INSTANCE", "Value": task.deployment_instance },
                        { "Name": "RUST_LOG", "Value": "trace,aws_runtime=info,aws_sdk_ecs=info,aws_smithy_runtime=info,aws_smithy_runtime_api=info,aws_config=info,hyper_util=info,aws_smithy_http_client=info,aws_sigv4=info" },
                        { "Name": "RUST_BACKTRACE", "Value": "1" },
                        { "Name": "NO_COLOR", "Value": "1" }
                    ]
                }]
            }
        }));

        // ECS Service
        resources.insert(
            format!("EcsService{}", safe_name),
            json!({
                "Type": "AWS::ECS::Service",
                "DependsOn": ["Route", format!("TaskDef{}", safe_name)],
                "Properties": {
                    "ServiceName": task.task_name,
                    "Cluster": { "Ref": "EcsCluster" },
                    "TaskDefinition": { "Ref": format!("TaskDef{}", safe_name) },
                    "DesiredCount": 1,
                    "LaunchType": "FARGATE",
                    "NetworkConfiguration": {
                        "AwsvpcConfiguration": {
                            "Subnets": [{ "Ref": "Subnet" }],
                            "SecurityGroups": [{ "Ref": "SecurityGroup" }],
                            "AssignPublicIp": "ENABLED"
                        }
                    }
                }
            }),
        );
    }

    let template = json!({
        "AWSTemplateFormatVersion": "2010-09-09",
        "Description": "Hydro ECS Infrastructure",
        "Resources": resources,
        "Outputs": {
            "ClusterName": {
                "Value": { "Ref": "EcsCluster" }
            }
        }
    });

    Ok(serde_json::to_string(&template)?)
}

#[instrument(level = "trace", skip_all, fields(%deployment_instance))]
async fn deploy_stack(
    deployment_instance: &str,
    tasks: &[TaskConfig],
) -> Result<String, anyhow::Error> {
    use aws_config::BehaviorVersion;
    use aws_sdk_cloudformation::Client as CfnClient;
    use aws_sdk_cloudformation::types::{Capability, StackStatus};

    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let cfn_client = CfnClient::new(&config);

    let stack_name = format!("hydro-{}", deployment_instance);
    let template = generate_cloudformation_template(deployment_instance, tasks)?;

    trace!(name: "creating_stack", %stack_name);
    cfn_client
        .create_stack()
        .stack_name(&stack_name)
        .template_body(&template)
        .capabilities(Capability::CapabilityNamedIam)
        .send()
        .await?;

    // Wait for stack creation
    trace!(name: "waiting_for_stack", %stack_name);
    loop {
        let describe = cfn_client
            .describe_stacks()
            .stack_name(&stack_name)
            .send()
            .await?;

        let stack = describe
            .stacks()
            .first()
            .ok_or_else(|| anyhow::anyhow!("Stack not found"))?;

        match stack.stack_status() {
            Some(StackStatus::CreateComplete) => {
                trace!(name: "stack_created", %stack_name);
                break;
            }
            Some(StackStatus::CreateFailed)
            | Some(StackStatus::RollbackComplete)
            | Some(StackStatus::RollbackFailed) => {
                return Err(anyhow::anyhow!(
                    "Stack creation failed: {:?}",
                    stack.stack_status_reason()
                ));
            }
            status => {
                trace!(name: "stack_status", ?status);
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            }
        }
    }

    // Extract cluster name from outputs
    let describe = cfn_client
        .describe_stacks()
        .stack_name(&stack_name)
        .send()
        .await?;

    let stack = describe
        .stacks()
        .first()
        .ok_or_else(|| anyhow::anyhow!("Stack not found"))?;

    let cluster_name = stack
        .outputs()
        .iter()
        .find(|o| o.output_key() == Some("ClusterName"))
        .and_then(|o| o.output_value())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Missing ClusterName output"))?;

    Ok(cluster_name)
}
use crate::compile::builder::ExternalPortId;
use crate::compile::deploy::DeployResult;
use crate::compile::deploy_provider::{
    ClusterSpec, Deploy, ExternalSpec, Node, ProcessSpec, RegisterPort,
};
use crate::compile::trybuild::generate::{LinkingMode, create_graph_trybuild};
use crate::location::dynamic::LocationId;
use crate::location::member_id::TaglessMemberId;
use crate::location::{MembershipEvent, NetworkHint};

/// represents a docker network
#[derive(Clone, Debug)]
pub struct DockerNetworkEcs {
    _name: String,
}

impl DockerNetworkEcs {
    /// creates a new docker network (will actually be created when deployment.start() is called).
    pub fn new(name: String) -> Self {
        Self {
            _name: format!("{name}-{}", nanoid::nanoid!(6, &CONTAINER_ALPHABET)),
        }
    }
}

/// Represents a process running in a docker container
#[derive(Clone)]
pub struct DockerDeployProcessEcs {
    id: usize,
    name: String,
    next_port: Rc<RefCell<u16>>,
    rust_crate: Rc<RefCell<Option<RustCrate>>>,

    exposed_ports: Rc<RefCell<Vec<u16>>>,

    docker_container_name: Rc<RefCell<Option<String>>>,

    compilation_options: Option<String>,

    config: Vec<String>,

    network: DockerNetworkEcs,
}

impl Node for DockerDeployProcessEcs {
    type Port = u16;
    type Meta = ();
    type InstantiateEnv = DockerDeployEcs;

    #[instrument(level = "trace", skip_all, ret, fields(id = self.id, name = self.name))]
    fn next_port(&self) -> Self::Port {
        let port = {
            let mut borrow = self.next_port.borrow_mut();
            let port = *borrow;
            *borrow += 1;
            port
        };

        port
    }

    #[instrument(level = "trace", skip_all, fields(id = self.id, name = self.name))]
    fn update_meta(&self, _meta: &Self::Meta) {}

    #[instrument(level = "trace", skip_all, fields(id = self.id, name = self.name, ?meta, extra_stmts = extra_stmts.len()))]
    fn instantiate(
        &self,
        _env: &mut Self::InstantiateEnv,
        meta: &mut Self::Meta,
        graph: DfirGraph,
        extra_stmts: Vec<syn::Stmt>,
    ) {
        let (bin_name, config) = create_graph_trybuild(
            graph,
            extra_stmts.clone(),
            &Some(self.name.clone()),
            true,
            LinkingMode::Static,
        );

        let mut ret = RustCrate::new(config.project_dir)
            .target_dir(config.target_dir)
            .example(bin_name.clone())
            .no_default_features();

        ret = ret.display_name("test_display_name");

        ret = ret.features(vec!["hydro___feature_ecs_runtime".to_string()]);

        if let Some(features) = config.features {
            ret = ret.features(features);
        }

        ret = ret.build_env("STAGELEFT_TRYBUILD_BUILD_STAGED", "1");
        ret = ret.config("build.incremental = false");

        *self.rust_crate.borrow_mut() = Some(ret);
    }
}

/// Represents a logical cluster, which can be a variable amount of individual containers.
#[derive(Clone)]
pub struct DockerDeployClusterEcs {
    id: usize,
    name: String,
    next_port: Rc<RefCell<u16>>,
    rust_crate: Rc<RefCell<Option<RustCrate>>>,

    docker_container_name: Rc<RefCell<Vec<String>>>,

    compilation_options: Option<String>,

    config: Vec<String>,

    count: usize,
}

impl Node for DockerDeployClusterEcs {
    type Port = u16;
    type Meta = ();
    type InstantiateEnv = DockerDeployEcs;

    #[instrument(level = "trace", skip_all, ret, fields(id = self.id, name = self.name))]
    fn next_port(&self) -> Self::Port {
        let port = {
            let mut borrow = self.next_port.borrow_mut();
            let port = *borrow;
            *borrow += 1;
            port
        };

        port
    }

    #[instrument(level = "trace", skip_all, fields(id = self.id, name = self.name))]
    fn update_meta(&self, _meta: &Self::Meta) {}

    #[instrument(level = "trace", skip_all, fields(id = self.id, name = self.name, extra_stmts = extra_stmts.len()))]
    fn instantiate(
        &self,
        _env: &mut Self::InstantiateEnv,
        _meta: &mut Self::Meta,
        graph: DfirGraph,
        extra_stmts: Vec<syn::Stmt>,
    ) {
        let (bin_name, config) = create_graph_trybuild(
            graph,
            extra_stmts.clone(),
            &Some(self.name.clone()),
            true,
            LinkingMode::Static,
        );

        let mut ret = RustCrate::new(config.project_dir)
            .target_dir(config.target_dir)
            .example(bin_name.clone())
            .no_default_features();

        ret = ret.display_name("test_display_name");

        ret = ret.features(vec!["hydro___feature_ecs_runtime".to_string()]);

        if let Some(features) = config.features {
            ret = ret.features(features);
        }

        ret = ret.build_env("STAGELEFT_TRYBUILD_BUILD_STAGED", "1");
        ret = ret.config("build.incremental = false");

        *self.rust_crate.borrow_mut() = Some(ret);
    }
}

/// Represents an external process, outside the control of this deployment but still with some communication into this deployment.
#[derive(Clone, Debug)]
pub struct DockerDeployExternalEcs {
    name: String,
    next_port: Rc<RefCell<u16>>,

    ports: Rc<RefCell<HashMap<ExternalPortId, u16>>>,

    #[expect(clippy::type_complexity, reason = "internal code")]
    connection_info:
        Rc<RefCell<HashMap<u16, (Rc<RefCell<Option<String>>>, u16, DockerNetworkEcs)>>>,

    deployment_instance: String,
}

impl Node for DockerDeployExternalEcs {
    type Port = u16;
    type Meta = ();
    type InstantiateEnv = DockerDeployEcs;

    #[instrument(level = "trace", skip_all, ret, fields(name = self.name))]
    fn next_port(&self) -> Self::Port {
        let port = {
            let mut borrow = self.next_port.borrow_mut();
            let port = *borrow;
            *borrow += 1;
            port
        };

        port
    }

    #[instrument(level = "trace", skip_all, fields(name = self.name))]
    fn update_meta(&self, _meta: &Self::Meta) {}

    #[instrument(level = "trace", skip_all, fields(name = self.name, ?meta, extra_stmts = extra_stmts.len()))]
    fn instantiate(
        &self,
        _env: &mut Self::InstantiateEnv,
        meta: &mut Self::Meta,
        graph: DfirGraph,
        extra_stmts: Vec<syn::Stmt>,
    ) {
        trace!(name: "surface", surface = graph.surface_syntax_string());
    }
}

type DynSourceSink<Out, In, InErr> = (
    Pin<Box<dyn Stream<Item = Out>>>,
    Pin<Box<dyn Sink<In, Error = InErr>>>,
);

impl<'a> RegisterPort<'a, DockerDeployEcs> for DockerDeployExternalEcs {
    #[instrument(level = "trace", skip_all, fields(name = self.name, %external_port_id, %port))]
    fn register(&self, external_port_id: ExternalPortId, port: Self::Port) {
        self.ports.borrow_mut().insert(external_port_id, port);
    }

    fn as_bytes_bidi(
        &self,
        external_port_id: ExternalPortId,
    ) -> impl Future<
        Output = DynSourceSink<Result<bytes::BytesMut, std::io::Error>, Bytes, std::io::Error>,
    > + 'a {
        let _span =
            tracing::trace_span!("as_bytes_bidi", name = %self.name, %external_port_id).entered(); // the instrument macro doesn't work here because of lifetime issues?
        async { todo!() }
    }

    fn as_bincode_bidi<InT, OutT>(
        &self,
        external_port_id: ExternalPortId,
    ) -> impl Future<Output = DynSourceSink<OutT, InT, std::io::Error>> + 'a
    where
        InT: serde::Serialize + 'static,
        OutT: serde::de::DeserializeOwned + 'static,
    {
        let _span =
            tracing::trace_span!("as_bincode_bidi", name = %self.name, %external_port_id).entered(); // the instrument macro doesn't work here because of lifetime issues?
        async { todo!() }
    }

    fn as_bincode_sink<T>(
        &self,
        external_port_id: ExternalPortId,
    ) -> impl Future<Output = Pin<Box<dyn Sink<T, Error = std::io::Error>>>> + 'a
    where
        T: serde::Serialize + 'static,
    {
        let guard =
            tracing::trace_span!("as_bincode_sink", name = %self.name, %external_port_id).entered();

        let local_port = *self.ports.borrow().get(&external_port_id).unwrap();
        let (docker_container_name, remote_port, _network) = self
            .connection_info
            .borrow()
            .get(&local_port)
            .unwrap()
            .clone();
        let deployment_instance = self.deployment_instance.clone();

        async move {
            use aws_config::BehaviorVersion;
            use aws_sdk_ecs::Client as EcsClient;

            let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
            let ecs_client = EcsClient::new(&config);

            let task_name = docker_container_name.borrow().as_ref().unwrap().clone();
            trace!(name: "query_ecs", %task_name);

            let cluster_name = format!("hydro-{}", deployment_instance);

            let task_arn = loop {
                let tasks = ecs_client
                    .list_tasks()
                    .cluster(&cluster_name)
                    .family(&task_name)
                    .send()
                    .await
                    .unwrap();

                if let Some(arn) = tasks.task_arns().first() {
                    break arn.clone();
                }

                trace!(name: "waiting_for_task", %task_name);
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            };
            trace!(name: "found_task", %task_arn);

            let eni_id = loop {
                let task_details = ecs_client
                    .describe_tasks()
                    .cluster(&cluster_name)
                    .tasks(&task_arn)
                    .send()
                    .await
                    .unwrap();

                if let Some(eni) = task_details.tasks().first().and_then(|task| {
                    task.attachments()
                        .iter()
                        .flat_map(|a| a.details())
                        .find(|d| d.name() == Some("networkInterfaceId"))
                        .and_then(|d| d.value())
                }) {
                    break eni.to_string();
                }

                trace!(name: "waiting_for_eni", %task_arn);
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            };
            trace!(name: "found_eni", %eni_id);

            let ec2_client = aws_sdk_ec2::Client::new(&config);

            let remote_ip_address = loop {
                let eni_info = ec2_client
                    .describe_network_interfaces()
                    .network_interface_ids(&eni_id)
                    .send()
                    .await
                    .unwrap();

                if let Some(ip) = eni_info
                    .network_interfaces()
                    .first()
                    .and_then(|ni| ni.association())
                    .and_then(|assoc| assoc.public_ip())
                {
                    break ip.to_string();
                }

                trace!(name: "waiting_for_public_ip", %eni_id);
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            };
            trace!(name: "resolved_ip", %remote_ip_address);

            Box::pin(
                LazySink::new(move || {
                    Box::pin(async move {
                        trace!(name: "connecting", %remote_ip_address, %remote_port);

                        let stream =
                            TcpStream::connect(format!("{remote_ip_address}:{remote_port}"))
                                .await?;

                        trace!(name: "connected", %remote_ip_address, %remote_port);

                        Result::<_, std::io::Error>::Ok(FramedWrite::new(
                            stream,
                            LengthDelimitedCodec::new(),
                        ))
                    })
                })
                .with(move |v| async move { Ok(Bytes::from(bincode::serialize(&v).unwrap())) }),
            ) as Pin<Box<dyn Sink<T, Error = std::io::Error>>>
        }
        .instrument(guard.exit())
    }

    fn as_bincode_source<T>(
        &self,
        external_port_id: ExternalPortId,
    ) -> impl Future<Output = Pin<Box<dyn Stream<Item = T>>>> + 'a
    where
        T: serde::de::DeserializeOwned + 'static,
    {
        let guard =
            tracing::trace_span!("as_bincode_sink", name = %self.name, %external_port_id).entered();

        let local_port = *self.ports.borrow().get(&external_port_id).unwrap();
        let (docker_container_name, remote_port, _network) = self
            .connection_info
            .borrow()
            .get(&local_port)
            .unwrap()
            .clone();
        let deployment_instance = self.deployment_instance.clone();

        async move {
            use aws_config::BehaviorVersion;
            use aws_sdk_ecs::Client as EcsClient;

            let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
            let ecs_client = EcsClient::new(&config);

            let task_name = docker_container_name.borrow().as_ref().unwrap().clone();
            trace!(name: "query_ecs", %task_name);

            let cluster_name = format!("hydro-{}", deployment_instance);

            let task_arn = loop {
                let tasks = ecs_client
                    .list_tasks()
                    .cluster(&cluster_name)
                    .family(&task_name)
                    .send()
                    .await
                    .unwrap();

                if let Some(arn) = tasks.task_arns().first() {
                    break arn.clone();
                }

                trace!(name: "waiting_for_task", %task_name);
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            };
            trace!(name: "found_task", %task_arn);

            let eni_id = loop {
                let task_details = ecs_client
                    .describe_tasks()
                    .cluster(&cluster_name)
                    .tasks(&task_arn)
                    .send()
                    .await
                    .unwrap();

                if let Some(eni) = task_details.tasks().first().and_then(|task| {
                    task.attachments()
                        .iter()
                        .flat_map(|a| a.details())
                        .find(|d| d.name() == Some("networkInterfaceId"))
                        .and_then(|d| d.value())
                }) {
                    break eni.to_string();
                }

                trace!(name: "waiting_for_eni", %task_arn);
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            };
            trace!(name: "found_eni", %eni_id);

            let ec2_client = aws_sdk_ec2::Client::new(&config);

            let remote_ip_address = loop {
                let eni_info = ec2_client
                    .describe_network_interfaces()
                    .network_interface_ids(&eni_id)
                    .send()
                    .await
                    .unwrap();

                if let Some(ip) = eni_info
                    .network_interfaces()
                    .first()
                    .and_then(|ni| ni.association())
                    .and_then(|assoc| assoc.public_ip())
                {
                    break ip.to_string();
                }

                trace!(name: "waiting_for_public_ip", %eni_id);
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            };
            trace!(name: "resolved_ip", %remote_ip_address);

            trace!(name: "connecting", %remote_ip_address, %remote_port);

            let stream = TcpStream::connect(format!("{remote_ip_address}:{remote_port}"))
                .await
                .unwrap();

            trace!(name: "connected", %remote_ip_address, %remote_port);

            Box::pin(
                FramedRead::new(stream, LengthDelimitedCodec::new())
                    .map(|v| bincode::deserialize(&v.unwrap()).unwrap()),
            ) as Pin<Box<dyn Stream<Item = T>>>
        }
        .instrument(guard.exit())
    }
}

/// For deploying to a local docker instance
pub struct DockerDeployEcs {
    docker_processes: Vec<DockerDeployProcessSpecEcs>,
    docker_clusters: Vec<DockerDeployClusterSpecEcs>,
    network: DockerNetworkEcs,
    deployment_instance: String,
}

#[instrument(level = "trace", skip_all, fields(%image_name))]
async fn build_and_create_image(
    rust_crate: &Rc<RefCell<Option<RustCrate>>>,
    compilation_options: &Option<String>,
    config: &[String],
    image_name: &str,
) -> Result<(), anyhow::Error> {
    let mut rust_crate = rust_crate
        .borrow_mut()
        .take()
        .unwrap()
        .rustflags(compilation_options.clone().unwrap_or("".to_string()));

    for cfg in config {
        rust_crate = rust_crate.config(cfg);
    }

    let build_output = match build_crate_memoized(
        rust_crate.get_build_params(hydro_deploy::HostTargetType::Linux(LinuxCompileType::Glibc)),
    )
    .await
    {
        Ok(build_output) => build_output,
        Err(BuildError::FailedToBuildCrate {
            exit_status,
            diagnostics,
            text_lines,
            stderr_lines,
        }) => {
            let diagnostics = diagnostics
                .into_iter()
                .map(|d| d.rendered.unwrap())
                .collect::<Vec<_>>()
                .join("\n");
            let text_lines = text_lines.join("\n");
            let stderr_lines = stderr_lines.join("\n");

            anyhow::bail!(
                r#"
Failed to build crate {exit_status:?}
--- diagnostics
---
{diagnostics}
---
---
---

--- text_lines
---
---
{text_lines}
---
---
---

--- stderr_lines
---
---
{stderr_lines}
---
---
---"#
            );
        }
        Err(err) => {
            anyhow::bail!("Failed to build crate {err:?}");
        }
    };

    let docker = Docker::connect_with_local_defaults()?;

    let mut tar_data = Vec::new();
    {
        let mut tar = Builder::new(&mut tar_data);

        let dockerfile_content = br#"
                    FROM debian:trixie-slim
                    RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
                    COPY app /app
                    CMD ["/app"]
                "#;
        let mut header = Header::new_gnu();
        header.set_path("Dockerfile")?;
        header.set_size(dockerfile_content.len() as u64);
        header.set_cksum();
        tar.append(&header, &dockerfile_content[..])?;

        let mut header = Header::new_gnu();
        header.set_path("app")?;
        header.set_size(build_output.bin_data.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        tar.append(&header, &build_output.bin_data[..])?;

        tar.finish()?;
    }

    let build_options = BuildImageOptions {
        dockerfile: "Dockerfile".to_owned(),
        t: Some(image_name.to_string()),
        rm: true,
        ..Default::default()
    };

    use bollard::errors::Error;

    let body = http_body_util::Either::Left(Full::new(Bytes::from(tar_data)));
    let mut build_stream = docker.build_image(build_options, None, Some(body));
    while let Some(msg) = build_stream.next().await {
        match msg {
            Ok(_) => {}
            Err(e) => match e {
                Error::DockerStreamError { error } => {
                    return Err(anyhow::anyhow!(
                        "Docker build failed: DockerStreamError: {{ error: {error} }}"
                    ));
                }
                _ => return Err(anyhow::anyhow!("Docker build failed: {}", e)),
            },
        }
    }

    Ok(())
}

#[instrument(level = "trace", skip_all, fields(%image_name))]
async fn upload_image_to_ecr(docker: &Docker, image_name: &str) -> Result<(), anyhow::Error> {
    use aws_config::BehaviorVersion;
    use aws_sdk_ecr::Client as EcrClient;
    use base64::Engine;
    use bollard::auth::DockerCredentials;
    use bollard::query_parameters::PushImageOptions;

    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let ecr_client = EcrClient::new(&config);

    // Create the ECR repository if it doesn't exist
    match ecr_client
        .create_repository()
        .repository_name(image_name)
        .send()
        .await
    {
        Ok(_) => trace!(name: "repository_created", %image_name),
        Err(error) => {
            // Repository might already exist, which is fine
            trace!(name: "repository_creation_result", ?error);
        }
    }

    let auth_response = ecr_client.get_authorization_token().send().await?;

    let auth_token = auth_response
        .authorization_data()
        .first()
        .ok_or_else(|| anyhow::anyhow!("No ECR authorization data"))?;

    let endpoint = auth_token
        .proxy_endpoint()
        .ok_or_else(|| anyhow::anyhow!("No ECR endpoint"))?;
    let token = auth_token
        .authorization_token()
        .ok_or_else(|| anyhow::anyhow!("No ECR token"))?;

    let decoded = String::from_utf8(base64::prelude::BASE64_STANDARD.decode(token)?)?;
    let (username, password) = decoded
        .split_once(':')
        .ok_or_else(|| anyhow::anyhow!("Invalid ECR token format"))?;

    let registry = endpoint.trim_start_matches("https://");
    let ecr_image_name = format!("{registry}/{image_name}");

    // Create the ECR repository if it doesn't exist
    match ecr_client
        .create_repository()
        .repository_name(image_name)
        .send()
        .await
    {
        Ok(_) => {}
        Err(e) => {
            // Ignore "RepositoryAlreadyExistsException" error
            let is_already_exists = e
                .as_service_error()
                .map(|se| se.is_repository_already_exists_exception())
                .unwrap_or(false);
            if !is_already_exists {
                return Err(anyhow::anyhow!("Failed to create ECR repository: {}", e));
            }
        }
    }

    docker
        .tag_image(
            image_name,
            Some(TagImageOptions {
                repo: Some(ecr_image_name.clone()),
                ..Default::default()
            }),
        )
        .await?;

    let mut push_stream = docker.push_image(
        &ecr_image_name,
        Some(PushImageOptions {
            ..Default::default()
        }),
        Some(DockerCredentials {
            username: Some(username.to_string()),
            password: Some(password.to_string()),
            ..Default::default()
        }),
    );

    while let Some(msg) = push_stream.next().await {
        match msg {
            Ok(_) => {}
            Err(e) => match e {
                bollard::errors::Error::DockerStreamError { error } => {
                    return Err(anyhow::anyhow!(
                        "Docker push failed: DockerStreamError: {{ error: {error} }}"
                    ));
                }
                _ => return Err(anyhow::anyhow!("Docker push failed: {}", e)),
            },
        }
        msg?;
    }

    Ok(())
}

impl DockerDeployEcs {
    /// Create a new deployment
    pub fn new(network: DockerNetworkEcs) -> Self {
        Self {
            docker_processes: Vec::new(),
            docker_clusters: Vec::new(),
            network,
            deployment_instance: nanoid!(6, &CONTAINER_ALPHABET),
        }
    }

    /// Add an internal docker service to the deployment.
    pub fn add_localhost_docker(
        &mut self,
        compilation_options: Option<String>,
        config: Vec<String>,
    ) -> DockerDeployProcessSpecEcs {
        let process = DockerDeployProcessSpecEcs {
            compilation_options,
            config,
            network: self.network.clone(),
            deployment_instance: self.deployment_instance.clone(),
        };

        self.docker_processes.push(process.clone());

        process
    }

    /// Add an internal docker cluster to the deployment.
    pub fn add_localhost_docker_cluster(
        &mut self,
        compilation_options: Option<String>,
        config: Vec<String>,
        count: usize,
    ) -> DockerDeployClusterSpecEcs {
        let cluster = DockerDeployClusterSpecEcs {
            compilation_options,
            config,
            count,
            deployment_instance: self.deployment_instance.clone(),
        };

        self.docker_clusters.push(cluster.clone());

        cluster
    }

    /// Add an external process to the deployment.
    pub fn add_external(&self, name: String) -> DockerDeployExternalSpecEcs {
        DockerDeployExternalSpecEcs {
            name,
            deployment_instance: self.deployment_instance.clone(),
        }
    }

    /// Get the deployment instance from this deployment.
    pub fn get_deployment_instance(&self) -> String {
        self.deployment_instance.clone()
    }

    /// Create docker images.
    #[instrument(level = "trace", skip_all)]
    pub async fn provision(&self, nodes: &DeployResult<'_, Self>) -> Result<(), anyhow::Error> {
        for (_, _, process) in nodes.get_all_processes() {
            build_and_create_image(
                &process.rust_crate,
                &process.compilation_options,
                &process.config,
                &process.name,
            )
            .await?;
        }

        for (_, _, cluster) in nodes.get_all_clusters() {
            build_and_create_image(
                &cluster.rust_crate,
                &cluster.compilation_options,
                &cluster.config,
                &cluster.name,
            )
            .await?;
        }

        // upload created docker images to amazon aws ECR
        let docker = Docker::connect_with_local_defaults()?;

        for (_, _, process) in nodes.get_all_processes() {
            upload_image_to_ecr(&docker, &process.name).await?;
        }

        for (_, _, cluster) in nodes.get_all_clusters() {
            upload_image_to_ecr(&docker, &cluster.name).await?;
        }

        Ok(())
    }

    /// Start the deployment, create ECS tasks from the provisioned images.
    #[instrument(level = "trace", skip_all)]
    pub async fn start(&self, nodes: &DeployResult<'_, Self>) -> Result<(), anyhow::Error> {
        use aws_config::BehaviorVersion;
        use aws_sdk_ecs::Client as EcsClient;

        let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
        let region = config.region().unwrap().to_string();

        // Get ECR registry URL
        let ecr_client = aws_sdk_ecr::Client::new(&config);
        let auth_response = ecr_client.get_authorization_token().send().await?;
        let auth_token = auth_response
            .authorization_data()
            .first()
            .ok_or_else(|| anyhow::anyhow!("No ECR authorization data"))?;
        let endpoint = auth_token
            .proxy_endpoint()
            .ok_or_else(|| anyhow::anyhow!("No ECR endpoint"))?;
        let registry = endpoint.trim_start_matches("https://");

        // Build task configurations for CloudFormation template
        let mut tasks = Vec::new();

        for (_, _, process) in nodes.get_all_processes() {
            let task_name = get_docker_container_name(&process.name, None);
            *process.docker_container_name.borrow_mut() = Some(task_name.clone());

            let image_uri = format!("{registry}/{}", process.name);
            let exposed_ports = process.exposed_ports.borrow().clone();

            tasks.push(TaskConfig {
                task_name,
                image_uri,
                deployment_instance: self.deployment_instance.clone(),
                exposed_ports,
                region: region.clone(),
            });
        }

        for (_, _, cluster) in nodes.get_all_clusters() {
            let image_uri = format!("{registry}/{}", cluster.name);

            for num in 0..cluster.count {
                let task_name = get_docker_container_name(&cluster.name, Some(num));
                cluster
                    .docker_container_name
                    .borrow_mut()
                    .push(task_name.clone());

                tasks.push(TaskConfig {
                    task_name,
                    image_uri: image_uri.clone(),
                    deployment_instance: self.deployment_instance.clone(),
                    exposed_ports: vec![],
                    region: region.clone(),
                });
            }
        }

        // Deploy everything via CloudFormation
        let cluster_name = deploy_stack(&self.deployment_instance, &tasks).await?;
        trace!(name: "stack_deployed", %cluster_name);

        // Wait for all services to have running tasks
        let ecs_client = EcsClient::new(&config);
        trace!("waiting_for_services_to_stabilize");

        for task in &tasks {
            trace!(name: "waiting_for_service", service_name = %task.task_name);
            loop {
                let services = ecs_client
                    .describe_services()
                    .cluster(&cluster_name)
                    .services(&task.task_name)
                    .send()
                    .await?;

                if let Some(service) = services.services().first() {
                    let desired = service.desired_count();
                    let running = service.running_count();
                    if running >= desired && desired > 0 {
                        trace!(name: "service_running", service_name = %task.task_name, %running, %desired);
                        break;
                    }
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        }

        Ok(())
    }

    /// Stop the deployment, destroy all containers
    #[instrument(level = "trace", skip_all)]
    pub async fn stop(&mut self, nodes: &DeployResult<'_, Self>) -> Result<(), anyhow::Error> {
        use aws_config::BehaviorVersion;
        use aws_sdk_ecs::Client as EcsClient;

        let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
        let ecs_client = EcsClient::new(&config);
        let cluster_name = format!("hydro-{}", self.deployment_instance);

        // Stop ECS services by setting desired count to 0
        for (_, _, process) in nodes.get_all_processes() {
            let service_name = get_docker_container_name(&process.name, None);
            let _ = ecs_client
                .update_service()
                .cluster(&cluster_name)
                .service(&service_name)
                .desired_count(0)
                .send()
                .await;
        }

        for (_, _, cluster) in nodes.get_all_clusters() {
            for num in 0..cluster.count {
                let service_name = get_docker_container_name(&cluster.name, Some(num));
                let _ = ecs_client
                    .update_service()
                    .cluster(&cluster_name)
                    .service(&service_name)
                    .desired_count(0)
                    .send()
                    .await;
            }
        }

        Ok(())
    }

    /// remove containers, images, and networks.
    #[instrument(level = "trace", skip_all)]
    pub async fn cleanup(&mut self, nodes: &DeployResult<'_, Self>) -> Result<(), anyhow::Error> {
        use aws_config::BehaviorVersion;
        use aws_sdk_cloudformation::types::StackStatus;

        let config = aws_config::load_defaults(BehaviorVersion::latest()).await;

        // Delete CloudFormation stack first (handles VPC, IAM, ECS, Cloud Map)
        let cfn_client = aws_sdk_cloudformation::Client::new(&config);
        let stack_name = format!("hydro-{}", self.deployment_instance);
        let _ = cfn_client
            .delete_stack()
            .stack_name(&stack_name)
            .send()
            .await;
        trace!(name: "stack_deletion_initiated", %stack_name);

        // Wait for stack deletion to complete
        while let Ok(resp) = cfn_client
            .describe_stacks()
            .stack_name(&stack_name)
            .send()
            .await
        {
            if let Some(stack) = resp.stacks().first() {
                match stack.stack_status() {
                    Some(StackStatus::DeleteComplete) => break,
                    Some(StackStatus::DeleteFailed) => {
                        trace!(name: "stack_deletion_failed", %stack_name);
                        break;
                    }
                    _ => {
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    }
                }
            } else {
                break;
            }
        }
        trace!(name: "stack_deleted", %stack_name);

        // Delete ECR repositories after stack is deleted
        let ecr_client = aws_sdk_ecr::Client::new(&config);
        for (_, _, process) in nodes.get_all_processes() {
            let _ = ecr_client
                .delete_repository()
                .repository_name(&process.name)
                .force(true)
                .send()
                .await;
        }
        for (_, _, cluster) in nodes.get_all_clusters() {
            let _ = ecr_client
                .delete_repository()
                .repository_name(&cluster.name)
                .force(true)
                .send()
                .await;
        }

        Ok(())
    }
}

impl<'a> Deploy<'a> for DockerDeployEcs {
    type Meta = ();
    type InstantiateEnv = Self;

    type Process = DockerDeployProcessEcs;
    type Cluster = DockerDeployClusterEcs;
    type External = DockerDeployExternalEcs;

    #[instrument(level = "trace", skip_all, fields(p1 = p1.name, %p1_port, p2 = p2.name, p2_port))]
    fn o2o_sink_source(
        p1: &Self::Process,
        p1_port: &<Self::Process as Node>::Port,
        p2: &Self::Process,
        p2_port: &<Self::Process as Node>::Port,
    ) -> (syn::Expr, syn::Expr) {
        // Pass container name for ECS API resolution
        deploy_containerized_o2o(&p2.name, *p2_port)
    }

    #[instrument(level = "trace", skip_all, fields(p1 = p1.name, %p1_port, p2 = p2.name, p2_port))]
    fn o2o_connect(
        p1: &Self::Process,
        p1_port: &<Self::Process as Node>::Port,
        p2: &Self::Process,
        p2_port: &<Self::Process as Node>::Port,
    ) -> Box<dyn FnOnce()> {
        let serialized = format!(
            "o2o_connect {}:{p1_port:?} -> {}:{p2_port:?}",
            p1.name, p2.name
        );

        Box::new(move || {
            trace!(name: "o2o_connect thunk", %serialized);
        })
    }

    #[instrument(level = "trace", skip_all, fields(p1 = p1.name, %p1_port, c2 = c2.name, %c2_port))]
    fn o2m_sink_source(
        p1: &Self::Process,
        p1_port: &<Self::Process as Node>::Port,
        c2: &Self::Cluster,
        c2_port: &<Self::Cluster as Node>::Port,
    ) -> (syn::Expr, syn::Expr) {
        deploy_containerized_o2m(*c2_port)
    }

    #[instrument(level = "trace", skip_all, fields(p1 = p1.name, %p1_port, c2 = c2.name, %c2_port))]
    fn o2m_connect(
        p1: &Self::Process,
        p1_port: &<Self::Process as Node>::Port,
        c2: &Self::Cluster,
        c2_port: &<Self::Cluster as Node>::Port,
    ) -> Box<dyn FnOnce()> {
        let serialized = format!(
            "o2m_connect {}:{p1_port:?} -> {}:{c2_port:?}",
            p1.name, c2.name
        );

        Box::new(move || {
            trace!(name: "o2m_connect thunk", %serialized);
        })
    }

    #[instrument(level = "trace", skip_all, fields(c1 = c1.name, %c1_port, p2 = p2.name, %p2_port))]
    fn m2o_sink_source(
        c1: &Self::Cluster,
        c1_port: &<Self::Cluster as Node>::Port,
        p2: &Self::Process,
        p2_port: &<Self::Process as Node>::Port,
    ) -> (syn::Expr, syn::Expr) {
        // Pass container name for ECS API resolution
        deploy_containerized_m2o(*p2_port, &p2.name)
    }

    #[instrument(level = "trace", skip_all, fields(c1 = c1.name, %c1_port, p2 = p2.name, %p2_port))]
    fn m2o_connect(
        c1: &Self::Cluster,
        c1_port: &<Self::Cluster as Node>::Port,
        p2: &Self::Process,
        p2_port: &<Self::Process as Node>::Port,
    ) -> Box<dyn FnOnce()> {
        let serialized = format!(
            "o2m_connect {}:{c1_port:?} -> {}:{p2_port:?}",
            c1.name, p2.name
        );

        Box::new(move || {
            trace!(name: "m2o_connect thunk", %serialized);
        })
    }

    #[instrument(level = "trace", skip_all, fields(c1 = c1.name, %c1_port, c2 = c2.name, %c2_port))]
    fn m2m_sink_source(
        c1: &Self::Cluster,
        c1_port: &<Self::Cluster as Node>::Port,
        c2: &Self::Cluster,
        c2_port: &<Self::Cluster as Node>::Port,
    ) -> (syn::Expr, syn::Expr) {
        deploy_containerized_m2m(*c2_port)
    }

    #[instrument(level = "trace", skip_all, fields(c1 = c1.name, %c1_port, c2 = c2.name, %c2_port))]
    fn m2m_connect(
        c1: &Self::Cluster,
        c1_port: &<Self::Cluster as Node>::Port,
        c2: &Self::Cluster,
        c2_port: &<Self::Cluster as Node>::Port,
    ) -> Box<dyn FnOnce()> {
        let serialized = format!(
            "m2m_connect {}:{c1_port:?} -> {}:{c2_port:?}",
            c1.name, c2.name
        );

        Box::new(move || {
            trace!(name: "m2m_connect thunk", %serialized);
        })
    }

    #[instrument(level = "trace", skip_all, fields(p2 = p2.name, %p2_port, ?codec_type, %shared_handle, extra_stmts = extra_stmts.len()))]
    fn e2o_many_source(
        extra_stmts: &mut Vec<syn::Stmt>,
        p2: &Self::Process,
        p2_port: &<Self::Process as Node>::Port,
        codec_type: &syn::Type,
        shared_handle: String,
    ) -> syn::Expr {
        todo!()
    }

    #[instrument(level = "trace", skip_all, fields(%shared_handle))]
    fn e2o_many_sink(shared_handle: String) -> syn::Expr {
        todo!()
    }

    #[instrument(level = "trace", skip_all, fields(p1 = p1.name, %p1_port, p2 = p2.name, %p2_port, ?codec_type, %shared_handle))]
    fn e2o_source(
        extra_stmts: &mut Vec<syn::Stmt>,
        p1: &Self::External,
        p1_port: &<Self::External as Node>::Port,
        p2: &Self::Process,
        p2_port: &<Self::Process as Node>::Port,
        codec_type: &syn::Type,
        shared_handle: String,
    ) -> syn::Expr {
        p1.connection_info.borrow_mut().insert(
            *p1_port,
            (
                p2.docker_container_name.clone(),
                *p2_port,
                p2.network.clone(),
            ),
        );

        p2.exposed_ports.borrow_mut().push(*p2_port);

        let socket_ident = syn::Ident::new(
            &format!("__hydro_deploy_{}_socket", &shared_handle),
            Span::call_site(),
        );

        let source_ident = syn::Ident::new(
            &format!("__hydro_deploy_{}_source", &shared_handle),
            Span::call_site(),
        );

        let sink_ident = syn::Ident::new(
            &format!("__hydro_deploy_{}_sink", &shared_handle),
            Span::call_site(),
        );

        let bind_addr = format!("0.0.0.0:{}", p2_port);

        extra_stmts.push(syn::parse_quote! {
            let #socket_ident = tokio::net::TcpListener::bind(#bind_addr).await.unwrap();
        });

        let create_expr =
            deploy_containerized_external_sink_source_ident(bind_addr, socket_ident.clone());

        extra_stmts.push(syn::parse_quote! {
            let (#sink_ident, #source_ident) = (#create_expr).split();
        });

        parse_quote!(#source_ident)
    }

    #[instrument(level = "trace", skip_all, fields(p1 = p1.name, %p1_port, p2 = p2.name, %p2_port, ?many, ?server_hint))]
    fn e2o_connect(
        p1: &Self::External,
        p1_port: &<Self::External as Node>::Port,
        p2: &Self::Process,
        p2_port: &<Self::Process as Node>::Port,
        many: bool,
        server_hint: NetworkHint,
    ) -> Box<dyn FnOnce()> {
        let serialized = format!(
            "e2o_connect {}:{p1_port:?} -> {}:{p2_port:?}",
            p1.name, p2.name
        );

        Box::new(move || {
            trace!(name: "e2o_connect thunk", %serialized);
        })
    }

    #[instrument(level = "trace", skip_all, fields(p1 = p1.name, %p1_port, p2 = p2.name, %p2_port, %shared_handle))]
    fn o2e_sink(
        p1: &Self::Process,
        p1_port: &<Self::Process as Node>::Port,
        p2: &Self::External,
        p2_port: &<Self::External as Node>::Port,
        shared_handle: String,
    ) -> syn::Expr {
        let sink_ident = syn::Ident::new(
            &format!("__hydro_deploy_{}_sink", &shared_handle),
            Span::call_site(),
        );
        parse_quote!(#sink_ident)
    }

    #[instrument(level = "trace", skip_all, fields(%of_cluster))]
    fn cluster_ids(
        of_cluster: usize,
    ) -> impl QuotedWithContext<'a, &'a [TaglessMemberId], ()> + Clone + 'a {
        cluster_ids()
    }

    #[instrument(level = "trace", skip_all)]
    fn cluster_self_id() -> impl QuotedWithContext<'a, TaglessMemberId, ()> + Clone + 'a {
        cluster_self_id()
    }

    #[instrument(level = "trace", skip_all, fields(?location_id))]
    fn cluster_membership_stream(
        location_id: &LocationId,
    ) -> impl QuotedWithContext<'a, Box<dyn Stream<Item = (TaglessMemberId, MembershipEvent)> + Unpin>, ()>
    {
        cluster_membership_stream(location_id)
    }
}

const CONTAINER_ALPHABET: [char; 36] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i',
    'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
];

#[instrument(level = "trace", skip_all, ret, fields(%name_hint, %location, %deployment_instance))]
fn get_docker_image_name(name_hint: &str, location: usize, deployment_instance: &str) -> String {
    let name_hint = name_hint
        .split("::")
        .last()
        .unwrap()
        .to_string()
        .to_ascii_lowercase()
        .replace(".", "-")
        .replace("_", "-")
        .replace("::", "-");

    let image_unique_tag = nanoid::nanoid!(6, &CONTAINER_ALPHABET);

    format!("hy-{name_hint}-{image_unique_tag}-{deployment_instance}-{location}")
}

#[instrument(level = "trace", skip_all, ret, fields(%image_name, ?instance))]
fn get_docker_container_name(image_name: &str, instance: Option<usize>) -> String {
    if let Some(instance) = instance {
        format!("{image_name}-{instance}")
    } else {
        image_name.to_string()
    }
}
/// Represents a Process running in a docker container
#[derive(Clone)]
pub struct DockerDeployProcessSpecEcs {
    compilation_options: Option<String>,
    config: Vec<String>,
    network: DockerNetworkEcs,
    deployment_instance: String,
}

impl<'a> ProcessSpec<'a, DockerDeployEcs> for DockerDeployProcessSpecEcs {
    #[instrument(level = "trace", skip_all, fields(%id, %name_hint))]
    fn build(self, id: usize, name_hint: &'_ str) -> <DockerDeployEcs as Deploy<'a>>::Process {
        DockerDeployProcessEcs {
            id,
            name: get_docker_image_name(name_hint, id, &self.deployment_instance),

            next_port: Rc::new(RefCell::new(1000)),
            rust_crate: Rc::new(RefCell::new(None)),

            exposed_ports: Rc::new(RefCell::new(Vec::new())),

            docker_container_name: Rc::new(RefCell::new(None)),

            compilation_options: self.compilation_options,
            config: self.config,

            network: self.network.clone(),
        }
    }
}

/// Represents a Cluster running across `count` docker containers.
#[derive(Clone)]
pub struct DockerDeployClusterSpecEcs {
    compilation_options: Option<String>,
    config: Vec<String>,
    count: usize,
    deployment_instance: String,
}

impl<'a> ClusterSpec<'a, DockerDeployEcs> for DockerDeployClusterSpecEcs {
    #[instrument(level = "trace", skip_all, fields(%id, %name_hint))]
    fn build(self, id: usize, name_hint: &str) -> <DockerDeployEcs as Deploy<'a>>::Cluster {
        DockerDeployClusterEcs {
            id,
            name: get_docker_image_name(name_hint, id, &self.deployment_instance),

            next_port: Rc::new(RefCell::new(1000)),
            rust_crate: Rc::new(RefCell::new(None)),

            docker_container_name: Rc::new(RefCell::new(Vec::new())),

            compilation_options: self.compilation_options,
            config: self.config,

            count: self.count,
        }
    }
}

/// Represents an external process outside of the management of hydro deploy.
pub struct DockerDeployExternalSpecEcs {
    name: String,
    deployment_instance: String,
}

impl<'a> ExternalSpec<'a, DockerDeployEcs> for DockerDeployExternalSpecEcs {
    #[instrument(level = "trace", skip_all, fields(%id, %name_hint))]
    fn build(self, id: usize, name_hint: &str) -> <DockerDeployEcs as Deploy<'a>>::External {
        DockerDeployExternalEcs {
            name: self.name,
            next_port: Rc::new(RefCell::new(10000)),
            ports: Rc::new(RefCell::new(HashMap::new())),
            connection_info: Rc::new(RefCell::new(HashMap::new())),
            deployment_instance: self.deployment_instance,
        }
    }
}
