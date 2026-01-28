use std::any::type_name;
use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

use slotmap::{SecondaryMap, SlotMap};

#[cfg(feature = "build")]
use super::compiled::CompiledFlow;
#[cfg(feature = "build")]
use super::deploy::{DeployFlow, DeployResult};
#[cfg(feature = "build")]
use super::deploy_provider::{ClusterSpec, Deploy, ExternalSpec, IntoProcessSpec};
use super::ir::HydroRoot;
use crate::location::{Cluster, External, LocationKey, LocationType, Process};
#[cfg(feature = "sim")]
#[cfg(stageleft_runtime)]
use crate::sim::flow::SimFlow;
use crate::staging_util::Invariant;

crate::newtype_counter! {
    /// Counter for generating unique external output identifiers.
    pub struct ExternalPortId(usize);
}

pub(crate) type FlowState = Rc<RefCell<FlowStateInner>>;

pub(crate) struct FlowStateInner {
    /// Tracks the roots of the dataflow IR. This is referenced by
    /// `Stream` and `HfCycle` to build the IR. The inner option will
    /// be set to `None` when this builder is finalized.
    pub(crate) roots: Option<Vec<HydroRoot>>,

    /// Counter for generating unique external output identifiers.
    pub(crate) next_external_port: ExternalPortId,

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

pub struct FlowBuilder<'a> {
    flow_state: FlowState,

    /// Locations and their type.
    locations: SlotMap<LocationKey, LocationType>,
    /// Map from raw location ID to name (including externals).
    location_names: SecondaryMap<LocationKey, String>,

    /// Application name used in telemetry.
    #[cfg_attr(
        not(feature = "build"),
        expect(dead_code, reason = "unused without build")
    )]
    flow_name: String,

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
        let mut name = std::env::var("CARGO_PKG_NAME").unwrap_or_else(|_| "unknown".to_owned());
        if let Ok(bin_path) = std::env::current_exe()
            && let Some(bin_name) = bin_path.file_stem()
        {
            name = format!("{}/{}", name, bin_name.display());
        }
        Self::with_name(name)
    }

    pub fn with_name(name: impl Into<String>) -> Self {
        FlowBuilder {
            flow_state: Rc::new(RefCell::new(FlowStateInner {
                roots: Some(vec![]),
                next_external_port: ExternalPortId(0),
                cycle_counts: 0,
                next_clock_id: 0,
            })),
            locations: SlotMap::with_key(),
            location_names: SecondaryMap::new(),
            flow_name: name.into(),
            finalized: false,
            _phantom: PhantomData,
        }
    }

    pub(crate) fn flow_state(&self) -> &FlowState {
        &self.flow_state
    }

    pub fn process<P>(&mut self) -> Process<'a, P> {
        let key = self.locations.insert(LocationType::Process);
        self.location_names
            .insert(key, type_name::<P>().to_string());
        Process {
            key,
            flow_state: self.flow_state().clone(),
            _phantom: PhantomData,
        }
    }

    pub fn cluster<C>(&mut self) -> Cluster<'a, C> {
        let key = self.locations.insert(LocationType::Cluster);
        self.location_names
            .insert(key, type_name::<C>().to_string());
        Cluster {
            key,
            flow_state: self.flow_state().clone(),
            _phantom: PhantomData,
        }
    }

    pub fn external<E>(&mut self) -> External<'a, E> {
        let key = self.locations.insert(LocationType::External);
        self.location_names
            .insert(key, type_name::<E>().to_string());
        External {
            key,
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
            locations: std::mem::take(&mut self.locations),
            location_names: std::mem::take(&mut self.location_names),
            flow_name: std::mem::take(&mut self.flow_name),
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

    pub fn compile<D: Deploy<'a>>(self) -> CompiledFlow<'a> {
        self.with_default_optimize::<D>().compile()
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

    pub fn from_built<'b>(built: &super::built::BuiltFlow) -> FlowBuilder<'b> {
        FlowBuilder {
            flow_state: Rc::new(RefCell::new(FlowStateInner {
                roots: None,
                next_external_port: ExternalPortId(0),
                cycle_counts: 0,
                next_clock_id: 0,
            })),
            locations: built.locations.clone(),
            location_names: built.location_names.clone(),
            flow_name: built.flow_name.clone(),
            finalized: false,
            _phantom: PhantomData,
        }
    }

    #[doc(hidden)] // TODO(mingwei): This is an unstable API for now
    pub fn replace_ir(&mut self, roots: Vec<HydroRoot>) {
        self.flow_state.borrow_mut().roots = Some(roots);
    }
}
