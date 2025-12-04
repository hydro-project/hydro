use std::any::type_name;
use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

#[cfg(feature = "build")]
use super::compiled::CompiledFlow;
#[cfg(feature = "build")]
use super::deploy::{DeployFlow, DeployResult};
#[cfg(feature = "build")]
use super::deploy_provider::{ClusterSpec, Deploy, ExternalSpec, IntoProcessSpec};
use super::ir::HydroRoot;
use crate::location::{Cluster, External, Process};
#[cfg(feature = "sim")]
#[cfg(stageleft_runtime)]
use crate::sim::flow::SimFlow;
use crate::staging_util::Invariant;

pub(crate) type FlowState = Rc<RefCell<FlowStateInner>>;

pub(crate) struct FlowStateInner {
    /// Tracks the roots of the dataflow IR. This is referenced by
    /// `Stream` and `HfCycle` to build the IR. The inner option will
    /// be set to `None` when this builder is finalized.
    pub(crate) roots: Option<Vec<HydroRoot>>,

    /// Counter for generating unique external output identifiers.
    pub(crate) next_external_out: usize,

    /// Counters for generating identifiers for cycles.
    pub(crate) cycle_counts: usize,

    /// Counters for clock IDs.
    pub(crate) next_clock_id: usize,
}

impl FlowStateInner {
    pub fn next_cycle_id(&mut self) -> usize {
        let id = self.cycle_counts;
        self.cycle_counts += 1;
        id
    }

    pub fn push_root(&mut self, root: HydroRoot) {
        self.roots
            .as_mut()
            .expect("Attempted to add a root to a flow that has already been finalized. No roots can be added after the flow has been compiled.")
            .push(root);
    }
}

#[expect(missing_docs, reason = "TODO")]
pub struct FlowBuilder<'a> {
    flow_state: FlowState,
    processes: RefCell<Vec<(usize, String)>>,
    clusters: RefCell<Vec<(usize, String)>>,
    externals: RefCell<Vec<(usize, String)>>,

    next_location_id: RefCell<usize>,

    /// Tracks whether this flow has been finalized; it is an error to
    /// drop without finalizing.
    finalized: bool,

    /// 'a on a FlowBuilder is used to ensure that staged code does not
    /// capture more data that it is allowed to; 'a is generated at the
    /// entrypoint of the staged code and we keep it invariant here
    /// to enforce the appropriate constraints
    _phantom: Invariant<'a>,
}

impl Drop for FlowBuilder<'_> {
    fn drop(&mut self) {
        if !self.finalized && !std::thread::panicking() {
            panic!(
                "Dropped FlowBuilder without finalizing, you may have forgotten to call `with_default_optimize`, `optimize_with`, or `finalize`."
            );
        }
    }
}

#[expect(missing_docs, reason = "TODO")]
impl<'a> FlowBuilder<'a> {
    #[expect(
        clippy::new_without_default,
        reason = "call `new` explicitly, not `default`"
    )]
    pub fn new() -> FlowBuilder<'a> {
        FlowBuilder {
            flow_state: Rc::new(RefCell::new(FlowStateInner {
                roots: Some(vec![]),
                next_external_out: 0,
                cycle_counts: 0,
                next_clock_id: 0,
            })),
            processes: RefCell::new(vec![]),
            clusters: RefCell::new(vec![]),
            externals: RefCell::new(vec![]),
            next_location_id: RefCell::new(0),
            finalized: false,
            _phantom: PhantomData,
        }
    }

    pub fn rewritten_ir_builder<'b>(&self) -> RewriteIrFlowBuilder<'b> {
        let processes = self.processes.borrow().clone();
        let clusters = self.clusters.borrow().clone();
        let externals = self.externals.borrow().clone();
        let next_location_id = *self.next_location_id.borrow();
        RewriteIrFlowBuilder {
            builder: FlowBuilder {
                flow_state: Rc::new(RefCell::new(FlowStateInner {
                    roots: None,
                    next_external_out: 0,
                    cycle_counts: 0,
                    next_clock_id: 0,
                })),
                processes: RefCell::new(processes),
                clusters: RefCell::new(clusters),
                externals: RefCell::new(externals),
                next_location_id: RefCell::new(next_location_id),
                finalized: false,
                _phantom: PhantomData,
            },
        }
    }

    pub(crate) fn flow_state(&self) -> &FlowState {
        &self.flow_state
    }

    pub fn process<P>(&self) -> Process<'a, P> {
        let mut next_location_id = self.next_location_id.borrow_mut();
        let id = *next_location_id;
        *next_location_id += 1;

        self.processes
            .borrow_mut()
            .push((id, type_name::<P>().to_string()));

        Process {
            id,
            flow_state: self.flow_state().clone(),
            _phantom: PhantomData,
        }
    }

    pub fn external<P>(&self) -> External<'a, P> {
        let mut next_location_id = self.next_location_id.borrow_mut();
        let id = *next_location_id;
        *next_location_id += 1;

        self.externals
            .borrow_mut()
            .push((id, type_name::<P>().to_string()));

        External {
            id,
            flow_state: self.flow_state().clone(),
            _phantom: PhantomData,
        }
    }

    pub fn cluster<C>(&self) -> Cluster<'a, C> {
        let mut next_location_id = self.next_location_id.borrow_mut();
        let id = *next_location_id;
        *next_location_id += 1;

        self.clusters
            .borrow_mut()
            .push((id, type_name::<C>().to_string()));

        Cluster {
            id,
            flow_state: self.flow_state().clone(),
            _phantom: PhantomData,
        }
    }
}

#[cfg(feature = "build")]
#[cfg_attr(docsrs, doc(cfg(feature = "build")))]
#[expect(missing_docs, reason = "TODO")]
impl<'a> FlowBuilder<'a> {
    pub fn finalize(mut self) -> super::built::BuiltFlow<'a> {
        self.finalized = true;

        super::built::BuiltFlow {
            ir: self.flow_state.borrow_mut().roots.take().unwrap(),
            process_id_name: self.processes.replace(vec![]),
            cluster_id_name: self.clusters.replace(vec![]),
            external_id_name: self.externals.replace(vec![]),
            _phantom: PhantomData,
        }
    }

    pub fn with_default_optimize<D: Deploy<'a>>(self) -> DeployFlow<'a, D> {
        self.finalize().with_default_optimize()
    }

    pub fn optimize_with(self, f: impl FnOnce(&mut [HydroRoot])) -> super::built::BuiltFlow<'a> {
        self.finalize().optimize_with(f)
    }

    pub fn with_process<P, D: Deploy<'a>>(
        self,
        process: &Process<P>,
        spec: impl IntoProcessSpec<'a, D>,
    ) -> DeployFlow<'a, D> {
        self.with_default_optimize().with_process(process, spec)
    }

    pub fn with_remaining_processes<D: Deploy<'a>, S: IntoProcessSpec<'a, D> + 'a>(
        self,
        spec: impl Fn() -> S,
    ) -> DeployFlow<'a, D> {
        self.with_default_optimize().with_remaining_processes(spec)
    }

    pub fn with_external<P, D: Deploy<'a>>(
        self,
        process: &External<P>,
        spec: impl ExternalSpec<'a, D>,
    ) -> DeployFlow<'a, D> {
        self.with_default_optimize().with_external(process, spec)
    }

    pub fn with_remaining_externals<D: Deploy<'a>, S: ExternalSpec<'a, D> + 'a>(
        self,
        spec: impl Fn() -> S,
    ) -> DeployFlow<'a, D> {
        self.with_default_optimize().with_remaining_externals(spec)
    }

    pub fn with_cluster<C, D: Deploy<'a>>(
        self,
        cluster: &Cluster<C>,
        spec: impl ClusterSpec<'a, D>,
    ) -> DeployFlow<'a, D> {
        self.with_default_optimize().with_cluster(cluster, spec)
    }

    pub fn with_remaining_clusters<D: Deploy<'a>, S: ClusterSpec<'a, D> + 'a>(
        self,
        spec: impl Fn() -> S,
    ) -> DeployFlow<'a, D> {
        self.with_default_optimize().with_remaining_clusters(spec)
    }

    pub fn compile<D: Deploy<'a>>(self) -> CompiledFlow<'a, D::GraphId> {
        self.with_default_optimize::<D>().compile()
    }

    pub fn compile_no_network<D: Deploy<'a>>(self) -> CompiledFlow<'a, D::GraphId> {
        self.with_default_optimize::<D>().compile_no_network()
    }

    pub fn deploy<D: Deploy<'a>>(self, env: &mut D::InstantiateEnv) -> DeployResult<'a, D> {
        self.with_default_optimize().deploy(env)
    }

    #[cfg(feature = "sim")]
    /// Creates a simulation for this builder, which can be used to run deterministic simulations
    /// of the Hydro program.
    pub fn sim(self) -> SimFlow<'a> {
        self.finalize().sim()
    }
}

#[expect(missing_docs, reason = "TODO")]
pub struct RewriteIrFlowBuilder<'a> {
    builder: FlowBuilder<'a>,
}

#[expect(missing_docs, reason = "TODO")]
impl<'a> RewriteIrFlowBuilder<'a> {
    pub fn build_with(
        self,
        thunk: impl FnOnce(&FlowBuilder<'a>) -> Vec<HydroRoot>,
    ) -> FlowBuilder<'a> {
        let roots = thunk(&self.builder);
        self.builder.flow_state().borrow_mut().roots = Some(roots);
        self.builder
    }
}
