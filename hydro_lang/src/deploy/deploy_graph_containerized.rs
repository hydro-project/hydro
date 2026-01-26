//! Deployment backend for Hydro that uses Docker to provision and launch services.

use std::cell::RefCell;
use std::collections::HashMap;
use std::pin::Pin;
use std::rc::Rc;

use bollard::Docker;
use bollard::models::{ContainerCreateBody, EndpointSettings, HostConfig, NetworkCreateRequest};
use bollard::query_parameters::{
    BuildImageOptions, CreateContainerOptions, InspectContainerOptions, KillContainerOptions,
    RemoveContainerOptions, StartContainerOptions,
};
use bollard::secret::NetworkingConfig;
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
use tracing::{Instrument, instrument, trace, warn};

use super::deploy_runtime_containerized::*;
use crate::compile::builder::ExternalPortId;
use crate::compile::deploy::DeployResult;
use crate::compile::deploy_provider::{
    ClusterSpec, Deploy, ExternalSpec, Node, ProcessSpec, RegisterPort,
};
use crate::compile::trybuild::generate::{LinkingMode, create_graph_trybuild};
use crate::location::dynamic::LocationId;
use crate::location::member_id::TaglessMemberId;
use crate::location::{LocationKey, MembershipEvent, NetworkHint};

/// represents a docker network
#[derive(Clone, Debug)]
pub struct DockerNetwork {
    name: String,
}

impl DockerNetwork {
    /// creates a new docker network (will actually be created when deployment.start() is called).
    pub fn new(name: String) -> Self {
        Self {
            name: format!("{name}-{}", nanoid::nanoid!(6, &CONTAINER_ALPHABET)),
        }
    }
}

/// Represents a process running in a docker container
#[derive(Clone)]
pub struct DockerDeployProcess {
    key: LocationKey,
    name: String,
    next_port: Rc<RefCell<u16>>,
    rust_crate: Rc<RefCell<Option<RustCrate>>>,

    exposed_ports: Rc<RefCell<Vec<u16>>>,

    docker_container_name: Rc<RefCell<Option<String>>>,

    compilation_options: Option<String>,

    config: Vec<String>,

    network: DockerNetwork,
}

impl Node for DockerDeployProcess {
    type Port = u16;
    type Meta = ();
    type InstantiateEnv = DockerDeploy;

    #[instrument(level = "trace", skip_all, ret, fields(key = %self.key, name = self.name))]
    fn next_port(&self) -> Self::Port {
        let port = {
            let mut borrow = self.next_port.borrow_mut();
            let port = *borrow;
            *borrow += 1;
            port
        };

        port
    }

    #[instrument(level = "trace", skip_all, fields(key = %self.key, name = self.name))]
    fn update_meta(&self, _meta: &Self::Meta) {}

    #[instrument(level = "trace", skip_all, fields(key = %self.key, name = self.name, ?meta, extra_stmts = extra_stmts.len()))]
    fn instantiate(
        &self,
        _env: &mut Self::InstantiateEnv,
        meta: &mut Self::Meta,
        graph: DfirGraph,
        extra_stmts: Vec<syn::Stmt>,
    ) {
        let (bin_name, config) = create_graph_trybuild(
            graph,
            extra_stmts,
            Some(&self.name),
            true,
            LinkingMode::Static,
        );

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
pub struct DockerDeployCluster {
    key: LocationKey,
    name: String,
    next_port: Rc<RefCell<u16>>,
    rust_crate: Rc<RefCell<Option<RustCrate>>>,

    docker_container_name: Rc<RefCell<Vec<String>>>,

    compilation_options: Option<String>,

    config: Vec<String>,

    count: usize,
}

impl Node for DockerDeployCluster {
    type Port = u16;
    type Meta = ();
    type InstantiateEnv = DockerDeploy;

    #[instrument(level = "trace", skip_all, ret, fields(key = %self.key, name = self.name))]
    fn next_port(&self) -> Self::Port {
        let port = {
            let mut borrow = self.next_port.borrow_mut();
            let port = *borrow;
            *borrow += 1;
            port
        };

        port
    }

    #[instrument(level = "trace", skip_all, fields(key = %self.key, name = self.name))]
    fn update_meta(&self, _meta: &Self::Meta) {}

    #[instrument(level = "trace", skip_all, fields(key = %self.key, name = self.name, extra_stmts = extra_stmts.len()))]
    fn instantiate(
        &self,
        _env: &mut Self::InstantiateEnv,
        _meta: &mut Self::Meta,
        graph: DfirGraph,
        extra_stmts: Vec<syn::Stmt>,
    ) {
        let (bin_name, config) = create_graph_trybuild(
            graph,
            extra_stmts,
            Some(&self.name),
            true,
            LinkingMode::Static,
        );

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

/// Represents an external process, outside the control of this deployment but still with some communication into this deployment.
#[derive(Clone, Debug)]
pub struct DockerDeployExternal {
    name: String,
    next_port: Rc<RefCell<u16>>,

    ports: Rc<RefCell<HashMap<ExternalPortId, u16>>>,

    #[expect(clippy::type_complexity, reason = "internal code")]
    connection_info: Rc<RefCell<HashMap<u16, (Rc<RefCell<Option<String>>>, u16, DockerNetwork)>>>,
}

impl Node for DockerDeployExternal {
    type Port = u16;
    type Meta = ();
    type InstantiateEnv = DockerDeploy;

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

impl<'a> RegisterPort<'a, DockerDeploy> for DockerDeployExternal {
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
        let (docker_container_name, remote_port, _) = self
            .connection_info
            .borrow()
            .get(&local_port)
            .unwrap()
            .clone();

        let docker_container_name = docker_container_name.borrow().as_ref().unwrap().clone();

        async move {
            let local_port = find_dynamically_allocated_docker_port(&docker_container_name, remote_port).await;
            let remote_ip_address = "localhost";

            Box::pin(
                LazySink::new(move || {
                    Box::pin(async move {
                        trace!(name: "as_bincode_sink_connecting", to = %remote_ip_address, to_port = %local_port);

                        let stream =
                            TcpStream::connect(format!("{remote_ip_address}:{local_port}"))
                                .await?;

                        trace!(name: "as_bincode_sink_connected", to = %remote_ip_address, to_port = %local_port);

                        Result::<_, std::io::Error>::Ok(FramedWrite::new(
                            stream,
                            LengthDelimitedCodec::new(),
                        ))
                    })
                })
                .with(move |v| async move {
                    Ok(Bytes::from(bincode::serialize(&v).unwrap()))
                }),
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
        let (docker_container_name, remote_port, _) = self
            .connection_info
            .borrow()
            .get(&local_port)
            .unwrap()
            .clone();

        let docker_container_name = docker_container_name.borrow().as_ref().unwrap().clone();

        async move {

            let local_port = find_dynamically_allocated_docker_port(&docker_container_name, remote_port).await;
            let remote_ip_address = "localhost";

            trace!(name: "as_bincode_source_connecting", to = %remote_ip_address, to_port = %local_port);

            let stream = TcpStream::connect(format!("{remote_ip_address}:{local_port}"))
                .await
                .unwrap();

            trace!(name: "as_bincode_source_connected", to = %remote_ip_address, to_port = %local_port);

            Box::pin(
                FramedRead::new(stream, LengthDelimitedCodec::new())
                    .map(|v| bincode::deserialize(&v.unwrap()).unwrap()),
            ) as Pin<Box<dyn Stream<Item = T>>>
        }
        .instrument(guard.exit())
    }
}

#[instrument(level = "trace", skip_all, fields(%docker_container_name, %destination_port))]
async fn find_dynamically_allocated_docker_port(
    docker_container_name: &str,
    destination_port: u16,
) -> u16 {
    let docker = Docker::connect_with_local_defaults().unwrap();

    let container_info = docker
        .inspect_container(docker_container_name, None::<InspectContainerOptions>)
        .await
        .unwrap();

    trace!(name: "port struct", container_info = ?container_info.network_settings.as_ref().unwrap().ports.as_ref().unwrap());

    // container_info={"1001/tcp": Some([PortBinding { host_ip: Some("0.0.0.0"), host_port: Some("32771") }, PortBinding { host_ip: Some("::"), host_port: Some("32771") }])} destination_port=1001
    let remote_port = container_info
        .network_settings
        .as_ref()
        .unwrap()
        .ports
        .as_ref()
        .unwrap()
        .get(&format!("{destination_port}/tcp"))
        .unwrap()
        .as_ref()
        .unwrap()
        .iter()
        .find(|v| v.host_ip == Some("0.0.0.0".to_string()))
        .unwrap()
        .host_port
        .as_ref()
        .unwrap()
        .parse()
        .unwrap();

    remote_port
}

/// For deploying to a local docker instance
pub struct DockerDeploy {
    docker_processes: Vec<DockerDeployProcessSpec>,
    docker_clusters: Vec<DockerDeployClusterSpec>,
    network: DockerNetwork,
    deployment_instance: String,
}

#[instrument(level = "trace", skip_all, fields(%image_name, %container_name, %network_name, %deployment_instance))]
async fn create_and_start_container(
    docker: &Docker,
    container_name: &str,
    image_name: &str,
    network_name: &str,
    deployment_instance: &str,
) -> Result<(), anyhow::Error> {
    let config = ContainerCreateBody {
        image: Some(image_name.to_string()),
        hostname: Some(container_name.to_string()),
        host_config: Some(HostConfig {
            binds: Some(vec![
                "/var/run/docker.sock:/var/run/docker.sock".to_string(),
            ]),
            publish_all_ports: Some(true),
            port_bindings: Some(HashMap::new()), /* Due to a bug in docker, if you don't send empty port bindings with publish_all_ports set to true and with a docker image that has EXPOSE directives in it, docker will crash because it will try to write to a map in memory that it has not initialized yet. Setting port_bindings explicitly to an empty map will initialize it first so that it does not break. */
            ..Default::default()
        }),
        env: Some(vec![
            format!("CONTAINER_NAME={container_name}"),
            format!("DEPLOYMENT_INSTANCE={deployment_instance}"),
            format!("RUST_LOG=trace"),
        ]),
        networking_config: Some(NetworkingConfig {
            endpoints_config: Some(HashMap::from([(
                network_name.to_string(),
                EndpointSettings {
                    ..Default::default()
                },
            )])),
        }),
        tty: Some(true),
        ..Default::default()
    };

    let options = CreateContainerOptions {
        name: Some(container_name.to_string()),
        ..Default::default()
    };

    tracing::error!("Config: {}", serde_json::to_string_pretty(&config).unwrap());
    docker.create_container(Some(options), config).await?;
    docker
        .start_container(container_name, None::<StartContainerOptions>)
        .await?;

    Ok(())
}

#[instrument(level = "trace", skip_all, fields(%image_name))]
async fn build_and_create_image(
    rust_crate: &Rc<RefCell<Option<RustCrate>>>,
    compilation_options: &Option<String>,
    config: &[String],
    exposed_ports: &[u16],
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

        let exposed_ports = exposed_ports
            .iter()
            .map(|port| format!("EXPOSE {port}/tcp"))
            .collect::<Vec<_>>()
            .join("\n");

        let dockerfile_content = format!(
            r#"
                FROM scratch
                {exposed_ports}
                COPY app /app
                CMD ["/app"]
            "#,
        );

        trace!(name: "dockerfile", %dockerfile_content);

        let mut header = Header::new_gnu();
        header.set_path("Dockerfile")?;
        header.set_size(dockerfile_content.len() as u64);
        header.set_cksum();
        tar.append(&header, dockerfile_content.as_bytes())?;

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

impl DockerDeploy {
    /// Create a new deployment
    pub fn new(network: DockerNetwork) -> Self {
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
    ) -> DockerDeployProcessSpec {
        let process = DockerDeployProcessSpec {
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
    ) -> DockerDeployClusterSpec {
        let cluster = DockerDeployClusterSpec {
            compilation_options,
            config,
            count,
            deployment_instance: self.deployment_instance.clone(),
        };

        self.docker_clusters.push(cluster.clone());

        cluster
    }

    /// Add an external process to the deployment.
    pub fn add_external(&self, name: String) -> DockerDeployExternalSpec {
        DockerDeployExternalSpec { name }
    }

    /// Get the deployment instance from this deployment.
    pub fn get_deployment_instance(&self) -> String {
        self.deployment_instance.clone()
    }

    /// Create docker images.
    #[instrument(level = "trace", skip_all)]
    pub async fn provision(&self, nodes: &DeployResult<'_, Self>) -> Result<(), anyhow::Error> {
        for (_, _, process) in nodes.get_all_processes() {
            let exposed_ports = process.exposed_ports.borrow().clone();

            build_and_create_image(
                &process.rust_crate,
                &process.compilation_options,
                &process.config,
                &exposed_ports,
                &process.name,
            )
            .await?;
        }

        for (_, _, cluster) in nodes.get_all_clusters() {
            build_and_create_image(
                &cluster.rust_crate,
                &cluster.compilation_options,
                &cluster.config,
                &[], // clusters don't have exposed ports.
                &cluster.name,
            )
            .await?;
        }

        Ok(())
    }

    /// Start the deployment, tell docker to create containers from the existing provisioned images.
    #[instrument(level = "trace", skip_all)]
    pub async fn start(&self, nodes: &DeployResult<'_, Self>) -> Result<(), anyhow::Error> {
        let docker = Docker::connect_with_local_defaults()?;

        match docker
            .create_network(NetworkCreateRequest {
                name: self.network.name.clone(),
                driver: Some("bridge".to_string()),
                ..Default::default()
            })
            .await
        {
            Ok(v) => v.id,
            Err(e) => {
                panic!("Failed to create docker network: {e:?}");
            }
        };

        for (_, _, process) in nodes.get_all_processes() {
            let docker_container_name: String = get_docker_container_name(&process.name, None);
            *process.docker_container_name.borrow_mut() = Some(docker_container_name.clone());

            create_and_start_container(
                &docker,
                &docker_container_name,
                &process.name,
                &self.network.name,
                &self.deployment_instance,
            )
            .await?;
        }

        for (_, _, cluster) in nodes.get_all_clusters() {
            for num in 0..cluster.count {
                let docker_container_name = get_docker_container_name(&cluster.name, Some(num));
                cluster
                    .docker_container_name
                    .borrow_mut()
                    .push(docker_container_name.clone());

                create_and_start_container(
                    &docker,
                    &docker_container_name,
                    &cluster.name,
                    &self.network.name,
                    &self.deployment_instance,
                )
                .await?;
            }
        }

        Ok(())
    }

    /// Stop the deployment, destroy all containers
    #[instrument(level = "trace", skip_all)]
    pub async fn stop(&mut self, nodes: &DeployResult<'_, Self>) -> Result<(), anyhow::Error> {
        let docker = Docker::connect_with_local_defaults()?;

        for (_, _, process) in nodes.get_all_processes() {
            let docker_container_name: String = get_docker_container_name(&process.name, None);

            docker
                .kill_container(&docker_container_name, None::<KillContainerOptions>)
                .await?;
        }

        for (_, _, cluster) in nodes.get_all_clusters() {
            for num in 0..cluster.count {
                let docker_container_name = get_docker_container_name(&cluster.name, Some(num));

                docker
                    .kill_container(&docker_container_name, None::<KillContainerOptions>)
                    .await?;
            }
        }

        Ok(())
    }

    /// remove containers, images, and networks.
    #[instrument(level = "trace", skip_all)]
    pub async fn cleanup(&mut self, nodes: &DeployResult<'_, Self>) -> Result<(), anyhow::Error> {
        let docker = Docker::connect_with_local_defaults()?;

        for (_, _, process) in nodes.get_all_processes() {
            let docker_container_name: String = get_docker_container_name(&process.name, None);

            docker
                .remove_container(&docker_container_name, None::<RemoveContainerOptions>)
                .await?;
        }

        for (_, _, cluster) in nodes.get_all_clusters() {
            for num in 0..cluster.count {
                let docker_container_name = get_docker_container_name(&cluster.name, Some(num));

                docker
                    .remove_container(&docker_container_name, None::<RemoveContainerOptions>)
                    .await?;
            }
        }

        docker
            .remove_network(&self.network.name)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to remove docker network: {e:?}"))?;

        use bollard::query_parameters::RemoveImageOptions;

        for (_, _, process) in nodes.get_all_processes() {
            docker
                .remove_image(&process.name, None::<RemoveImageOptions>, None)
                .await?;
        }

        for (_, _, cluster) in nodes.get_all_clusters() {
            docker
                .remove_image(&cluster.name, None::<RemoveImageOptions>, None)
                .await?;
        }

        Ok(())
    }
}

impl<'a> Deploy<'a> for DockerDeploy {
    type Meta = ();
    type InstantiateEnv = Self;

    type Process = DockerDeployProcess;
    type Cluster = DockerDeployCluster;
    type External = DockerDeployExternal;

    #[instrument(level = "trace", skip_all, fields(p1 = p1.name, %p1_port, p2 = p2.name, p2_port))]
    fn o2o_sink_source(
        p1: &Self::Process,
        p1_port: &<Self::Process as Node>::Port,
        p2: &Self::Process,
        p2_port: &<Self::Process as Node>::Port,
    ) -> (syn::Expr, syn::Expr) {
        let bind_addr = format!("0.0.0.0:{}", p2_port);
        let target = format!("{}:{p2_port}", p2.name);

        deploy_containerized_o2o(target.as_str(), bind_addr.as_str())
    }

    #[instrument(level = "trace", skip_all, fields(p1 = p1.name, %p1_port, p2 = p2.name, p2_port))]
    fn o2o_connect(
        p1: &Self::Process,
        p1_port: &<Self::Process as Node>::Port,
        p2: &Self::Process,
        p2_port: &<Self::Process as Node>::Port,
    ) -> Box<dyn FnOnce()> {
        let serialized = format!("o2o_connect {}:{p1_port} -> {}:{p2_port}", p1.name, p2.name);

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
        let serialized = format!("o2m_connect {}:{p1_port} -> {}:{c2_port}", p1.name, c2.name);

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
        deploy_containerized_m2o(*p2_port, &p2.name)
    }

    #[instrument(level = "trace", skip_all, fields(c1 = c1.name, %c1_port, p2 = p2.name, %p2_port))]
    fn m2o_connect(
        c1: &Self::Cluster,
        c1_port: &<Self::Cluster as Node>::Port,
        p2: &Self::Process,
        p2_port: &<Self::Process as Node>::Port,
    ) -> Box<dyn FnOnce()> {
        let serialized = format!("o2m_connect {}:{c1_port} -> {}:{p2_port}", c1.name, p2.name);

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
        let serialized = format!("m2m_connect {}:{c1_port} -> {}:{c2_port}", c1.name, c2.name);

        Box::new(move || {
            trace!(name: "m2m_connect thunk", %serialized);
        })
    }

    #[instrument(level = "trace", skip_all, fields(p2 = p2.name, %p2_port, %shared_handle, extra_stmts = extra_stmts.len()))]
    fn e2o_many_source(
        extra_stmts: &mut Vec<syn::Stmt>,
        p2: &Self::Process,
        p2_port: &<Self::Process as Node>::Port,
        _codec_type: &syn::Type,
        shared_handle: String,
    ) -> syn::Expr {
        todo!()
    }

    #[instrument(level = "trace", skip_all, fields(%shared_handle))]
    fn e2o_many_sink(shared_handle: String) -> syn::Expr {
        todo!()
    }

    #[instrument(level = "trace", skip_all, fields(p1 = p1.name, %p1_port, p2 = p2.name, %p2_port, %shared_handle))]
    fn e2o_source(
        extra_stmts: &mut Vec<syn::Stmt>,
        p1: &Self::External,
        p1_port: &<Self::External as Node>::Port,
        p2: &Self::Process,
        p2_port: &<Self::Process as Node>::Port,
        _codec_type: &syn::Type,
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

        let create_expr = deploy_containerized_external_sink_source_ident(socket_ident.clone());

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
        let serialized = format!("e2o_connect {}:{p1_port} -> {}:{p2_port}", p1.name, p2.name);

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
        of_cluster: LocationKey,
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

#[instrument(level = "trace", skip_all, ret, fields(%name_hint, %location_key, %deployment_instance))]
fn get_docker_image_name(
    name_hint: &str,
    location_key: LocationKey,
    deployment_instance: &str,
) -> String {
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

    format!("hy-{name_hint}-{image_unique_tag}-{deployment_instance}-{location_key}")
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
pub struct DockerDeployProcessSpec {
    compilation_options: Option<String>,
    config: Vec<String>,
    network: DockerNetwork,
    deployment_instance: String,
}

impl<'a> ProcessSpec<'a, DockerDeploy> for DockerDeployProcessSpec {
    #[instrument(level = "trace", skip_all, fields(%key, %name_hint))]
    fn build(self, key: LocationKey, name_hint: &'_ str) -> <DockerDeploy as Deploy<'a>>::Process {
        DockerDeployProcess {
            key,
            name: get_docker_image_name(name_hint, key, &self.deployment_instance),

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
pub struct DockerDeployClusterSpec {
    compilation_options: Option<String>,
    config: Vec<String>,
    count: usize,
    deployment_instance: String,
}

impl<'a> ClusterSpec<'a, DockerDeploy> for DockerDeployClusterSpec {
    #[instrument(level = "trace", skip_all, fields(%key, %name_hint))]
    fn build(self, key: LocationKey, name_hint: &str) -> <DockerDeploy as Deploy<'a>>::Cluster {
        DockerDeployCluster {
            key,
            name: get_docker_image_name(name_hint, key, &self.deployment_instance),

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
pub struct DockerDeployExternalSpec {
    name: String,
}

impl<'a> ExternalSpec<'a, DockerDeploy> for DockerDeployExternalSpec {
    #[instrument(level = "trace", skip_all, fields(%key, %name_hint))]
    fn build(self, key: LocationKey, name_hint: &str) -> <DockerDeploy as Deploy<'a>>::External {
        DockerDeployExternal {
            name: self.name,
            next_port: Rc::new(RefCell::new(10000)),
            ports: Rc::new(RefCell::new(HashMap::new())),
            connection_info: Rc::new(RefCell::new(HashMap::new())),
        }
    }
}
