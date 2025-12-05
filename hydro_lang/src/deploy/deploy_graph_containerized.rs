//! Deployment backend for Hydro that uses Docker to provision and launch services.

use std::cell::RefCell;
use std::collections::HashMap;
use std::pin::Pin;
use std::rc::Rc;

use bollard::Docker;
use bollard::container::{
    Config, CreateContainerOptions, KillContainerOptions, NetworkingConfig, RemoveContainerOptions,
    StartContainerOptions,
};
use bollard::image::BuildImageOptions;
use bollard::network::CreateNetworkOptions;
use bollard::secret::{EndpointSettings, HostConfig};
use dfir_lang::graph::DfirGraph;
use futures::{Sink, SinkExt, Stream, StreamExt};
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

use super::deploy_runtime_containerized::*;
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
pub struct DockerNetwork {
    name: String,
}

impl DockerNetwork {
    /// creates a new docker network (wil actually be created when deployment.start() is called).
    pub fn new(name: String) -> Self {
        Self {
            name: format!("{name}-{}", nanoid::nanoid!(6, &CONTAINER_ALPHABET)),
        }
    }
}

/// Represents a process running in a docker container
#[derive(Clone)]
pub struct DockerDeployProcess {
    id: usize,
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

        ret = ret.features(vec!["hydro___feature_deploy_integration".to_string()]);

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
    id: usize,
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
pub struct DockerDeployExternal {
    name: String,
    next_port: Rc<RefCell<u16>>,

    ports: Rc<RefCell<HashMap<usize, u16>>>,

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

impl<'a> RegisterPort<'a, DockerDeploy> for DockerDeployExternal {
    #[instrument(level = "trace", skip_all, fields(name = self.name, %key, %port))]
    fn register(&self, key: usize, port: <DockerDeploy as Deploy>::Port) {
        self.ports.borrow_mut().insert(key, port);
    }

    #[instrument(level = "trace", skip_all, fields(name = self.name, %key))]
    fn raw_port(&self, key: usize) -> <DockerDeploy as Deploy<'a>>::ExternalRawPort {
        todo!()
    }

    fn as_bytes_bidi(
        &self,
        key: usize,
    ) -> impl Future<
        Output = DynSourceSink<
            Result<bytes::BytesMut, std::io::Error>,
            bytes::Bytes,
            std::io::Error,
        >,
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
        let (docker_container_name, remote_port, network) = self
            .connection_info
            .borrow()
            .get(&local_port)
            .unwrap()
            .clone();

        async move {
            let docker = Docker::connect_with_local_defaults().unwrap();

            let docker_container_name = docker_container_name.borrow().as_ref().unwrap().clone();

            let container_info = docker
                .inspect_container(&docker_container_name, None)
                .await
                .unwrap();

            let remote_ip_address = container_info
                .network_settings
                .as_ref()
                .unwrap()
                .networks
                .as_ref()
                .unwrap()
                .get(&network.name)
                .unwrap()
                .ip_address
                .as_ref()
                .unwrap()
                .clone();

            Box::pin(
                LazySink::new(move || {
                    Box::pin(async move {
                        trace!(name: "as_bincode_sink_connecting", to = %remote_ip_address, to_port = %remote_port);

                        let stream =
                            TcpStream::connect(format!("{remote_ip_address}:{remote_port}"))
                                .await?;

                        trace!(name: "as_bincode_sink_connected", to = %remote_ip_address, to_port = %remote_port);

                        Result::<_, std::io::Error>::Ok(FramedWrite::new(
                            stream,
                            LengthDelimitedCodec::new(),
                        ))
                    })
                })
                .with(move |v| async move {
                    Ok(bytes::Bytes::from(bincode::serialize(&v).unwrap()))
                }),
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
        let (docker_container_name, remote_port, network) = self
            .connection_info
            .borrow()
            .get(&local_port)
            .unwrap()
            .clone();

        async move {
            let docker = Docker::connect_with_local_defaults().unwrap();

            let docker_container_name = docker_container_name.borrow().as_ref().unwrap().clone();

            let container_info = docker
                .inspect_container(&docker_container_name, None)
                .await
                .unwrap();

            let remote_ip_address = container_info
                .network_settings
                .as_ref()
                .unwrap()
                .networks
                .as_ref()
                .unwrap()
                .get(&network.name)
                .unwrap()
                .ip_address
                .as_ref()
                .unwrap()
                .clone();

            trace!(name: "as_bincode_source_connecting", to = %remote_ip_address, to_port = %remote_port);

            let stream = TcpStream::connect(format!("{remote_ip_address}:{remote_port}"))
                .await
                .unwrap();

            trace!(name: "as_bincode_source_connected", to = %remote_ip_address, to_port = %remote_port);

            Box::pin(
                FramedRead::new(stream, LengthDelimitedCodec::new())
                    .map(|v| bincode::deserialize(&v.unwrap()).unwrap()),
            ) as Pin<Box<dyn Stream<Item = T>>>
        }
        .instrument(guard.exit())
    }
}

/// For deploying to a local docker instance
pub struct DockerDeploy {
    docker_processes: Vec<DockerDeployProcessSpec>,
    docker_clusters: Vec<DockerDeployClusterSpec>,
    network: DockerNetwork,
    deployment_instance: String,
}

#[instrument(level = "trace", skip_all, fields(%image_name, %container_name, %network_name, %deployment_instance, ?exposed_ports))]
async fn create_and_start_container(
    docker: &Docker,
    container_name: &str,
    image_name: &str,
    network_name: &str,
    deployment_instance: &str,
    exposed_ports: Option<HashMap<String, HashMap<(), ()>>>,
) -> Result<(), anyhow::Error> {
    let config = Config {
        image: Some(image_name.to_string()),
        hostname: Some(container_name.to_string()),
        host_config: Some(HostConfig {
            binds: Some(vec![
                "/var/run/docker.sock:/var/run/docker.sock".to_string(),
            ]),
            ..Default::default()
        }),
        env: Some(vec![
            format!("CONTAINER_NAME={container_name}"),
            format!("DEPLOYMENT_INSTANCE={deployment_instance}"),
            format!("RUST_LOG=trace"),
        ]),
        exposed_ports,
        networking_config: Some(NetworkingConfig {
            endpoints_config: HashMap::from([(
                network_name.to_string(),
                EndpointSettings {
                    ..Default::default()
                },
            )]),
        }),
        tty: Some(true),
        ..Default::default()
    };

    let options = CreateContainerOptions {
        name: container_name.to_string(),
        platform: None,
    };

    docker.create_container(Some(options), config).await?;
    docker
        .start_container(container_name, None::<StartContainerOptions<String>>)
        .await?;

    Ok(())
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
                    FROM scratch
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
        t: image_name.to_string(),
        rm: true,
        ..Default::default()
    };

    use bollard::errors::Error;

    let mut build_stream = docker.build_image(build_options, None, Some(tar_data.into()));
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

        Ok(())
    }

    /// Start the deployment, tell docker to create containers from the existing provisioned images.
    #[instrument(level = "trace", skip_all)]
    pub async fn start(&self, nodes: &DeployResult<'_, Self>) -> Result<(), anyhow::Error> {
        let docker = Docker::connect_with_local_defaults()?;

        match docker
            .create_network(CreateNetworkOptions {
                name: self.network.name.clone(),
                driver: "bridge".to_string(),
                ..Default::default()
            })
            .await
        {
            Ok(v) => v.id.unwrap(),
            Err(e) => {
                panic!("Failed to create docker network: {e:?}");
            }
        };

        for (_, _, process) in nodes.get_all_processes() {
            let docker_container_name: String = get_docker_container_name(&process.name, None);
            *process.docker_container_name.borrow_mut() = Some(docker_container_name.clone());

            let exposed_ports = Some(HashMap::from_iter(
                process
                    .exposed_ports
                    .borrow()
                    .iter()
                    .map(|port| (format!("{port}/tcp"), HashMap::new())),
            ));

            create_and_start_container(
                &docker,
                &docker_container_name,
                &process.name,
                &self.network.name,
                &self.deployment_instance,
                exposed_ports,
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
                    None,
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
                .kill_container(&docker_container_name, None::<KillContainerOptions<String>>)
                .await?;
        }

        for (_, _, cluster) in nodes.get_all_clusters() {
            for num in 0..cluster.count {
                let docker_container_name = get_docker_container_name(&cluster.name, Some(num));

                docker
                    .kill_container(&docker_container_name, None::<KillContainerOptions<String>>)
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

        for (_, _, process) in nodes.get_all_processes() {
            docker.remove_image(&process.name, None, None).await?;
        }

        for (_, _, cluster) in nodes.get_all_clusters() {
            docker.remove_image(&cluster.name, None, None).await?;
        }

        Ok(())
    }
}

impl<'a> Deploy<'a> for DockerDeploy {
    type InstantiateEnv = Self;
    type Process = DockerDeployProcess;
    type Cluster = DockerDeployCluster;
    type External = DockerDeployExternal;
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
        let target = format!("{}:{p2_port}", p2.name);

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
        deploy_containerized_m2o(*p2_port, &p2.name)
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
pub struct DockerDeployProcessSpec {
    compilation_options: Option<String>,
    config: Vec<String>,
    network: DockerNetwork,
    deployment_instance: String,
}

impl<'a> ProcessSpec<'a, DockerDeploy> for DockerDeployProcessSpec {
    #[instrument(level = "trace", skip_all, fields(%id, %name_hint))]
    fn build(self, id: usize, name_hint: &'_ str) -> <DockerDeploy as Deploy<'a>>::Process {
        DockerDeployProcess {
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
pub struct DockerDeployClusterSpec {
    compilation_options: Option<String>,
    config: Vec<String>,
    count: usize,
    deployment_instance: String,
}

impl<'a> ClusterSpec<'a, DockerDeploy> for DockerDeployClusterSpec {
    #[instrument(level = "trace", skip_all, fields(%id, %name_hint))]
    fn build(self, id: usize, name_hint: &str) -> <DockerDeploy as Deploy<'a>>::Cluster {
        DockerDeployCluster {
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
pub struct DockerDeployExternalSpec {
    name: String,
}

impl<'a> ExternalSpec<'a, DockerDeploy> for DockerDeployExternalSpec {
    #[instrument(level = "trace", skip_all, fields(%id, %name_hint))]
    fn build(self, id: usize, name_hint: &str) -> <DockerDeploy as Deploy<'a>>::External {
        DockerDeployExternal {
            name: self.name,
            next_port: Rc::new(RefCell::new(10000)),
            ports: Rc::new(RefCell::new(HashMap::new())),
            connection_info: Rc::new(RefCell::new(HashMap::new())),
        }
    }
}
