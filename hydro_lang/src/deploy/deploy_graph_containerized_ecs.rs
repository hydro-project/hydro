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
use sinktools::lazy::LazySink;
use stageleft::QuotedWithContext;
use syn::parse_quote;
use tar::{Builder, Header};
use tokio::net::TcpStream;
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};
use tracing::{Instrument, instrument, trace};

use super::deploy_runtime_containerized_ecs::*;
use crate::compile::deploy::DeployResult;
use crate::compile::deploy_provider::{
    ClusterSpec, Deploy, ExternalSpec, Node, ProcessSpec, RegisterPort,
};
use crate::compile::trybuild::generate::create_graph_trybuild;
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

    deployment_instance: String,
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
    fn update_meta(&mut self, _meta: &Self::Meta) {}

    #[instrument(level = "trace", skip_all, fields(id = self.id, name = self.name, ?meta, extra_stmts = extra_stmts.len()))]
    fn instantiate(
        &self,
        _env: &mut Self::InstantiateEnv,
        meta: &mut Self::Meta,
        graph: DfirGraph,
        extra_stmts: Vec<syn::Stmt>,
    ) {
        let (bin_name, config) =
            create_graph_trybuild(graph, extra_stmts.clone(), &Some(self.name.clone()), true);

        let mut ret = RustCrate::new(config.project_dir)
            .target_dir(config.target_dir)
            .example(bin_name.clone())
            .no_default_features();

        ret = ret.display_name("test_display_name");

        ret = ret.features(vec!["hydro___feature_docker_runtime".to_string()]);

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
    fn update_meta(&mut self, _meta: &Self::Meta) {}

    #[instrument(level = "trace", skip_all, fields(id = self.id, name = self.name, extra_stmts = extra_stmts.len()))]
    fn instantiate(
        &self,
        _env: &mut Self::InstantiateEnv,
        _meta: &mut Self::Meta,
        graph: DfirGraph,
        extra_stmts: Vec<syn::Stmt>,
    ) {
        let (bin_name, config) =
            create_graph_trybuild(graph, extra_stmts.clone(), &Some(self.name.clone()), true);

        let mut ret = RustCrate::new(config.project_dir)
            .target_dir(config.target_dir)
            .example(bin_name.clone())
            .no_default_features();

        ret = ret.display_name("test_display_name");

        ret = ret.features(vec!["hydro___feature_deploy_integration".to_string()]);

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

    ports: Rc<RefCell<HashMap<usize, u16>>>,

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
    fn update_meta(&mut self, _meta: &Self::Meta) {}

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
    #[instrument(level = "trace", skip_all, fields(name = self.name, %key, %port))]
    fn register(&self, key: usize, port: <DockerDeployEcs as Deploy>::Port) {
        self.ports.borrow_mut().insert(key, port);
    }

    #[instrument(level = "trace", skip_all, fields(name = self.name, %key))]
    fn raw_port(&self, key: usize) -> <DockerDeployEcs as Deploy<'a>>::ExternalRawPort {
        todo!()
    }

    fn as_bytes_bidi(
        &self,
        key: usize,
    ) -> impl Future<
        Output = DynSourceSink<Result<bytes::BytesMut, std::io::Error>, Bytes, std::io::Error>,
    > + 'a {
        let _span = tracing::trace_span!("as_bytes_bidi", name = %self.name, %key).entered(); // the instrument macro doesn't work here because of lifetime issues?
        async { todo!() }
    }

    fn as_bincode_bidi<InT, OutT>(
        &self,
        key: usize,
    ) -> impl Future<Output = DynSourceSink<OutT, InT, std::io::Error>> + 'a
    where
        InT: serde::Serialize + 'static,
        OutT: serde::de::DeserializeOwned + 'static,
    {
        let _span = tracing::trace_span!("as_bincode_bidi", name = %self.name, %key).entered(); // the instrument macro doesn't work here because of lifetime issues?
        async { todo!() }
    }

    fn as_bincode_sink<T>(
        &self,
        key: usize,
    ) -> impl Future<Output = Pin<Box<dyn Sink<T, Error = std::io::Error>>>> + 'a
    where
        T: serde::Serialize + 'static,
    {
        let guard = tracing::trace_span!("as_bincode_sink", name = %self.name, %key).entered();

        let local_port = *self.ports.borrow().get(&key).unwrap();
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
        key: usize,
    ) -> impl Future<Output = Pin<Box<dyn Stream<Item = T>>>> + 'a
    where
        T: serde::de::DeserializeOwned + 'static,
    {
        let guard = tracing::trace_span!("as_bincode_sink", name = %self.name, %key).entered();

        let local_port = *self.ports.borrow().get(&key).unwrap();
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

// #[instrument(level = "trace", skip_all, fields(%image_name, %container_name, %network_name, %deployment_instance, ?exposed_ports))]
// async fn create_and_start_container(
//     docker: &Docker,
//     container_name: &str,
//     image_name: &str,
//     network_name: &str,
//     deployment_instance: &str,
//     exposed_ports: Option<HashMap<String, HashMap<(), ()>>>,
// ) -> Result<(), anyhow::Error> {
//     let config = Config {
//         image: Some(image_name.to_string()),
//         hostname: Some(container_name.to_string()),
//         host_config: Some(HostConfig {
//             binds: Some(vec![
//                 "/var/run/docker.sock:/var/run/docker.sock".to_string(),
//             ]),
//             ..Default::default()
//         }),
//         env: Some(vec![
//             format!("CONTAINER_NAME={container_name}"),
//             format!("DEPLOYMENT_INSTANCE={deployment_instance}"),
//             format!("RUST_LOG=trace"),
//         ]),
//         exposed_ports,
//         networking_config: Some(NetworkingConfig {
//             endpoints_config: HashMap::from([(
//                 network_name.to_string(),
//                 EndpointSettings {
//                     ..Default::default()
//                 },
//             )]),
//         }),
//         tty: Some(true),
//         ..Default::default()
//     };

//     let options = CreateContainerOptions {
//         name: container_name.to_string(),
//         platform: None,
//     };

//     docker.create_container(Some(options), config).await?;
//     docker
//         .start_container(container_name, None::<StartContainerOptions<String>>)
//         .await?;

//     Ok(())
// }

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
        rust_crate.get_build_params(hydro_deploy::HostTargetType::Linux(LinuxCompileType::Musl)),
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
                    FROM alpine:latest
                    RUN apk add --no-cache ca-certificates
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
        use aws_sdk_ecs::types::{
            AwsVpcConfiguration, ContainerDefinition, NetworkConfiguration, ServiceRegistry,
        };
        use aws_sdk_servicediscovery::Client as ServiceDiscoveryClient;

        let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
        let ecs_client = EcsClient::new(&config);

        // Get AWS account ID
        let sts_client = aws_sdk_sts::Client::new(&config);
        let identity = sts_client.get_caller_identity().send().await?;
        let account_id = identity
            .account()
            .ok_or_else(|| anyhow::anyhow!("No account ID"))?;

        // Create ECS task execution role
        let iam_client = aws_sdk_iam::Client::new(&config);
        let execution_role_name = "ecsTaskExecutionRole";
        let trust_policy = r#"{"Version":"2012-10-17","Statement":[{"Effect":"Allow","Principal":{"Service":"ecs-tasks.amazonaws.com"},"Action":"sts:AssumeRole"}]}"#;

        let execution_role_arn = match iam_client
            .create_role()
            .role_name(execution_role_name)
            .assume_role_policy_document(trust_policy)
            .send()
            .await
        {
            Ok(resp) => resp.role().unwrap().arn().to_string(),
            Err(_) => format!("arn:aws:iam::{account_id}:role/{execution_role_name}"),
        };

        let _ = iam_client
            .attach_role_policy()
            .role_name(execution_role_name)
            .policy_arn("arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy")
            .send()
            .await;

        let logs_policy = r#"{
            "Version": "2012-10-17",
            "Statement": [{
                "Effect": "Allow",
                "Action": "logs:CreateLogGroup",
                "Resource": "*"
            }]
        }"#;
        let _ = iam_client
            .put_role_policy()
            .role_name(execution_role_name)
            .policy_name("CloudWatchLogsCreateLogGroup")
            .policy_document(logs_policy)
            .send()
            .await;

        // Create task role for ECS API access
        let task_role_name = "hydroEcsTaskRole";
        let task_role_arn = match iam_client
            .create_role()
            .role_name(task_role_name)
            .assume_role_policy_document(trust_policy)
            .send()
            .await
        {
            Ok(resp) => resp.role().unwrap().arn().to_string(),
            Err(_) => format!("arn:aws:iam::{account_id}:role/{task_role_name}"),
        };

        let task_policy = r#"{
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
        }"#;
        let _ = iam_client
            .put_role_policy()
            .role_name(task_role_name)
            .policy_name("HydroEcsAccess")
            .policy_document(task_policy)
            .send()
            .await;

        // Wait for IAM role to propagate
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

        // Create Cloud Map namespace for service discovery
        let sd_client = ServiceDiscoveryClient::new(&config);
        let namespace_name = format!("hydro-{}.local", self.deployment_instance);

        // Create ECS service-linked role if it doesn't exist
        match iam_client
            .create_service_linked_role()
            .aws_service_name("ecs.amazonaws.com")
            .send()
            .await
        {
            Ok(_) => {}
            Err(_) => {} // Role might already exist
        }

        // Create ECS cluster based on deployment instance
        let cluster_name = format!("hydro-{}", self.deployment_instance);
        match ecs_client
            .create_cluster()
            .cluster_name(&cluster_name)
            .send()
            .await
        {
            Ok(_) => trace!(name: "created_cluster", %cluster_name),
            Err(_) => trace!(name: "cluster_exists", %cluster_name),
        };

        // Create VPC and subnet
        let ec2_client = aws_sdk_ec2::Client::new(&config);
        trace!("creating_vpc");
        let vpc = ec2_client
            .create_vpc()
            .cidr_block("10.0.0.0/16")
            .send()
            .await?;
        let vpc_id = vpc.vpc().and_then(|v| v.vpc_id()).unwrap();
        trace!(name: "created_vpc", %vpc_id);

        // Enable DNS support
        trace!("enabling_dns");
        ec2_client
            .modify_vpc_attribute()
            .vpc_id(vpc_id)
            .enable_dns_support(
                aws_sdk_ec2::types::AttributeBooleanValue::builder()
                    .value(true)
                    .build(),
            )
            .send()
            .await?;
        ec2_client
            .modify_vpc_attribute()
            .vpc_id(vpc_id)
            .enable_dns_hostnames(
                aws_sdk_ec2::types::AttributeBooleanValue::builder()
                    .value(true)
                    .build(),
            )
            .send()
            .await?;

        // Create Cloud Map private DNS namespace
        trace!(name: "creating_namespace", %namespace_name);
        let namespace_result = sd_client
            .create_private_dns_namespace()
            .name(&namespace_name)
            .vpc(vpc_id)
            .send()
            .await?;

        let operation_id = namespace_result.operation_id().unwrap();

        // Wait for namespace creation to complete
        let namespace_id = loop {
            let op = sd_client
                .get_operation()
                .operation_id(operation_id)
                .send()
                .await?;

            let operation = op.operation().unwrap();
            match operation.status() {
                Some(aws_sdk_servicediscovery::types::OperationStatus::Success) => {
                    let ns_id = operation
                        .targets()
                        .and_then(|t| {
                            t.get(&aws_sdk_servicediscovery::types::OperationTargetType::Namespace)
                        })
                        .map(|s| s.to_string())
                        .ok_or_else(|| anyhow::anyhow!("No namespace ID in operation result"))?;
                    break ns_id;
                }
                Some(aws_sdk_servicediscovery::types::OperationStatus::Fail) => {
                    return Err(anyhow::anyhow!("Failed to create namespace"));
                }
                _ => {
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                }
            }
        };
        trace!(name: "created_namespace", %namespace_id);

        trace!("creating_subnet");
        let subnet = ec2_client
            .create_subnet()
            .vpc_id(vpc_id)
            .cidr_block("10.0.1.0/24")
            .send()
            .await?;
        let subnet_id = subnet.subnet().and_then(|s| s.subnet_id()).unwrap();
        trace!(name: "created_subnet", %subnet_id);

        // Create internet gateway
        trace!("creating_igw");
        let igw = ec2_client.create_internet_gateway().send().await?;
        let igw_id = igw
            .internet_gateway()
            .and_then(|i| i.internet_gateway_id())
            .unwrap();
        trace!(name: "created_igw", %igw_id);
        ec2_client
            .attach_internet_gateway()
            .vpc_id(vpc_id)
            .internet_gateway_id(igw_id)
            .send()
            .await?;
        trace!("attached_igw");

        // Create route table and add route to internet gateway
        trace!("creating_route_table");
        let route_table = ec2_client
            .create_route_table()
            .vpc_id(vpc_id)
            .send()
            .await?;
        let route_table_id = route_table
            .route_table()
            .and_then(|r| r.route_table_id())
            .unwrap();
        trace!(name: "created_route_table", %route_table_id);
        ec2_client
            .create_route()
            .route_table_id(route_table_id)
            .destination_cidr_block("0.0.0.0/0")
            .gateway_id(igw_id)
            .send()
            .await?;
        trace!("created_route");
        ec2_client
            .associate_route_table()
            .route_table_id(route_table_id)
            .subnet_id(subnet_id)
            .send()
            .await?;
        trace!("associated_route_table");

        // Create security group allowing all inbound traffic
        trace!("creating_security_group");
        let sg = ec2_client
            .create_security_group()
            .group_name("hydro-ecs-sg")
            .description("Security group for Hydro ECS tasks")
            .vpc_id(vpc_id)
            .send()
            .await?;
        let sg_id = sg.group_id().unwrap();
        trace!(name: "created_security_group", %sg_id);

        ec2_client
            .authorize_security_group_ingress()
            .group_id(sg_id)
            .ip_protocol("-1")
            .cidr_ip("0.0.0.0/0")
            .send()
            .await?;
        trace!("authorized_ingress");

        let subnets = vec![subnet_id.to_string()];
        let security_groups = vec![sg_id.to_string()];

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

        let log_group_name = "/ecs/hydro";

        for (_, _, process) in nodes.get_all_processes() {
            let task_name = get_docker_container_name(&process.name, None);
            *process.docker_container_name.borrow_mut() = Some(task_name.clone());

            let image_uri = format!("{registry}/{}", process.name);
            trace!(name: "configuring_process_task", %task_name, %image_uri);

            let mut container_def = ContainerDefinition::builder()
                .name(&task_name)
                .image(image_uri)
                .log_configuration(
                    aws_sdk_ecs::types::LogConfiguration::builder()
                        .log_driver(aws_sdk_ecs::types::LogDriver::Awslogs)
                        .options("awslogs-group", log_group_name)
                        .options("awslogs-region", config.region().unwrap().to_string())
                        .options("awslogs-stream-prefix", "ecs")
                        .options("awslogs-create-group", "true")
                        .build()
                        .unwrap(),
                )
                .environment(
                    aws_sdk_ecs::types::KeyValuePair::builder()
                        .name("CONTAINER_NAME")
                        .value(&task_name)
                        .build(),
                )
                .environment(
                    aws_sdk_ecs::types::KeyValuePair::builder()
                        .name("DEPLOYMENT_INSTANCE")
                        .value(&self.deployment_instance)
                        .build(),
                )
                .environment(
                    aws_sdk_ecs::types::KeyValuePair::builder()
                        .name("RUST_LOG")
                        .value("trace,aws_runtime=info,aws_sdk_ecs=info,aws_smithy_runtime=info,aws_smithy_runtime_api=info,aws_config=info,hyper_util=info,aws_smithy_http_client=info,aws_sigv4=info")
                        .build(),
                )
                .environment(
                    aws_sdk_ecs::types::KeyValuePair::builder()
                        .name("RUST_BACKTRACE")
                        .value("1")
                        .build(),
                )
                .environment(
                    aws_sdk_ecs::types::KeyValuePair::builder()
                        .name("NO_COLOR") // color codes seem kinda broken in ecs?
                        .value("1")
                        .build(),
                );

            // Add port mappings for exposed ports
            let exposed_ports = process.exposed_ports.borrow();
            if !exposed_ports.is_empty() {
                trace!(name: "adding_port_mappings", ports = ?exposed_ports.as_slice());
            }
            for port in exposed_ports.iter() {
                container_def = container_def.port_mappings(
                    aws_sdk_ecs::types::PortMapping::builder()
                        .container_port(*port as i32)
                        .protocol(aws_sdk_ecs::types::TransportProtocol::Tcp)
                        .build(),
                );
            }

            // Register task definition
            trace!(name: "registering_task_definition", %task_name);
            ecs_client
                .register_task_definition()
                .family(&task_name)
                .requires_compatibilities(aws_sdk_ecs::types::Compatibility::Fargate)
                .network_mode(aws_sdk_ecs::types::NetworkMode::Awsvpc)
                .cpu("256")
                .memory("512")
                .execution_role_arn(&execution_role_arn)
                .task_role_arn(&task_role_arn)
                .container_definitions(container_def.build())
                .send()
                .await?;

            // This looks to be broken for now? Seems as though the containers can't talk to ECR so they can't start up without a public ip address.
            // // Run task - only assign public IP if task has exposed ports
            // let assign_public_ip = if exposed_ports.is_empty() {
            //     aws_sdk_ecs::types::AssignPublicIp::Disabled
            // } else {
            //     aws_sdk_ecs::types::AssignPublicIp::Enabled
            // };
            let assign_public_ip = aws_sdk_ecs::types::AssignPublicIp::Enabled;

            // Create Cloud Map service for this task
            trace!(name: "creating_sd_service", %task_name);
            let sd_service = sd_client
                .create_service()
                .name(&task_name)
                .namespace_id(&namespace_id)
                .dns_config(
                    aws_sdk_servicediscovery::types::DnsConfig::builder()
                        .routing_policy(aws_sdk_servicediscovery::types::RoutingPolicy::Multivalue)
                        .dns_records(
                            aws_sdk_servicediscovery::types::DnsRecord::builder()
                                .r#type(aws_sdk_servicediscovery::types::RecordType::A)
                                .ttl(10)
                                .build()
                                .unwrap(),
                        )
                        .build()
                        .unwrap(),
                )
                .send()
                .await?;
            let sd_service_arn = sd_service.service().unwrap().arn().unwrap().to_string();
            trace!(name: "created_sd_service", %sd_service_arn);

            // Create ECS service (instead of run_task) to enable service discovery
            trace!(name: "creating_ecs_service", %task_name, ?assign_public_ip);
            ecs_client
                .create_service()
                .cluster(&cluster_name)
                .service_name(&task_name)
                .task_definition(&task_name)
                .desired_count(1)
                .launch_type(aws_sdk_ecs::types::LaunchType::Fargate)
                .network_configuration(
                    NetworkConfiguration::builder()
                        .awsvpc_configuration(
                            AwsVpcConfiguration::builder()
                                .set_subnets(Some(subnets.clone()))
                                .set_security_groups(Some(security_groups.clone()))
                                .assign_public_ip(assign_public_ip)
                                .build()?,
                        )
                        .build(),
                )
                .service_registries(
                    ServiceRegistry::builder()
                        .registry_arn(&sd_service_arn)
                        .build(),
                )
                .send()
                .await?;
            trace!(name: "service_created", %task_name);
        }

        for (_, _, cluster) in nodes.get_all_clusters() {
            let image_uri = format!("{registry}/{}", cluster.name);

            for num in 0..cluster.count {
                let task_name = get_docker_container_name(&cluster.name, Some(num));
                cluster
                    .docker_container_name
                    .borrow_mut()
                    .push(task_name.clone());

                trace!(name: "registering_cluster_task", %task_name, %num);
                ecs_client
                    .register_task_definition()
                    .family(&task_name)
                    .requires_compatibilities(aws_sdk_ecs::types::Compatibility::Fargate)
                    .network_mode(aws_sdk_ecs::types::NetworkMode::Awsvpc)
                    .cpu("256")
                    .memory("512")
                    .execution_role_arn(&execution_role_arn)
                    .task_role_arn(&task_role_arn)
                    .container_definitions(
                        ContainerDefinition::builder()
                            .name(&task_name)
                            .image(&image_uri)
                            .log_configuration(
                                aws_sdk_ecs::types::LogConfiguration::builder()
                                    .log_driver(aws_sdk_ecs::types::LogDriver::Awslogs)
                                    .options("awslogs-group", log_group_name)
                                    .options("awslogs-region", config.region().unwrap().to_string())
                                    .options("awslogs-stream-prefix", "ecs")
                                    .options("awslogs-create-group", "true")
                                    .build()
                                    .unwrap(),
                            )
                            .environment(
                                aws_sdk_ecs::types::KeyValuePair::builder()
                                    .name("CONTAINER_NAME")
                                    .value(&task_name)
                                    .build(),
                            )
                            .environment(
                                aws_sdk_ecs::types::KeyValuePair::builder()
                                    .name("DEPLOYMENT_INSTANCE")
                                    .value(&self.deployment_instance)
                                    .build(),
                            )
                            .environment(
                                aws_sdk_ecs::types::KeyValuePair::builder()
                                    .name("RUST_LOG")
                                    .value("trace,aws_runtime=info,aws_sdk_ecs=info,aws_smithy_runtime=info,aws_smithy_runtime_api=info,aws_config=info,hyper_util=info,aws_smithy_http_client=info,aws_sigv4=info")
                                    .build(),
                            )
                            .environment(
                                aws_sdk_ecs::types::KeyValuePair::builder()
                                    .name("RUST_BACKTRACE")
                                    .value("1")
                                    .build(),
                            )
                            .environment(
                                aws_sdk_ecs::types::KeyValuePair::builder()
                                    .name("NO_COLOR") // color codes seem kinda broken in ecs?
                                    .value("1")
                                    .build(),
                            )
                            .build(),
                    )
                    .send()
                    .await?;

                // Create Cloud Map service for this cluster task
                trace!(name: "creating_cluster_sd_service", %task_name);
                let sd_service = sd_client
                    .create_service()
                    .name(&task_name)
                    .namespace_id(&namespace_id)
                    .dns_config(
                        aws_sdk_servicediscovery::types::DnsConfig::builder()
                            .routing_policy(
                                aws_sdk_servicediscovery::types::RoutingPolicy::Multivalue,
                            )
                            .dns_records(
                                aws_sdk_servicediscovery::types::DnsRecord::builder()
                                    .r#type(aws_sdk_servicediscovery::types::RecordType::A)
                                    .ttl(10)
                                    .build()
                                    .unwrap(),
                            )
                            .build()
                            .unwrap(),
                    )
                    .send()
                    .await?;
                let sd_service_arn = sd_service.service().unwrap().arn().unwrap().to_string();
                trace!(name: "created_cluster_sd_service", %sd_service_arn);

                // Create ECS service for this cluster task
                trace!(name: "creating_cluster_ecs_service", %task_name);
                ecs_client
                    .create_service()
                    .cluster(&cluster_name)
                    .service_name(&task_name)
                    .task_definition(&task_name)
                    .desired_count(1)
                    .launch_type(aws_sdk_ecs::types::LaunchType::Fargate)
                    .network_configuration(
                        NetworkConfiguration::builder()
                            .awsvpc_configuration(
                                AwsVpcConfiguration::builder()
                                    .set_subnets(Some(subnets.clone()))
                                    .set_security_groups(Some(security_groups.clone()))
                                    .assign_public_ip(aws_sdk_ecs::types::AssignPublicIp::Enabled)
                                    .build()?,
                            )
                            .build(),
                    )
                    .service_registries(
                        ServiceRegistry::builder()
                            .registry_arn(&sd_service_arn)
                            .build(),
                    )
                    .send()
                    .await?;
                trace!(name: "cluster_service_created", %task_name);
            }
        }

        // Wait for all services to have running tasks
        trace!("waiting_for_services_to_stabilize");
        let mut all_service_names = Vec::new();
        for (_, _, process) in nodes.get_all_processes() {
            all_service_names.push(get_docker_container_name(&process.name, None));
        }
        for (_, _, cluster) in nodes.get_all_clusters() {
            for num in 0..cluster.count {
                all_service_names.push(get_docker_container_name(&cluster.name, Some(num)));
            }
        }

        for service_name in &all_service_names {
            trace!(name: "waiting_for_service", %service_name);
            loop {
                let services = ecs_client
                    .describe_services()
                    .cluster(&cluster_name)
                    .services(service_name)
                    .send()
                    .await?;

                if let Some(service) = services.services().first() {
                    if service.running_count() >= 1 {
                        trace!(name: "service_running", %service_name, running_count = service.running_count());
                        break;
                    }
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        }

        // Give DNS a bit more time to propagate
        trace!("waiting_for_dns_propagation");
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

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
        use aws_sdk_ecs::Client as EcsClient;
        use aws_sdk_servicediscovery::Client as ServiceDiscoveryClient;

        let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
        let ecs_client = EcsClient::new(&config);
        let sd_client = ServiceDiscoveryClient::new(&config);
        let cluster_name = format!("hydro-{}", self.deployment_instance);
        let namespace_name = format!("hydro-{}.local", self.deployment_instance);

        // Collect all service names
        let mut service_names = Vec::new();
        for (_, _, process) in nodes.get_all_processes() {
            service_names.push(get_docker_container_name(&process.name, None));
        }
        for (_, _, cluster) in nodes.get_all_clusters() {
            for num in 0..cluster.count {
                service_names.push(get_docker_container_name(&cluster.name, Some(num)));
            }
        }

        // Delete ECS services
        for service_name in &service_names {
            let _ = ecs_client
                .delete_service()
                .cluster(&cluster_name)
                .service(service_name)
                .force(true)
                .send()
                .await;
        }

        // Deregister task definitions
        for service_name in &service_names {
            let _ = ecs_client
                .deregister_task_definition()
                .task_definition(service_name)
                .send()
                .await;
        }

        // Find and delete Cloud Map namespace and services
        if let Ok(namespaces) = sd_client.list_namespaces().send().await {
            for ns in namespaces.namespaces() {
                if ns.name() == Some(&namespace_name) {
                    if let Some(ns_id) = ns.id() {
                        // Delete all services in the namespace first
                        if let Ok(services) = sd_client.list_services().send().await {
                            for svc in services.services() {
                                if service_names.iter().any(|n| Some(n.as_str()) == svc.name()) {
                                    if let Some(svc_id) = svc.id() {
                                        let _ = sd_client.delete_service().id(svc_id).send().await;
                                    }
                                }
                            }
                        }
                        // Delete the namespace
                        let _ = sd_client.delete_namespace().id(ns_id).send().await;
                    }
                }
            }
        }

        // Delete ECS cluster
        let _ = ecs_client
            .delete_cluster()
            .cluster(&cluster_name)
            .send()
            .await;

        // Delete ECR repositories
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

        // Note: VPC cleanup is complex due to dependencies.
        // For now, VPCs created by this deployment will need manual cleanup or a separate cleanup script.
        // TODO: Track VPC ID during start() and clean up here

        Ok(())
    }
}

impl<'a> Deploy<'a> for DockerDeployEcs {
    type InstantiateEnv = Self;
    type Process = DockerDeployProcessEcs;
    type Cluster = DockerDeployClusterEcs;
    type External = DockerDeployExternalEcs;
    type Port = u16;
    type ExternalRawPort = ();
    type Meta = ();
    type GraphId = ();

    #[instrument(level = "trace", skip_all, ret)]
    fn allocate_process_port(process: &Self::Process) -> Self::Port {
        process.next_port()
    }

    #[instrument(level = "trace", skip_all, ret)]
    fn allocate_cluster_port(cluster: &Self::Cluster) -> Self::Port {
        cluster.next_port()
    }

    #[instrument(level = "trace", skip_all, ret)]
    fn allocate_external_port(external: &Self::External) -> Self::Port {
        external.next_port()
    }

    #[instrument(level = "trace", skip_all, fields(p1 = p1.name, %p1_port, p2 = p2.name, p2_port))]
    fn o2o_sink_source(
        p1: &Self::Process,
        p1_port: &Self::Port,
        p2: &Self::Process,
        p2_port: &Self::Port,
    ) -> (syn::Expr, syn::Expr) {
        let bind_addr = format!("0.0.0.0:{}", p2_port);
        // Use full Cloud Map DNS name for ECS service discovery
        let target = format!(
            "{}.hydro-{}.local:{p2_port}",
            p2.name, p2.deployment_instance
        );

        deploy_containerized_o2o(target.as_str(), bind_addr.as_str())
    }

    #[instrument(level = "trace", skip_all, fields(p1 = p1.name, %p1_port, p2 = p2.name, p2_port))]
    fn o2o_connect(
        p1: &Self::Process,
        p1_port: &Self::Port,
        p2: &Self::Process,
        p2_port: &Self::Port,
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
        p1_port: &Self::Port,
        c2: &Self::Cluster,
        c2_port: &Self::Port,
    ) -> (syn::Expr, syn::Expr) {
        deploy_containerized_o2m(*c2_port)
    }

    #[instrument(level = "trace", skip_all, fields(p1 = p1.name, %p1_port, c2 = c2.name, %c2_port))]
    fn o2m_connect(
        p1: &Self::Process,
        p1_port: &Self::Port,
        c2: &Self::Cluster,
        c2_port: &Self::Port,
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
        c1_port: &Self::Port,
        p2: &Self::Process,
        p2_port: &Self::Port,
    ) -> (syn::Expr, syn::Expr) {
        // Use full Cloud Map DNS name for ECS service discovery
        let target_dns = format!("{}.hydro-{}.local", p2.name, p2.deployment_instance);
        deploy_containerized_m2o(*p2_port, &target_dns)
    }

    #[instrument(level = "trace", skip_all, fields(c1 = c1.name, %c1_port, p2 = p2.name, %p2_port))]
    fn m2o_connect(
        c1: &Self::Cluster,
        c1_port: &Self::Port,
        p2: &Self::Process,
        p2_port: &Self::Port,
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
        c1_port: &Self::Port,
        c2: &Self::Cluster,
        c2_port: &Self::Port,
    ) -> (syn::Expr, syn::Expr) {
        deploy_containerized_m2m(*c2_port)
    }

    #[instrument(level = "trace", skip_all, fields(c1 = c1.name, %c1_port, c2 = c2.name, %c2_port))]
    fn m2m_connect(
        c1: &Self::Cluster,
        c1_port: &Self::Port,
        c2: &Self::Cluster,
        c2_port: &Self::Port,
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
        p2_port: &Self::Port,
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
        p1_port: &Self::Port,
        p2: &Self::Process,
        p2_port: &Self::Port,
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
            ::tracing::trace!(name: "binding", bind_addr = #bind_addr);
        });
        extra_stmts.push(syn::parse_quote! {
            let #socket_ident = tokio::net::TcpListener::bind(#bind_addr).await.unwrap();
        });
        extra_stmts.push(syn::parse_quote! {
            ::tracing::trace!(name: "bound", bind_addr = #bind_addr);
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
        p1_port: &Self::Port,
        p2: &Self::Process,
        p2_port: &Self::Port,
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
        p1_port: &Self::Port,
        p2: &Self::External,
        p2_port: &Self::Port,
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

            deployment_instance: self.deployment_instance,
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
