use std::any::type_name;
use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::{Rc, Weak};

use slotmap::{SecondaryMap, SlotMap};

#[cfg(feature = "build")]
use super::compiled::CompiledFlow;
#[cfg(feature = "build")]
use super::deploy::{DeployFlow, DeployResult};
#[cfg(feature = "build")]
use super::deploy_provider::{ClusterSpec, Deploy, ExternalSpec, IntoProcessSpec};
#[cfg(feature = "build")]
use super::ir::HydroIrOpMetadata;
use super::ir::{HydroNode, HydroRoot};
use crate::location::{Cluster, External, LocationKey, LocationType, Process};

/// A compile-time directive to spawn a future on a location's `LocalSet`
/// alongside the DFIR scheduler.
pub enum Sidecar {
    /// A ready-to-go future expression (e.g. telemetry metrics collection).
    Simple {
        location_key: LocationKey,
        future_expr: Box<syn::Expr>,
    },
    /// A user-owned sidecar that returns a `(Stream, Sink)` pair to the framework.
    /// The closure is called at startup; the returned stream feeds items into the
    /// dataflow and the returned sink receives items from the dataflow.
    Bidi {
        location_key: LocationKey,
        sidecar_id: SidecarId,
        sidecar_closure: Box<syn::Expr>,
    },
}
#[cfg(feature = "sim")]
#[cfg(stageleft_runtime)]
use crate::sim::flow::SimFlow;
use crate::staging_util::Invariant;

#[stageleft::export(ExternalPortId, CycleId, ClockId, SidecarId, StmtId, HandoffId)]
crate::newtype_counter! {
    /// ID for an external output.
    pub struct ExternalPortId(usize);

    /// ID for a [`crate::location::Location::forward_ref`] cycle.
    pub struct CycleId(usize);

    /// ID for clocks (ticks).
    pub struct ClockId(usize);

    /// ID for user-owned sidecars.
    pub struct SidecarId(usize);

    /// ID for a statement in the emitted DFIR graph.
    pub struct StmtId(usize);

    /// ID for a handoff channel in the simulator.
    pub struct HandoffId(usize);
}

impl CycleId {
    #[cfg(feature = "build")]
    pub(crate) fn as_ident(&self) -> syn::Ident {
        syn::Ident::new(&format!("cycle_{}", self), proc_macro2::Span::call_site())
    }
}

impl SidecarId {
    /// Derives the two idents for a bidi sidecar: `(stream, sink)`.
    pub fn idents(&self) -> (syn::Ident, syn::Ident) {
        let span = proc_macro2::Span::call_site();
        (
            syn::Ident::new(&format!("__hydro_sidecar_{}_stream", self), span),
            syn::Ident::new(&format!("__hydro_sidecar_{}_sink", self), span),
        )
    }
}

pub(crate) type FlowState = Rc<RefCell<FlowStateInner>>;

pub(crate) struct FlowStateInner {
    /// Tracks the roots of the dataflow IR. This is referenced by
    /// `Stream` and `HfCycle` to build the IR. The inner option will
    /// be set to `None` when this builder is finalized.
    roots: Option<Vec<HydroRoot>>,

    /// Counter for generating unique external output identifiers.
    next_external_port: crate::Counter<ExternalPortId>,

    /// Counters for generating identifiers for cycles.
    next_cycle_id: crate::Counter<CycleId>,

    /// Counters for clock IDs.
    next_clock_id: crate::Counter<ClockId>,

    /// Counter for generating unique sidecar identifiers, not used for anything else.
    next_sidecar_id: crate::Counter<SidecarId>,

    /// Compile-time sidecar directives. Processed during compilation,
    /// not part of the dataflow IR.
    pub sidecars: Vec<Sidecar>,

    /// Weak references to the IR nodes of all live collections (streams, singletons, ...)
    /// created against this flow. When the flow is finalized, any collection that is still
    /// alive (its `Rc` has not been dropped) has its IR yanked and registered as a root,
    /// so that dataflow with side effects (e.g. `inspect`) is not silently lost just
    /// because the collection was dropped after finalization.
    pub(crate) live_collection_nodes: Vec<Weak<RefCell<HydroNode>>>,
}

impl FlowStateInner {
    pub fn next_external_port(&mut self) -> ExternalPortId {
        self.next_external_port.get_and_increment()
    }

    pub fn next_cycle_id(&mut self) -> CycleId {
        self.next_cycle_id.get_and_increment()
    }

    pub fn next_clock_id(&mut self) -> ClockId {
        self.next_clock_id.get_and_increment()
    }

    pub fn next_sidecar_id(&mut self) -> SidecarId {
        self.next_sidecar_id.get_and_increment()
    }

    pub fn push_root(&mut self, root: HydroRoot) {
        self.roots
            .as_mut()
            .expect("Attempted to add a root to a flow that has already been finalized. No roots can be added after the flow has been compiled.")
            .push(root);
    }

    pub fn try_push_root(&mut self, root: HydroRoot) {
        if let Some(roots) = self.roots.as_mut() {
            roots.push(root);
        }
    }
}

pub struct FlowBuilder<'a> {
    /// Hydro IR and associated counters
    flow_state: FlowState,

    /// Locations and their type.
    locations: SlotMap<LocationKey, LocationType>,
    /// Map from raw location ID to name (including externals).
    location_names: SecondaryMap<LocationKey, String>,
    /// The program version each location belongs to. Every location has an entry (0 unless it is a
    /// `next_version` successor); populated eagerly at location creation.
    #[cfg(feature = "sim")]
    location_version: SecondaryMap<LocationKey, u32>,
    /// Maps each location to the root key of its cross-version correspondence group: version 0 of
    /// the same logical location. Every location has an entry (its own key unless it is a
    /// `next_version` successor); populated eagerly at location creation.
    #[cfg(feature = "sim")]
    location_version_group_root: SecondaryMap<LocationKey, LocationKey>,

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
    /// Creates a new `FlowBuilder` to construct a Hydro program, using the Cargo package name as the program name.
    #[expect(
        clippy::new_without_default,
        reason = "call `new` explicitly, not `default`"
    )]
    pub fn new() -> Self {
        let mut name = std::env::var("CARGO_PKG_NAME").unwrap_or_else(|_| "unknown".to_owned());
        if let Ok(bin_path) = std::env::current_exe()
            && let Some(bin_name) = bin_path.file_stem()
        {
            name = format!("{}/{}", name, bin_name.display());
        }
        Self::with_name(name)
    }

    /// Creates a new `FlowBuilder` to construct a Hydro program, with the given program name.
    pub fn with_name(name: impl Into<String>) -> Self {
        Self {
            flow_state: Rc::new(RefCell::new(FlowStateInner {
                roots: Some(vec![]),
                next_external_port: crate::Counter::default(),
                next_cycle_id: crate::Counter::default(),
                next_clock_id: crate::Counter::default(),
                next_sidecar_id: crate::Counter::default(),
                sidecars: Vec::new(),
                live_collection_nodes: Vec::new(),
            })),
            locations: SlotMap::with_key(),
            location_names: SecondaryMap::new(),
            #[cfg(feature = "sim")]
            location_version: SecondaryMap::new(),
            #[cfg(feature = "sim")]
            location_version_group_root: SecondaryMap::new(),
            flow_name: name.into(),
            finalized: false,
            _phantom: PhantomData,
        }
    }

    pub(crate) fn flow_state(&self) -> &FlowState {
        &self.flow_state
    }

    fn insert_location(&mut self, ty: LocationType, name: String) -> LocationKey {
        let key = self.locations.insert(ty);
        self.location_names.insert(key, name);
        #[cfg(feature = "sim")]
        {
            self.location_version.insert(key, 0);
            self.location_version_group_root.insert(key, key);
        }
        key
    }

    pub fn process<P>(&mut self) -> Process<'a, P> {
        let key = self.insert_location(LocationType::Process, type_name::<P>().to_owned());
        Process {
            key,
            flow_state: self.flow_state().clone(),
            _phantom: PhantomData,
        }
    }

    pub fn cluster<C>(&mut self) -> Cluster<'a, C> {
        let key = self.insert_location(LocationType::Cluster, type_name::<C>().to_owned());
        Cluster {
            key,
            flow_state: self.flow_state().clone(),
            _phantom: PhantomData,
        }
    }

    pub fn external<E>(&mut self) -> External<'a, E> {
        let key = self.insert_location(LocationType::External, type_name::<E>().to_owned());
        External {
            key,
            flow_state: self.flow_state().clone(),
            _phantom: PhantomData,
        }
    }

    #[cfg(feature = "sim")]
    pub fn next_version<C>(&mut self, cluster: &Cluster<'a, C>) -> Cluster<'a, C> {
        let group_root = self.location_version_group_root[cluster.key];
        let version = self
            .location_version_group_root
            .values()
            .filter(|&&r| r == group_root)
            .count() as u32;
        let key = self.insert_location(LocationType::Cluster, type_name::<C>().to_owned());
        self.location_version.insert(key, version);
        self.location_version_group_root.insert(key, group_root);
        Cluster {
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

        let mut flow_state = self.flow_state.borrow_mut();

        // Yank the IR from any live collections (streams, singletons, ...) that are still
        // alive, since their `Drop` will run after finalization and would otherwise
        // silently fail to register their dataflow as a root.
        let live_collection_nodes = std::mem::take(&mut flow_state.live_collection_nodes);
        for node_cell in live_collection_nodes {
            if let Some(node_cell) = node_cell.upgrade() {
                let ir_node = node_cell.replace(HydroNode::Placeholder);
                if !matches!(ir_node, HydroNode::Placeholder) && !ir_node.is_shared_with_others() {
                    flow_state.push_root(HydroRoot::Null {
                        input: Box::new(ir_node),
                        op_metadata: HydroIrOpMetadata::new(),
                    });
                }
            }
        }

        let mut ir = flow_state.roots.take().unwrap();
        let sidecars = std::mem::take(&mut flow_state.sidecars);
        drop(flow_state);

        super::ir::unify_atomic_ticks(&mut ir);

        super::built::BuiltFlow {
            ir,
            locations: std::mem::take(&mut self.locations),
            location_names: std::mem::take(&mut self.location_names),
            sidecars,
            flow_name: std::mem::take(&mut self.flow_name),
            #[cfg(feature = "sim")]
            location_version: std::mem::take(&mut self.location_version),
            #[cfg(feature = "sim")]
            location_version_group_root: std::mem::take(&mut self.location_version_group_root),
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

    pub fn compile<D: Deploy<'a, InstantiateEnv = ()>>(self) -> CompiledFlow<'a> {
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
                next_external_port: crate::Counter::default(),
                next_cycle_id: crate::Counter::default(),
                next_clock_id: crate::Counter::default(),
                next_sidecar_id: crate::Counter::default(),
                sidecars: Vec::new(),
                live_collection_nodes: Vec::new(),
            })),
            locations: built.locations.clone(),
            location_names: built.location_names.clone(),
            #[cfg(feature = "sim")]
            location_version: built.location_version.clone(),
            #[cfg(feature = "sim")]
            location_version_group_root: built.location_version_group_root.clone(),
            flow_name: built.flow_name.clone(),
            finalized: false,
            _phantom: PhantomData,
        }
    }

    #[doc(hidden)] // TODO(mingwei): This is an unstable API for now
    pub fn replace_ir(&mut self, roots: Vec<HydroRoot>) {
        self.flow_state.borrow_mut().roots = Some(roots);
    }

    #[doc(hidden)] // TODO(mingwei): This is an unstable API for now
    pub fn next_clock_id(&mut self) -> ClockId {
        self.flow_state.borrow_mut().next_clock_id()
    }

    #[doc(hidden)] // TODO(mingwei): This is an unstable API for now
    pub fn next_cycle_id(&mut self) -> CycleId {
        self.flow_state.borrow_mut().next_cycle_id()
    }
}
