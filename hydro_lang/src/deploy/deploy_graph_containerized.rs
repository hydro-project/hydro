//! big ol' TODO:

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    path::Path,
    pin::Pin,
    rc::Rc,
};

use super::deploy_runtime_containerized::*;

use crate::{
    compile::deploy_provider::{Deploy, ExternalSpec, Node, ProcessSpec, RegisterPort},
    deploy::trybuild::create_graph_trybuild,
    location::NetworkHint,
};
use bollard::{
    Docker,
    container::{
        Config, CreateContainerOptions, KillContainerOptions, NetworkingConfig,
        StartContainerOptions,
    },
    image::BuildImageOptions,
    network::CreateNetworkOptions,
    secret::{EndpointSettings, HostConfig},
};
use dfir_lang::graph::DfirGraph;
use futures::StreamExt;
use futures::{Sink, SinkExt, Stream};
use hydro_deploy::{
    RustCrate,
    rust_crate::build::{BuildError, build_crate_memoized},
};

use stageleft::{QuotedWithContext, q};
use tar::{Builder, Header};

const CONTAINER_ALPHABET: [char; 36] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i',
    'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
];

/// represents a docker network
#[derive(Clone, Debug)]
pub struct DockerNetwork {
    name: String,
}

impl DockerNetwork {
    /// creates a new docker network (wil actually be created when deployment.start() is called).
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

/// Represents a process running in a docker container
#[derive(Clone)]
pub struct DockerDeployProcess {
    name: String,
    next_port: Rc<RefCell<u16>>,
    rust_crate: Rc<RefCell<Option<RustCrate>>>,

    exposed_ports: Rc<RefCell<Vec<u16>>>,
    external_targets: Rc<RefCell<Vec<String>>>,

    docker_image_name: Rc<RefCell<Option<String>>>,
    docker_container_name: Rc<RefCell<Option<String>>>,

    network: DockerNetwork,
}

impl Node for DockerDeployProcess {
    type Port = u16;
    type Meta = ();
    type InstantiateEnv = DockerDeploy;

    fn next_port(&self) -> Self::Port {
        let port = {
            let mut borrow = self.next_port.borrow_mut();
            let port = *borrow;
            *borrow += 1;
            port
        };

        port
    }

    fn update_meta(&mut self, meta: &Self::Meta) {
        eprintln!(
            "DockerDeployProcess update_meta. name: {}, meta: {meta:?}",
            self.name
        );
    }

    fn instantiate(
        &self,
        _env: &mut Self::InstantiateEnv,
        meta: &mut Self::Meta,
        graph: DfirGraph,
        extra_stmts: Vec<syn::Stmt>,
    ) {
        // basically the same as
        // create_trybuild_service
        let surface = graph.surface_syntax_string();

        let (bin_name, config) =
            create_graph_trybuild(graph, extra_stmts.clone(), &Some(self.name.clone()), true);

        // create_trybuild_service
        let cloned_config = config.clone();

        let mut ret = RustCrate::new(config.project_dir)
            .target_dir(config.target_dir)
            .bin(bin_name.clone())
            .no_default_features();

        ret = ret.display_name("test_display_name");

        ret = ret.features(vec!["hydro___feature_deploy_integration".to_string()]);

        if let Some(features) = config.features {
            ret = ret.features(features);
        }

        ret = ret.build_env("STAGELEFT_TRYBUILD_BUILD_STAGED", "1");
        ret = ret.config("build.incremental = false");

        eprintln!(
            r#"DockerDeployProcess instantiate. name: {}, meta: {meta:?} extra_stmts: {extra_stmts:?} extra_stmts: {extra_stmts:?} bin_name: {bin_name} config: {cloned_config:?} surface: 

---
---
---
{surface}
---
---
---
{ret:?}
"#,
            self.name
        );

        // if let Some(features) = features {
        //     ret = ret.features(features);
        // }
        *self.rust_crate.borrow_mut() = Some(ret);
    }
}

/// TODO:
#[derive(Clone, Debug)]
pub struct DockerDeployCluster {
    #[allow(unused)] // TODO:
    name: String,
    next_port: Rc<RefCell<u16>>,
}

impl Node for DockerDeployCluster {
    type Port = u16;
    type Meta = ();
    type InstantiateEnv = DockerDeploy;

    fn next_port(&self) -> Self::Port {
        let port = {
            let mut borrow = self.next_port.borrow_mut();
            let port = *borrow;
            *borrow += 1;
            port
        };

        port
    }

    fn update_meta(&mut self, meta: &Self::Meta) {
        eprintln!("DockerDeployCluster update_meta. self: {self:?} meta: {meta:?}");
    }

    fn instantiate(
        &self,
        _env: &mut Self::InstantiateEnv,
        _meta: &mut Self::Meta,
        _graph: DfirGraph,
        _extra_stmts: Vec<syn::Stmt>,
    ) {
        todo!()
    }
}

/// Represents an external process, outside the control of this deployment but still with some communication into this deployment.
#[derive(Clone, Debug)]
pub struct DockerDeployExternal {
    name: String,
    next_port: Rc<RefCell<u16>>,

    ports: Rc<RefCell<HashMap<usize, u16>>>,

    sinks: Rc<RefCell<HashMap<u16, (Rc<RefCell<Option<String>>>, u16, DockerNetwork)>>>,
}

impl Node for DockerDeployExternal {
    type Port = u16;
    type Meta = ();
    type InstantiateEnv = DockerDeploy;

    fn next_port(&self) -> Self::Port {
        let port = {
            let mut borrow = self.next_port.borrow_mut();
            let port = *borrow;
            *borrow += 1;
            port
        };

        port
    }

    fn update_meta(&mut self, meta: &Self::Meta) {
        eprintln!("DockerDeployExternal update_meta. self: {self:?} meta: {meta:?}");
    }

    fn instantiate(
        &self,
        _env: &mut Self::InstantiateEnv,
        meta: &mut Self::Meta,
        graph: DfirGraph,
        extra_stmts: Vec<syn::Stmt>,
    ) {
        let surface = graph.surface_syntax_string();

        eprintln!(
            "DockerDeployExternal instantiate. self: {self:?} meta: {meta:?} extra_stmts: {extra_stmts:?}: surface: {surface}"
        );
    }
}

type DynSourceSink<Out, In, InErr> = (
    Pin<Box<dyn Stream<Item = Out>>>,
    Pin<Box<dyn Sink<In, Error = InErr>>>,
);

impl<'a> RegisterPort<'a, DockerDeploy> for DockerDeployExternal {
    fn register(&self, key: usize, port: <DockerDeploy as Deploy>::Port) {
        eprintln!("Registering external port {key} {port:?}");
        self.ports.borrow_mut().insert(key, port);
    }

    fn raw_port(&self, _key: usize) -> <DockerDeploy as Deploy<'a>>::ExternalRawPort {
        todo!()
    }

    fn as_bytes_bidi(
        &self,
        _key: usize,
    ) -> impl Future<
        Output = DynSourceSink<
            Result<bytes::BytesMut, std::io::Error>,
            bytes::Bytes,
            std::io::Error,
        >,
    > + 'a {
        async { todo!() }
    }

    fn as_bincode_bidi<InT, OutT>(
        &self,
        _key: usize,
    ) -> impl Future<Output = DynSourceSink<OutT, InT, std::io::Error>> + 'a
    where
        InT: serde::Serialize + 'static,
        OutT: serde::de::DeserializeOwned + 'static,
    {
        async { todo!() }
    }

    fn as_bincode_sink<T>(
        &self,
        key: usize,
    ) -> impl Future<Output = Pin<Box<dyn Sink<T, Error = std::io::Error>>>> + 'a
    where
        T: serde::Serialize + 'static,
    {
        let local_port = self.ports.borrow().get(&key).unwrap().clone();
        let (docker_container_name, remote_port, network) =
            self.sinks.borrow().get(&local_port).unwrap().clone();

        async move {
            let docker = Docker::connect_with_local_defaults().unwrap();

            let container_info = docker
                .inspect_container(docker_container_name.borrow().as_ref().unwrap(), None)
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

            eprintln!(
                "as_bincode_sink local_port: {local_port} remote_ip: {remote_ip_address} remote_port: {remote_port}"
            );

            let x = Box::pin(
                LazyTcpSink::new(format!("{remote_ip_address}:{remote_port}")).with(
                    move |v| async move {
                        Ok(bytes::Bytes::copy_from_slice(
                            &bincode::serialize(&v).unwrap(),
                        ))
                    },
                ),
            );

            x as Pin<Box<dyn Sink<T, Error = std::io::Error>>>
        }
    }

    fn as_bincode_source<T>(
        &self,
        key: usize,
    ) -> impl Future<Output = Pin<Box<dyn Stream<Item = T>>>> + 'a
    where
        T: serde::de::DeserializeOwned + 'static,
    {
        eprintln!(
            "as_bincode_source {key} - {}",
            self.ports.borrow().get(&key).unwrap().clone()
        );

        let x = Box::pin(
            LazyTcpSource::new(
                format!("0.0.0.0:{}", self.ports.borrow().get(&key).unwrap())
                    .parse()
                    .unwrap(),
            )
            .map(|v| bincode::deserialize(&v.unwrap()).unwrap()),
        );

        async { x as Pin<Box<dyn Stream<Item = T>>> }
    }
}

/// For deploying to a local docker instance
pub struct DockerDeploy {
    docker_processes: RefCell<Vec<DockerDeployProcess>>,
}

impl DockerDeploy {
    /// Create a new deployment
    pub fn new() -> Self {
        Self {
            docker_processes: RefCell::new(Vec::new()),
        }
    }

    /// Add an internal docker service to the deployment.
    pub fn add_docker(&self, name: String, network: DockerNetwork) -> DockerDeployProcess {
        let process = DockerDeployProcess {
            name,
            next_port: Rc::new(RefCell::new(
                (10000 * (self.docker_processes.borrow().len() + 1)) as u16,
            )),
            rust_crate: Rc::new(RefCell::new(None)),

            exposed_ports: Rc::new(RefCell::new(Vec::new())),
            external_targets: Rc::new(RefCell::new(Vec::new())),

            docker_image_name: Rc::new(RefCell::new(None)),
            docker_container_name: Rc::new(RefCell::new(None)),

            network,
        };

        self.docker_processes.borrow_mut().push(process.clone());

        process
    }

    /// Add an external process to the deployment.
    pub fn add_external(&self, name: String) -> DockerDeployExternal {
        DockerDeployExternal {
            name,
            next_port: Rc::new(RefCell::new(9000)),
            ports: Rc::new(RefCell::new(HashMap::new())),
            sinks: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    /// Create docker images.
    pub async fn provision(&self) -> Result<(), anyhow::Error> {
        for docker_process in self.docker_processes.borrow_mut().iter_mut() {
            let rust_crate = docker_process.rust_crate.borrow_mut().take().unwrap();
            let build_output = match build_crate_memoized(rust_crate.get_build_params()).await {
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
                        r#"Failed to build crate {exit_status:?}

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

            fn get_docker_image_name(host_id: &str, path: &Path) -> String {
                format!(
                    "hydro_image-{host_id}-{}-{}",
                    path.file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_ascii_lowercase()
                        .replace(".", "_")
                        .replace("::", "__"),
                    nanoid::nanoid!(6, &CONTAINER_ALPHABET)
                )
            }

            fn get_docker_container_name(host_id: &str, path: &Path) -> String {
                format!(
                    "hydro_container-{host_id}-{}-{}",
                    path.file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_ascii_lowercase()
                        .replace(".", "_")
                        .replace("::", "__"),
                    nanoid::nanoid!(6, &CONTAINER_ALPHABET)
                )
            }

            let docker = Docker::connect_with_local_defaults()?;
            let image_name = get_docker_image_name(&docker_process.name, &build_output.bin_path);

            // Create tar archive with binary and Dockerfile
            let mut tar_data = Vec::new();
            {
                let mut tar = Builder::new(&mut tar_data);

                // Add Dockerfile
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

                // Add binary
                let mut header = Header::new_gnu();
                header.set_path("app")?;
                header.set_size(build_output.bin_data.len() as u64);
                header.set_mode(0o755);
                header.set_cksum();
                tar.append(&header, &build_output.bin_data[..])?;

                tar.finish()?;
            }

            // Build Docker image
            let build_options = BuildImageOptions {
                dockerfile: "Dockerfile".to_owned(),
                t: image_name.clone(),
                rm: true,
                ..Default::default()
            };

            let mut build_stream = docker.build_image(build_options, None, Some(tar_data.into()));
            while let Some(msg) = build_stream.next().await {
                match msg {
                    Ok(_) => {} // Build progress
                    Err(e) => return Err(anyhow::anyhow!("Docker build failed: {}", e)),
                }
            }

            *docker_process.docker_image_name.borrow_mut() = Some(image_name);
            *docker_process.docker_container_name.borrow_mut() = Some(get_docker_container_name(
                &docker_process.name,
                &build_output.bin_path,
            ));
        }

        Ok(())
    }

    /// Start the deployment, tell docker to create containers from the existing provisionined images.
    pub async fn start(&self) -> Result<(), anyhow::Error> {
        let docker = Docker::connect_with_local_defaults()?;

        let networks = self
            .docker_processes
            .borrow()
            .iter()
            .map(|process| process.network.name.clone())
            .collect::<HashSet<_>>();

        for network in networks {
            match docker
                .create_network(CreateNetworkOptions {
                    name: network,
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
        }

        for process in self.docker_processes.borrow_mut().iter_mut() {
            let docker_image_name = process.docker_image_name.borrow().clone().unwrap();
            let docker_container_name = process.docker_container_name.borrow().clone().unwrap();

            let config = Config {
                image: Some(docker_image_name),
                hostname: Some(process.name.clone()),
                host_config: Some(HostConfig {
                    extra_hosts: Some(
                        process
                            .external_targets
                            .borrow()
                            .iter()
                            .map(|hostname| format!("{hostname}:10.169.214.57"))
                            .collect(),
                    ),
                    ..Default::default()
                }),
                exposed_ports: Some(HashMap::from_iter(
                    process
                        .exposed_ports
                        .borrow()
                        .iter()
                        .map(|port| (format!("{port}/tcp"), HashMap::new())),
                )),
                networking_config: Some(NetworkingConfig {
                    endpoints_config: HashMap::from([(
                        process.network.name.clone(),
                        EndpointSettings {
                            ..Default::default()
                        },
                    )]),
                }),
                ..Default::default()
            };

            let options = CreateContainerOptions {
                name: docker_container_name,
                platform: None,
            };

            docker.create_container(Some(options), config).await?;
            docker
                .start_container(
                    &process.docker_container_name.borrow().clone().unwrap(),
                    None::<StartContainerOptions<String>>,
                )
                .await?;
        }

        Ok(())
    }

    /// Stop the deployment, destroy all containers
    pub async fn stop(&self) -> Result<(), anyhow::Error> {
        let docker = Docker::connect_with_local_defaults()?;

        for process in self.docker_processes.borrow().iter() {
            if let Some(container_name) = process.docker_container_name.borrow().as_ref() {
                docker
                    .kill_container(container_name, None::<KillContainerOptions<String>>)
                    .await?;
            }
        }

        Ok(())
    }
}

impl<'a> Deploy<'a> for DockerDeploy {
    type InstantiateEnv = Self;
    type CompileEnv = ();
    type Process = DockerDeployProcess;
    type Cluster = DockerDeployCluster;
    type External = DockerDeployExternal;
    type Port = u16;
    type ExternalRawPort = ();
    type Meta = ();
    type GraphId = ();

    fn allocate_process_port(process: &Self::Process) -> Self::Port {
        process.next_port()
    }

    fn allocate_cluster_port(cluster: &Self::Cluster) -> Self::Port {
        cluster.next_port()
    }

    fn allocate_external_port(external: &Self::External) -> Self::Port {
        external.next_port()
    }

    fn o2o_sink_source(
        compile_env: &Self::CompileEnv,
        p1: &Self::Process,
        p1_port: &Self::Port,
        p2: &Self::Process,
        p2_port: &Self::Port,
    ) -> (syn::Expr, syn::Expr) {
        eprintln!(
            "o2o_sink_source {}:{p1_port:?} -> {}:{p2_port:?}. compile_env: {compile_env:?} ",
            p1.name, p2.name,
        );

        let bind_addr = format!("0.0.0.0:{}", p2_port);
        let target = format!("{}:{p2_port}", p2.name);

        deploy_containerized_o2o(target.as_str(), bind_addr.as_str())
    }

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
        eprintln!("{serialized}");

        Box::new(move || {
            eprintln!("o2o_connect thunk: {serialized}");
        })
    }

    fn o2m_sink_source(
        _compile_env: &Self::CompileEnv,
        _p1: &Self::Process,
        _p1_port: &Self::Port,
        _c2: &Self::Cluster,
        _c2_port: &Self::Port,
    ) -> (syn::Expr, syn::Expr) {
        todo!()
    }

    fn o2m_connect(
        _p1: &Self::Process,
        _p1_port: &Self::Port,
        _c2: &Self::Cluster,
        _c2_port: &Self::Port,
    ) -> Box<dyn FnOnce()> {
        todo!()
    }

    fn m2o_sink_source(
        _compile_env: &Self::CompileEnv,
        _c1: &Self::Cluster,
        _c1_port: &Self::Port,
        _p2: &Self::Process,
        _p2_port: &Self::Port,
    ) -> (syn::Expr, syn::Expr) {
        todo!()
    }

    fn m2o_connect(
        _c1: &Self::Cluster,
        _c1_port: &Self::Port,
        _p2: &Self::Process,
        _p2_port: &Self::Port,
    ) -> Box<dyn FnOnce()> {
        todo!()
    }

    fn m2m_sink_source(
        _compile_env: &Self::CompileEnv,
        _c1: &Self::Cluster,
        _c1_port: &Self::Port,
        _c2: &Self::Cluster,
        _c2_port: &Self::Port,
    ) -> (syn::Expr, syn::Expr) {
        todo!()
    }

    fn m2m_connect(
        _c1: &Self::Cluster,
        _c1_port: &Self::Port,
        _c2: &Self::Cluster,
        _c2_port: &Self::Port,
    ) -> Box<dyn FnOnce()> {
        todo!()
    }

    fn e2o_many_source(
        _compile_env: &Self::CompileEnv,
        _extra_stmts: &mut Vec<syn::Stmt>,
        _p2: &Self::Process,
        _p2_port: &Self::Port,
        _codec_type: &syn::Type,
        _shared_handle: String,
    ) -> syn::Expr {
        todo!()
    }

    fn e2o_many_sink(_shared_handle: String) -> syn::Expr {
        todo!()
    }

    fn e2o_source(
        _compile_env: &Self::CompileEnv,
        p1: &Self::External,
        p1_port: &Self::Port,
        p2: &Self::Process,
        p2_port: &Self::Port,
    ) -> syn::Expr {
        eprintln!(
            "e2o_source {}:{p1_port:?} -> {}:{p2_port:?}",
            p1.name, p2.name
        );

        let bind_addr = format!("0.0.0.0:{}", p2_port);

        p1.sinks.borrow_mut().insert(
            *p1_port,
            (
                p2.docker_container_name.clone(),
                *p2_port,
                p2.network.clone(),
            ),
        );

        p2.exposed_ports.borrow_mut().push(*p2_port);

        deploy_containerized_e2o(bind_addr.as_str())
    }

    fn e2o_connect(
        p1: &Self::External,
        p1_port: &Self::Port,
        p2: &Self::Process,
        p2_port: &Self::Port,
        many: bool,
        server_hint: NetworkHint,
    ) -> Box<dyn FnOnce()> {
        let serialized = format!(
            "e2o_connect many: {many}, server_hint: {server_hint:?}, {}:{p1_port:?} -> {}:{p2_port:?}",
            p1.name, p2.name
        );
        eprintln!("{serialized}");

        Box::new(move || {
            eprintln!("e2o_connect thunk: {serialized}");
        })
    }

    fn o2e_sink(
        compile_env: &Self::CompileEnv,
        p1: &Self::Process,
        p1_port: &Self::Port,
        p2: &Self::External,
        p2_port: &Self::Port,
    ) -> syn::Expr {
        eprintln!(
            "o2e_sink {}:{p1_port:?} -> {}:{p2_port:?}. compile_env: {compile_env:?} ",
            p1.name, p2.name
        );

        let target = format!("{}:{p2_port}", p2.name);

        p1.external_targets.borrow_mut().push(p2.name.clone());

        deploy_containerized_o2e(target.as_str())
    }

    fn o2e_connect(
        p1: &Self::Process,
        p1_port: &Self::Port,
        p2: &Self::External,
        p2_port: &Self::Port,
    ) -> Box<dyn FnOnce()> {
        let serialized = format!(
            "o2e_connect {}:{p1_port:?} -> {}:{p2_port:?}",
            p1.name, p2.name
        );
        eprintln!("{serialized}");

        Box::new(move || {
            eprintln!("o2e_connect thunk: {serialized}");
        })
    }

    fn cluster_ids(
        _env: &Self::CompileEnv,
        _of_cluster: usize,
    ) -> impl QuotedWithContext<'a, &'a [u32], ()> + Copy + 'a {
        q!(todo!()) // TODO:
    }

    fn cluster_self_id(_env: &Self::CompileEnv) -> impl QuotedWithContext<'a, u32, ()> + Copy + 'a {
        q!(todo!()) // TODO:
    }
}

impl<'a> ProcessSpec<'a, DockerDeploy> for DockerDeployProcess {
    fn build(self, _id: usize, _name_hint: &'_ str) -> <DockerDeploy as Deploy<'a>>::Process {
        self
    }
}

impl<'a> ProcessSpec<'a, DockerDeploy> for DockerDeployCluster {
    fn build(self, _id: usize, _name_hint: &str) -> <DockerDeploy as Deploy<'a>>::Process {
        todo!()
    }
}

impl<'a> ExternalSpec<'a, DockerDeploy> for DockerDeployExternal {
    fn build(self, _id: usize, _name_hint: &str) -> <DockerDeploy as Deploy<'a>>::External {
        self
    }
}
