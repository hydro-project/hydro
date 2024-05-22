use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use async_channel::Receiver;
use hydro_deploy::custom_service::CustomClientPort;
use hydro_deploy::hydroflow_crate::ports::{
    DemuxSink, HydroflowSink, HydroflowSource, TaggedSource,
};
use hydro_deploy::hydroflow_crate::HydroflowCrateService;
use hydro_deploy::{Deployment, Host};
use hydroflow_plus::location::{
    Cluster, ClusterSpec, Deploy, HfSendManyToMany, HfSendManyToOne, HfSendOneToMany,
    HfSendOneToOne, Location, ProcessSpec,
};
use stageleft::internal::syn::parse_quote;
use stageleft::q;
use tokio::sync::RwLock;

use super::HydroflowPlusMeta;

pub struct HydroDeploy {}

impl<'a> Deploy<'a> for HydroDeploy {
    type ClusterId = u32;
    type Process = DeployNode;
    type Cluster = DeployCluster;
    type Meta = HashMap<usize, Vec<u32>>;
    type GraphId = ();
    type ProcessPort = DeployPort<DeployNode>;
    type ClusterPort = DeployPort<DeployCluster>;
}

pub trait DeployCrateWrapper {
    fn underlying(&self) -> Arc<RwLock<HydroflowCrateService>>;

    #[allow(async_fn_in_trait)]
    async fn create_sender(
        &self,
        port: &str,
        deployment: &mut Deployment,
        on: &Arc<RwLock<impl Host + 'static>>,
    ) -> CustomClientPort {
        let sender_service = deployment.CustomService(on.clone(), vec![]);
        let mut sender_port = sender_service.read().await.declare_client(&sender_service);
        let mut recipient = self
            .underlying()
            .read()
            .await
            .get_port(port.to_string(), &self.underlying());

        sender_port.send_to(&mut recipient);
        sender_port
    }

    #[allow(async_fn_in_trait)]
    async fn stdout(&self) -> Receiver<String> {
        self.underlying().read().await.stdout().await
    }

    #[allow(async_fn_in_trait)]
    async fn stderr(&self) -> Receiver<String> {
        self.underlying().read().await.stderr().await
    }
}

#[derive(Clone)]
pub struct DeployNode {
    id: usize,
    next_port: Rc<RefCell<usize>>,
    underlying: Arc<RwLock<HydroflowCrateService>>,
}

impl DeployCrateWrapper for DeployNode {
    fn underlying(&self) -> Arc<RwLock<HydroflowCrateService>> {
        self.underlying.clone()
    }
}

pub struct DeployPort<N> {
    node: N,
    port: String,
}

impl DeployPort<DeployNode> {
    pub async fn create_sender(
        &self,
        deployment: &mut Deployment,
        on: &Arc<RwLock<impl Host + 'static>>,
    ) -> CustomClientPort {
        self.node.create_sender(&self.port, deployment, on).await
    }
}

impl DeployPort<DeployCluster> {
    pub async fn create_senders(
        &self,
        deployment: &mut Deployment,
        on: &Arc<RwLock<impl Host + 'static>>,
    ) -> Vec<CustomClientPort> {
        let mut out = vec![];
        for member in &self.node.members {
            out.push(member.create_sender(&self.port, deployment, on).await);
        }

        out
    }
}

impl Location for DeployNode {
    type Port = DeployPort<Self>;
    type Meta = HashMap<usize, Vec<u32>>;

    fn id(&self) -> usize {
        self.id
    }

    fn next_port(&self) -> DeployPort<Self> {
        let next_port = *self.next_port.borrow();
        *self.next_port.borrow_mut() += 1;

        DeployPort {
            node: self.clone(),
            port: format!("port_{}", next_port),
        }
    }

    fn update_meta(&mut self, meta: &Self::Meta) {
        let mut n = self.underlying.try_write().unwrap();
        n.update_meta(HydroflowPlusMeta {
            clusters: meta.clone(),
            cluster_id: None,
            subgraph_id: self.id,
        });
    }
}

#[derive(Clone)]
pub struct DeployClusterNode {
    underlying: Arc<RwLock<HydroflowCrateService>>,
}

impl DeployCrateWrapper for DeployClusterNode {
    fn underlying(&self) -> Arc<RwLock<HydroflowCrateService>> {
        self.underlying.clone()
    }
}

#[derive(Clone)]
pub struct DeployCluster {
    id: usize,
    next_port: Rc<RefCell<usize>>,
    pub members: Vec<DeployClusterNode>,
}

impl Location for DeployCluster {
    type Port = DeployPort<Self>;
    type Meta = HashMap<usize, Vec<u32>>;

    fn id(&self) -> usize {
        self.id
    }

    fn next_port(&self) -> DeployPort<Self> {
        let next_port = *self.next_port.borrow();
        *self.next_port.borrow_mut() += 1;

        DeployPort {
            node: self.clone(),
            port: format!("port_{}", next_port),
        }
    }

    fn update_meta(&mut self, meta: &Self::Meta) {
        self.members.iter().enumerate().for_each(|(cluster_id, n)| {
            let mut n = n.underlying.try_write().unwrap();
            n.update_meta(&HydroflowPlusMeta {
                clusters: meta.clone(),
                cluster_id: Some(cluster_id as u32),
                subgraph_id: self.id,
            });
        });
    }
}

impl<'a> Cluster<'a> for DeployCluster {
    type Id = u32;

    fn ids(&self) -> impl stageleft::Quoted<'a, &'a Vec<u32>> + Copy + 'a {
        q!(panic!())
    }

    fn self_id(&self) -> impl stageleft::Quoted<'a, Self::Id> + Copy + 'a {
        q!(panic!())
    }
}

impl HfSendOneToOne<DeployNode> for DeployNode {
    fn connect(
        &self,
        other: &DeployNode,
        source_port: &DeployPort<DeployNode>,
        recipient_port: &DeployPort<DeployNode>,
    ) {
        let mut source_port = self
            .underlying
            .try_read()
            .unwrap()
            .get_port(source_port.port.clone(), &self.underlying);

        let mut recipient_port = other
            .underlying
            .try_read()
            .unwrap()
            .get_port(recipient_port.port.clone(), &other.underlying);

        source_port.send_to(&mut recipient_port);
    }

    fn gen_sink_statement(&self, _port: &Self::Port) -> syn::Expr {
        parse_quote!(null)
    }

    fn gen_source_statement(_other: &DeployNode, _port: &Self::Port) -> syn::Expr {
        parse_quote!(null)
    }
}

impl HfSendManyToOne<DeployNode, u32> for DeployCluster {
    fn connect(
        &self,
        other: &DeployNode,
        source_port: &DeployPort<DeployCluster>,
        recipient_port: &DeployPort<DeployNode>,
    ) {
        let mut recipient_port = other
            .underlying
            .try_read()
            .unwrap()
            .get_port(recipient_port.port.clone(), &other.underlying)
            .merge();

        for (i, node) in self.members.iter().enumerate() {
            let source_port = node
                .underlying
                .try_read()
                .unwrap()
                .get_port(source_port.port.clone(), &node.underlying);

            TaggedSource {
                source: Arc::new(RwLock::new(source_port)),
                tag: i as u32,
            }
            .send_to(&mut recipient_port);
        }
    }

    fn gen_sink_statement(&self, _port: &Self::Port) -> syn::Expr {
        parse_quote!(null)
    }

    fn gen_source_statement(_other: &DeployNode, _port: &DeployPort<DeployNode>) -> syn::Expr {
        parse_quote!(null)
    }
}

impl HfSendOneToMany<DeployCluster, u32> for DeployNode {
    fn connect(
        &self,
        other: &DeployCluster,
        source_port: &DeployPort<DeployNode>,
        recipient_port: &DeployPort<DeployCluster>,
    ) {
        let mut source_port = self
            .underlying
            .try_read()
            .unwrap()
            .get_port(source_port.port.clone(), &self.underlying);

        let mut recipient_port = DemuxSink {
            demux: other
                .members
                .iter()
                .enumerate()
                .map(|(id, c)| {
                    let n = c.underlying.try_read().unwrap();
                    (
                        id as u32,
                        Arc::new(RwLock::new(
                            n.get_port(recipient_port.port.clone(), &c.underlying),
                        )) as Arc<RwLock<dyn HydroflowSink + 'static>>,
                    )
                })
                .collect(),
        };

        source_port.send_to(&mut recipient_port);
    }

    fn gen_sink_statement(&self, _port: &Self::Port) -> syn::Expr {
        parse_quote!(null)
    }

    fn gen_source_statement(
        _other: &DeployCluster,
        _port: &DeployPort<DeployCluster>,
    ) -> syn::Expr {
        parse_quote!(null)
    }
}

impl HfSendManyToMany<DeployCluster, u32> for DeployCluster {
    fn connect(
        &self,
        other: &DeployCluster,
        source_port: &DeployPort<DeployCluster>,
        recipient_port: &DeployPort<DeployCluster>,
    ) {
        for (i, sender) in self.members.iter().enumerate() {
            let source_port = sender
                .underlying
                .try_read()
                .unwrap()
                .get_port(source_port.port.clone(), &sender.underlying);

            let mut recipient_port = DemuxSink {
                demux: other
                    .members
                    .iter()
                    .enumerate()
                    .map(|(id, c)| {
                        let n = c.underlying.try_read().unwrap();
                        (
                            id as u32,
                            Arc::new(RwLock::new(
                                n.get_port(recipient_port.port.clone(), &c.underlying)
                                    .merge(),
                            ))
                                as Arc<RwLock<dyn HydroflowSink + 'static>>,
                        )
                    })
                    .collect(),
            };

            TaggedSource {
                source: Arc::new(RwLock::new(source_port)),
                tag: i as u32,
            }
            .send_to(&mut recipient_port);
        }
    }

    fn gen_sink_statement(&self, _port: &Self::Port) -> syn::Expr {
        parse_quote!(null)
    }

    fn gen_source_statement(
        _other: &DeployCluster,
        _port: &DeployPort<DeployCluster>,
    ) -> syn::Expr {
        parse_quote!(null)
    }
}

type CrateBuilder<'a> = dyn FnMut() -> Arc<RwLock<HydroflowCrateService>> + 'a;

pub struct DeployProcessSpec<'a>(RefCell<Box<CrateBuilder<'a>>>);

impl<'a> DeployProcessSpec<'a> {
    pub fn new<F: FnMut() -> Arc<RwLock<HydroflowCrateService>> + 'a>(f: F) -> Self {
        Self(RefCell::new(Box::new(f)))
    }
}

impl<'a: 'b, 'b> ProcessSpec<'a, HydroDeploy> for DeployProcessSpec<'b> {
    fn build(&self, id: usize, _meta: &mut HashMap<usize, Vec<u32>>) -> DeployNode {
        DeployNode {
            id,
            next_port: Rc::new(RefCell::new(0)),
            underlying: (self.0.borrow_mut())(),
        }
    }
}

type ClusterSpecFn<'a> = dyn FnMut() -> Vec<Arc<RwLock<HydroflowCrateService>>> + 'a;

pub struct DeployClusterSpec<'a>(RefCell<Box<ClusterSpecFn<'a>>>);

impl<'a> DeployClusterSpec<'a> {
    pub fn new<F: FnMut() -> Vec<Arc<RwLock<HydroflowCrateService>>> + 'a>(f: F) -> Self {
        Self(RefCell::new(Box::new(f)))
    }
}

impl<'a: 'b, 'b> ClusterSpec<'a, HydroDeploy> for DeployClusterSpec<'b> {
    fn build(&self, id: usize, meta: &mut HashMap<usize, Vec<u32>>) -> DeployCluster {
        let cluster_nodes = (self.0.borrow_mut())();
        meta.insert(id, (0..(cluster_nodes.len() as u32)).collect());

        DeployCluster {
            id,
            next_port: Rc::new(RefCell::new(0)),
            members: cluster_nodes
                .into_iter()
                .map(|u| DeployClusterNode { underlying: u })
                .collect(),
        }
    }
}
