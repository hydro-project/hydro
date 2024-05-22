use std::cell::RefCell;
use std::rc::Rc;

use hydroflow_plus::location::{
    Cluster, ClusterSpec, Deploy, HfSendManyToMany, HfSendManyToOne, HfSendOneToMany,
    HfSendOneToOne, Location, ProcessSpec,
};
use hydroflow_plus::util::cli::{
    ConnectedDemux, ConnectedDirect, ConnectedSink, ConnectedSource, ConnectedTagged, HydroCLI,
};
use stageleft::{q, Quoted, RuntimeData};

use super::HydroflowPlusMeta;

pub struct CLIRuntime {}

impl<'a> Deploy<'a> for CLIRuntime {
    type ClusterId = u32;
    type Process = CLIRuntimeNode<'a>;
    type Cluster = CLIRuntimeCluster<'a>;
    type Meta = ();
    type GraphId = usize;
    type ProcessPort = String;
    type ClusterPort = String;
}

#[derive(Clone)]
pub struct CLIRuntimeNode<'a> {
    id: usize,
    next_port: Rc<RefCell<usize>>,
    cli: RuntimeData<&'a HydroCLI<HydroflowPlusMeta>>,
}

impl<'a> Location for CLIRuntimeNode<'a> {
    type Port = String;
    type Meta = ();

    fn id(&self) -> usize {
        self.id
    }

    fn next_port(&self) -> String {
        let next_send_port = *self.next_port.borrow();
        *self.next_port.borrow_mut() += 1;
        format!("port_{}", next_send_port)
    }

    fn update_meta(&mut self, _meta: &Self::Meta) {}
}

#[derive(Clone)]
pub struct CLIRuntimeCluster<'a> {
    id: usize,
    next_port: Rc<RefCell<usize>>,
    cli: RuntimeData<&'a HydroCLI<HydroflowPlusMeta>>,
}

impl<'a> Location for CLIRuntimeCluster<'a> {
    type Port = String;
    type Meta = ();

    fn id(&self) -> usize {
        self.id
    }

    fn next_port(&self) -> String {
        let next_send_port = *self.next_port.borrow();
        *self.next_port.borrow_mut() += 1;
        format!("port_{}", next_send_port)
    }

    fn update_meta(&mut self, _meta: &Self::Meta) {}
}

impl<'a> Cluster<'a> for CLIRuntimeCluster<'a> {
    type Id = u32;

    fn ids(&self) -> impl Quoted<'a, &'a Vec<u32>> + Copy + 'a {
        let cli = self.cli;
        let self_id = self.id;
        q!(cli.meta.clusters.get(&self_id).unwrap())
    }

    fn self_id(&self) -> impl Quoted<'a, u32> + Copy + 'a {
        let cli = self.cli;
        q!(cli
            .meta
            .cluster_id
            .expect("Tried to read Cluster ID on a non-cluster node"))
    }
}

impl<'a> HfSendOneToOne<CLIRuntimeNode<'a>> for CLIRuntimeNode<'a> {
    fn connect(&self, _other: &CLIRuntimeNode, _source_port: &String, _recipient_port: &String) {}

    fn gen_sink_statement(&self, port: &String) -> syn::Expr {
        let self_cli = self.cli;
        let port = port.as_str();
        q!({
            self_cli
                .port(port)
                .connect_local_blocking::<ConnectedDirect>()
                .into_sink()
        })
        .splice()
    }

    fn gen_source_statement(other: &CLIRuntimeNode<'a>, port: &String) -> syn::Expr {
        let self_cli = other.cli;
        let port = port.as_str();
        q!({
            self_cli
                .port(port)
                .connect_local_blocking::<ConnectedDirect>()
                .into_source()
        })
        .splice()
    }
}

impl<'a> HfSendManyToOne<CLIRuntimeNode<'a>, u32> for CLIRuntimeCluster<'a> {
    fn connect(&self, _other: &CLIRuntimeNode, _source_port: &String, _recipient_port: &String) {}

    fn gen_sink_statement(&self, port: &String) -> syn::Expr {
        let self_cli = self.cli;
        let port = port.as_str();
        q!({
            self_cli
                .port(port)
                .connect_local_blocking::<ConnectedDirect>()
                .into_sink()
        })
        .splice()
    }

    fn gen_source_statement(other: &CLIRuntimeNode<'a>, port: &String) -> syn::Expr {
        let self_cli = other.cli;
        let port = port.as_str();
        q!({
            self_cli
                .port(port)
                .connect_local_blocking::<ConnectedTagged<ConnectedDirect>>()
                .into_source()
        })
        .splice()
    }
}

impl<'a> HfSendOneToMany<CLIRuntimeCluster<'a>, u32> for CLIRuntimeNode<'a> {
    fn connect(&self, _other: &CLIRuntimeCluster, _source_port: &String, _recipient_port: &String) {
    }

    fn gen_sink_statement(&self, port: &String) -> syn::Expr {
        let self_cli = self.cli;
        let port = port.as_str();

        q!({
            self_cli
                .port(port)
                .connect_local_blocking::<ConnectedDemux<ConnectedDirect>>()
                .into_sink()
        })
        .splice()
    }

    fn gen_source_statement(other: &CLIRuntimeCluster<'a>, port: &String) -> syn::Expr {
        let self_cli = other.cli;
        let port = port.as_str();

        q!({
            self_cli
                .port(port)
                .connect_local_blocking::<ConnectedDirect>()
                .into_source()
        })
        .splice()
    }
}

impl<'a> HfSendManyToMany<CLIRuntimeCluster<'a>, u32> for CLIRuntimeCluster<'a> {
    fn connect(&self, _other: &CLIRuntimeCluster, _source_port: &String, _recipient_port: &String) {
    }

    fn gen_sink_statement(&self, port: &String) -> syn::Expr {
        let self_cli = self.cli;
        let port = port.as_str();

        q!({
            self_cli
                .port(port)
                .connect_local_blocking::<ConnectedDemux<ConnectedDirect>>()
                .into_sink()
        })
        .splice()
    }

    fn gen_source_statement(other: &CLIRuntimeCluster<'a>, port: &String) -> syn::Expr {
        let self_cli = other.cli;
        let port = port.as_str();

        q!({
            self_cli
                .port(port)
                .connect_local_blocking::<ConnectedTagged<ConnectedDirect>>()
                .into_source()
        })
        .splice()
    }
}

impl<'cli> ProcessSpec<'cli, CLIRuntime> for RuntimeData<&'cli HydroCLI<HydroflowPlusMeta>> {
    fn build(&self, id: usize, _meta: &mut ()) -> CLIRuntimeNode<'cli> {
        CLIRuntimeNode {
            id,
            next_port: Rc::new(RefCell::new(0)),
            cli: *self,
        }
    }
}

impl<'cli> ClusterSpec<'cli, CLIRuntime> for RuntimeData<&'cli HydroCLI<HydroflowPlusMeta>> {
    fn build(&self, id: usize, _meta: &mut ()) -> CLIRuntimeCluster<'cli> {
        CLIRuntimeCluster {
            id,
            next_port: Rc::new(RefCell::new(0)),
            cli: *self,
        }
    }
}
