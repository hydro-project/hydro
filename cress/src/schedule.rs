//! Driving a simulation under a fixed schedule through the simulator's own scripted hooks.
//!
//! The amplification checker needs three things from the simulator: to see every batch and
//! snapshot decision point in a program it did not write, to delay one of them for as long as it
//! likes while every other point moves data forward promptly, and to count the records each
//! point admits and the messages each location sends. The simulator already has a scripting
//! facility for tests ([`hydro_lang::sim::hooks`]): a hook bound to an operator acts only when the
//! test installs a decision, can be paused so that it buffers on purpose, and reports how much
//! it has buffered. This module uses that facility from the checker's side, so the program under
//! test is not modified and the simulator is not extended.
//!
//! [`instrument`] is an IR pass run on the harness's [`SimFlow`] after the user's function has
//! built the program. It binds a scripted hook to every `use::batch` and `use::snapshot`
//! operator the simulator can script (which is every one except snapshots of top-level folds
//! over unordered exactly-once streams and snapshots of optionals, which keep their ordinary
//! hooks), and it tees the input of every network send into an extra simulation output whose
//! arrivals the host counts, per sending location and member. A flow that already carries
//! scripted hooks of its own is refused, because the checker must be the only party installing
//! decisions.
//!
//! The hooks the pass cannot bind, and the choice of which ready tick runs next, are answered by
//! [`hydro_lang::sim::prompt_schedule::PromptScheduleDriver`] under
//! [`hydro_lang::sim::compiled::CompiledSim::run_with_driver`]. That driver stands in for the
//! simulator's `deterministic()` mode, which refuses unscripted hooks with a real choice and more
//! than one runnable tick; once every hook kind is scriptable or `deterministic()` takes a default
//! policy, the checker should run under `deterministic()` and the driver should be deleted.
//!
//! [`Schedule`] is the host-side loop. At the start of a run it puts every bound hook into
//! `auto_pause`, so no hook moves data until told to. [`Schedule::settle`] then repeats, until
//! nothing is buffered anywhere: wait for the simulation to quiesce, find the first tick, in a
//! fixed order (processes before clusters, by location, then by member, then by tick), with
//! records or versions waiting at one of its hooks, and install one decision per hook of that
//! tick: "release everything" for a batch, "reveal the newest version" for a snapshot. The fixed
//! order matters for programs in which a clock element is a budget spent by whichever tick
//! execution admits it: a downstream location's tick then always sees the data an upstream tick
//! produced in the same round, which is also how the simulator's own scheduler orders ticks that
//! become ready together (by the location whose dataflow fed them).
//!
//! # What a hold means
//!
//! A held hook receives no decision, so its records wait, its tick parks when nothing else feeds
//! it, and if the tick runs for another hook's sake the held hook contributes nothing: a held
//! batch admits no records, and a held snapshot shows the version it showed last, so the tick
//! acts on stale state. Ending a hold is doing nothing: the next `settle` moves everything that
//! waited, in one decision. For a batch that releases the whole buffer. For a snapshot it reveals
//! the *newest* queued version and skips the rest, which is also what every snapshot decision
//! under this schedule does: the reference schedule is the one in which every snapshot sees the
//! newest state promptly, so a hold's effect is a delay of `k` rounds and not a lag of `k`
//! versions that persists for the rest of the run. (A fold emits its initial value and then one
//! version per element it absorbs, so several versions can queue between two tick executions
//! even with nothing held; revealing them one per tick would run the tick on each stale value.)
//!
//! A snapshot that has never revealed anything has nothing to show while held: the simulator
//! aborts if a tick runs with such a snapshot and no decision. So a held snapshot whose first
//! version is waiting when its tick is about to run for another hook's sake is revealed as the
//! ordinary schedule would reveal it, and the hold takes effect from the next version on. The
//! same applies to a snapshot that is not held but has no version at all when its tick runs; that
//! is a program whose singleton has no value yet, which the simulator does not allow, and the
//! loop stops with a message naming the hook rather than letting the simulator abort.

use std::cell::RefCell;
use std::collections::BTreeMap;

use hydro_lang::compile::builder::ExternalPortId;
use hydro_lang::compile::ir::{
    ClosureExpr, CollectionKind, DebugInstantiate, DebugType, HydroIrMetadata, HydroIrOpMetadata,
    HydroNode, HydroRoot, SharedNode, StreamOrder, StreamRetry, transform_bottom_up,
};
use hydro_lang::location::LocationKey;
use hydro_lang::location::dynamic::LocationId;
use hydro_lang::sim::extension::{
    bincode_serialize_fn, drain_delivered, hook_control, install_decision,
};
use hydro_lang::sim::flow::SimFlow;
use hydro_lang::sim::quiesce;
use hydro_lang::sim::runtime::{
    BatchDecision, BatchStatus, KeyedBatchDecision, KeyedSnapshotDecision, KeyedSnapshotStatus,
    SnapshotDecision, SnapshotStatus, UnorderedBatchDecision, UnorderedKeyedBatchDecision,
};

/// What kind of decision point a bound hook is, which fixes the decisions it accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookKind {
    /// `use::batch` of a totally ordered stream.
    Batch,
    /// `use::batch` of an unordered stream.
    UnorderedBatch,
    /// `use::batch` of a keyed stream with ordered values.
    KeyedBatch,
    /// `use::batch` of a keyed stream with unordered values.
    UnorderedKeyedBatch,
    /// `use::snapshot` of a singleton.
    Snapshot,
    /// `use::snapshot` of a keyed singleton.
    KeyedSnapshot,
}

impl HookKind {
    /// Whether this hook admits stream records (counted as work) rather than versions.
    pub fn is_batch(&self) -> bool {
        !matches!(self, HookKind::Snapshot | HookKind::KeyedSnapshot)
    }

    /// The bincode-encoded decision the schedule installs: release the whole buffer, or reveal
    /// the newest version (see the [module docs](self)). Unit-payload variants encode identically
    /// for every item type, so the checker need not know the program's types.
    fn advance_blob(&self) -> Vec<u8> {
        match self {
            HookKind::Batch => bincode::serialize(&BatchDecision::<()>::All),
            HookKind::UnorderedBatch => bincode::serialize(&UnorderedBatchDecision::<()>::All),
            HookKind::KeyedBatch => bincode::serialize(&KeyedBatchDecision::<(), ()>::All),
            HookKind::UnorderedKeyedBatch => {
                bincode::serialize(&UnorderedKeyedBatchDecision::<(), ()>::All)
            }
            HookKind::Snapshot => bincode::serialize(&SnapshotDecision::<()>::RevealLatest),
            HookKind::KeyedSnapshot => {
                bincode::serialize(&KeyedSnapshotDecision::<(), ()>::RevealLatest)
            }
        }
        .unwrap()
    }
}

/// One decision point the pass bound, as the host sees it.
#[derive(Debug, Clone)]
pub struct BoundHook {
    /// The scripted hook id, shared by every member's instance on a cluster.
    pub id: usize,
    /// What kind of decision point this is.
    pub kind: HookKind,
    /// The tick the hook feeds.
    pub tick: LocationId,
    /// The process or cluster the tick belongs to.
    pub root: LocationId,
    /// The Rust type of the items batched or the singleton snapshotted, as written in the IR.
    pub item_type: String,
}

/// One network send the pass is counting.
#[derive(Debug, Clone)]
pub struct CountedSend {
    /// The sending process or cluster.
    pub from: LocationId,
    /// The simulation output port the tee feeds.
    pub port: ExternalPortId,
}

/// A decision point the simulator cannot script, and which therefore cannot be held. It keeps
/// its ordinary hook, answered by the prompt-schedule driver.
#[derive(Debug, Clone)]
pub struct NotScriptable {
    /// `file:line` of the `use::snapshot`, when the IR recorded it.
    pub location: String,
    /// Why: `"snapshot of an optional"` or `"snapshot of a fold over an unordered exactly-once
    /// stream"`.
    pub reason: &'static str,
}

/// What [`instrument`] did to a flow.
#[derive(Debug, Clone, Default)]
pub struct Instrumented {
    /// Every decision point bound to a scripted hook.
    pub hooks: Vec<BoundHook>,
    /// Every network send being counted.
    pub sends: Vec<CountedSend>,
    /// Number of members of each cluster, from the flow's sizing.
    pub cluster_sizes: BTreeMap<LocationId, usize>,
    /// Decision points left unbound because the simulator cannot script them.
    pub not_scriptable: Vec<NotScriptable>,
}

/// `file:line` of an operator from the position its macro recorded, or from its backtrace.
fn ir_location(op: &HydroIrOpMetadata) -> String {
    if let Some(u) = op.backtrace.user_location()
        && let Some(file) = u.file
    {
        return format!("{}:{}", relative_path(file), u.line);
    }
    op.backtrace
        .elements()
        .next()
        .and_then(|e| Some(format!("{}:{}", relative_path(e.filename.as_deref()?), e.lineno?)))
        .unwrap_or_else(|| "unknown location".to_owned())
}

/// A path relative to the nearest ancestor of the working directory that contains it.
fn relative_path(file: &str) -> String {
    let path = std::path::Path::new(file);
    std::env::current_dir()
        .ok()
        .and_then(|cwd| {
            cwd.ancestors()
                .filter(|base| base.parent().is_some())
                .find_map(|base| path.strip_prefix(base).ok())
                .map(|p| p.display().to_string())
        })
        .unwrap_or_else(|| file.to_owned())
}

/// Whether a singleton's snapshot takes the simulator's fuzz-passthrough shortcut and so cannot
/// be scripted. This mirrors the two conditions the simulator's code generation applies, which
/// must stay in step with it: the singleton is a `Fold` at a top-level location over an
/// unbounded, unordered, exactly-once stream (`emit_fold_hook` in `sim/builder.rs` then hooks
/// the fold's input, and `HydroNode::Fold` in `compile/ir/mod.rs` marks the fold's output as
/// hooked when the fold itself is not scripted, which [`instrument`] guarantees by refusing
/// flows with scripted hooks); and the snapshot reads that output, possibly through tees (which
/// propagate the mark). A keyed singleton never takes the shortcut: its snapshot is scriptable
/// whatever its fold's input order.
fn is_passthrough_singleton(node: &HydroNode) -> bool {
    match node {
        HydroNode::Tee { inner, .. } => is_passthrough_singleton(&inner.0.borrow()),
        HydroNode::Fold {
            input, metadata, ..
        } => {
            metadata.location_id.is_root()
                && !metadata.collection_kind.is_bounded()
                && matches!(
                    input.metadata().collection_kind,
                    CollectionKind::Stream {
                        order: StreamOrder::NoOrder,
                        retry: StreamRetry::ExactlyOnce,
                        ..
                    }
                )
        }
        _ => false,
    }
}

fn type_string(ty: &DebugType) -> String {
    quote::ToTokens::to_token_stream(&*ty.0)
        .to_string()
        .replace(" :: ", "::")
        .replace(" < ", "<")
        .replace(" > ", ">")
        .replace(" >", ">")
        .replace("< ", "<")
        .replace(" ,", ",")
}

/// Binds a scripted hook to every batch and snapshot the simulator can script, and tees every
/// network send into a counting output. See the [module docs](self).
pub fn instrument(flow: &mut SimFlow<'_>) -> Instrumented {
    // The checker must be the only party installing decisions: a scripted hook the program
    // bound itself would never receive one, and the simulator would report it as forgotten (or,
    // if it never got input, the run would complete as if the hook did not exist). Refuse such
    // flows up front, naming the hooks. Port ids are allocated above anything the program used.
    let scripted: RefCell<Vec<String>> = RefCell::new(Vec::new());
    let sends_to_count = RefCell::new(0usize);
    transform_bottom_up(
        flow.ir_mut(),
        &mut |root| {
            if root.op_metadata().sim_hook_id.is_some() {
                scripted.borrow_mut().push(ir_location(root.op_metadata()));
            }
        },
        &mut |node| {
            if node.metadata().op.sim_hook_id.is_some() {
                scripted.borrow_mut().push(ir_location(&node.metadata().op));
            }
            if matches!(node, HydroNode::Network { .. }) {
                *sends_to_count.borrow_mut() += 1;
            }
        },
        false,
    );
    let scripted = scripted.into_inner();
    assert!(
        scripted.is_empty(),
        "the amplification checker binds every decision point itself; the flow already has {} scripted hook(s) at {}; remove the sim_hook bindings from the function under check",
        scripted.len(),
        scripted.join(", ")
    );
    let next_hook_id = RefCell::new(0usize);
    let ports = RefCell::new(
        flow.unused_external_ports(sends_to_count.into_inner())
            .into_iter(),
    );

    let hooks = RefCell::new(Vec::new());
    let sends = RefCell::new(Vec::new());
    let new_roots = RefCell::new(Vec::new());
    let not_scriptable = RefCell::new(Vec::new());

    transform_bottom_up(
        flow.ir_mut(),
        &mut |_| {},
        &mut |node| match node {
            // A `use::batch` or `use::snapshot` at a top-level location, or the `.atomic()`
            // entry into an atomic context, which the simulator compiles through the same
            // `batch` path and hooks the same way. A `use::atomic` read *inside* an atomic
            // context (its input location is `Atomic`) is a pass-through; the simulator refuses
            // a hook there because the decision was made at the entry.
            HydroNode::Batch { inner, metadata } | HydroNode::BeginAtomic { inner, metadata } => {
                let in_location = &inner.metadata().location_id;
                if !in_location.is_top_level() || matches!(in_location, LocationId::Atomic(_)) {
                    return;
                }
                // The hook belongs to the tick the atomic context runs in.
                let tick = match &metadata.location_id {
                    LocationId::Atomic(tick) => tick.as_ref().clone(),
                    other => other.clone(),
                };
                let (kind, item_type) = match &inner.metadata().collection_kind {
                    CollectionKind::Stream {
                        order,
                        element_type,
                        ..
                    } => (
                        if matches!(order, StreamOrder::NoOrder) {
                            HookKind::UnorderedBatch
                        } else {
                            HookKind::Batch
                        },
                        type_string(element_type),
                    ),
                    CollectionKind::KeyedStream {
                        value_order,
                        key_type,
                        value_type,
                        ..
                    } => (
                        if matches!(value_order, StreamOrder::NoOrder) {
                            HookKind::UnorderedKeyedBatch
                        } else {
                            HookKind::KeyedBatch
                        },
                        format!("({}, {})", type_string(key_type), type_string(value_type)),
                    ),
                    CollectionKind::Singleton { element_type, .. } => {
                        if is_passthrough_singleton(inner) {
                            not_scriptable.borrow_mut().push(NotScriptable {
                                location: ir_location(&metadata.op),
                                reason: "snapshot of a fold over an unordered exactly-once stream",
                            });
                            return;
                        }
                        (HookKind::Snapshot, type_string(element_type))
                    }
                    CollectionKind::KeyedSingleton {
                        key_type,
                        value_type,
                        ..
                    } => (
                        HookKind::KeyedSnapshot,
                        format!("({}, {})", type_string(key_type), type_string(value_type)),
                    ),
                    CollectionKind::Optional { .. } => {
                        not_scriptable.borrow_mut().push(NotScriptable {
                            location: ir_location(&metadata.op),
                            reason: "snapshot of an optional",
                        });
                        return;
                    }
                };
                let id = {
                    let mut n = next_hook_id.borrow_mut();
                    let id = *n;
                    *n += 1;
                    id
                };
                metadata.op.sim_hook_id = Some(id);
                hooks.borrow_mut().push(BoundHook {
                    id,
                    kind,
                    tick,
                    root: in_location.clone(),
                    item_type,
                });
            }
            HydroNode::Network { input, .. } => {
                let from = input.metadata().location_id.root().clone();
                let input_meta = input.metadata().clone();
                // A send's input is a stream (one message per record) or, for a `demux` to a
                // cluster, a keyed stream whose key is the destination member (again one
                // message per record). Anything else is refused rather than left uncounted: a
                // send the checker cannot count would make a benign verdict unreliable.
                let (bound, order, retry, pairs) = match &input_meta.collection_kind {
                    CollectionKind::Stream {
                        bound,
                        order,
                        retry,
                        ..
                    } => (bound.clone(), order.clone(), retry.clone(), None),
                    CollectionKind::KeyedStream {
                        bound,
                        value_retry,
                        key_type,
                        value_type,
                        ..
                    } => (
                        bound.clone(),
                        StreamOrder::NoOrder,
                        value_retry.clone(),
                        Some((key_type.clone(), value_type.clone())),
                    ),
                    other => panic!(
                        "the amplification checker cannot count a network send whose input is a {other:?}; only streams and keyed streams are supported"
                    ),
                };
                // Replace the send's input with one branch of a tee, and count the other.
                let original = std::mem::replace(&mut **input, HydroNode::Placeholder);
                let shared = SharedNode(std::rc::Rc::new(RefCell::new(original)));
                let tee_meta = HydroIrMetadata {
                    location_id: input_meta.location_id.clone(),
                    collection_kind: input_meta.collection_kind.clone(),
                    consistency: input_meta.consistency.clone(),
                    cardinality: None,
                    tag: None,
                    op: HydroIrOpMetadata::new(),
                };
                **input = HydroNode::Tee {
                    inner: SharedNode(shared.0.clone()),
                    metadata: tee_meta.clone(),
                };
                let mut branch = HydroNode::Tee {
                    inner: shared,
                    metadata: tee_meta,
                };
                // The counting branch is a stream of `()`; a keyed stream is first flattened
                // into a stream of its entries, which is what `KeyedStream::entries` emits.
                if let Some((key_type, value_type)) = pairs {
                    let (key, value) = (*key_type.0, *value_type.0);
                    let entry_type: syn::Type = syn::parse_quote!((#key, #value));
                    branch = HydroNode::Cast {
                        inner: Box::new(branch),
                        metadata: HydroIrMetadata {
                            location_id: input_meta.location_id.clone(),
                            collection_kind: CollectionKind::Stream {
                                bound: bound.clone(),
                                order: order.clone(),
                                retry: retry.clone(),
                                element_type: DebugType::from(entry_type),
                            },
                            consistency: input_meta.consistency.clone(),
                            cardinality: None,
                            tag: None,
                            op: HydroIrOpMetadata::new(),
                        },
                    };
                }
                let count_closure: syn::Expr = syn::parse_quote!(|_| ());
                let unit_type: syn::Type = syn::parse_quote!(());
                let counted = HydroNode::Map {
                    f: ClosureExpr::from(count_closure),
                    input: Box::new(branch),
                    metadata: HydroIrMetadata {
                        location_id: input_meta.location_id.clone(),
                        collection_kind: CollectionKind::Stream {
                            bound,
                            order,
                            retry,
                            element_type: DebugType::from(unit_type),
                        },
                        consistency: input_meta.consistency,
                        cardinality: None,
                        tag: None,
                        op: HydroIrOpMetadata::new(),
                    },
                };
                let port = ports
                    .borrow_mut()
                    .next()
                    .expect("one unused port was reserved per network send");
                new_roots.borrow_mut().push(HydroRoot::SendExternal {
                    to_external_key: LocationKey::FIRST,
                    to_port_id: port,
                    to_many: false,
                    unpaired: true,
                    serialize_fn: Some(bincode_serialize_fn::<()>()),
                    instantiate_fn: DebugInstantiate::Building,
                    input: Box::new(counted),
                    op_metadata: HydroIrOpMetadata::new(),
                });
                sends.borrow_mut().push(CountedSend { from, port });
            }
            _ => {}
        },
        false,
    );
    flow.ir_mut().extend(new_roots.into_inner());

    let cluster_sizes: BTreeMap<LocationId, usize> = flow
        .cluster_sizes()
        .into_iter()
        .map(|(key, size)| (LocationId::Cluster(key), size))
        .collect();

    Instrumented {
        hooks: hooks.into_inner(),
        sends: sends.into_inner(),
        cluster_sizes,
        not_scriptable: not_scriptable.into_inner(),
    }
}

impl Instrumented {
    /// The member ids a hook or send at `root` has an instance on: `[None]` for a process.
    pub fn members(&self, root: &LocationId) -> Vec<Option<u32>> {
        match root {
            LocationId::Cluster(_) => {
                let n = *self
                    .cluster_sizes
                    .get(root)
                    .unwrap_or_else(|| panic!("no size for cluster {root:?}"));
                (0..n as u32).map(Some).collect()
            }
            _ => vec![None],
        }
    }
}

/// One instance of a bound hook: the hook on one member (or on a process).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HookInstance {
    /// The scripted hook id.
    pub id: usize,
    /// The cluster member, or `None` on a process.
    pub member: Option<u32>,
}

/// The counts one run accumulates. See [`Schedule`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Counts {
    /// Records admitted by each batch hook instance.
    pub admitted: BTreeMap<HookInstance, u64>,
    /// Network messages sent by each `(location, member)`.
    pub sent: BTreeMap<(LocationId, Option<u32>), u64>,
}

/// One tick's hooks (on one member), in the order the pass found them.
#[derive(Debug, Clone)]
struct TickHooks {
    hooks: Vec<(HookInstance, HookKind)>,
}

/// The most iterations one [`Schedule::settle`] may take before it gives up. Each iteration runs
/// one tick, so a program whose tick re-feeds its own batch on every execution with nothing to
/// stop it would otherwise spin here silently; this turns that into a failure naming the tick.
const SETTLE_ITERATION_CAP: u64 = 1_000_000;

/// The host-side scheduling loop for one run. Create it inside the simulation thunk.
pub struct Schedule<'i> {
    instrumented: &'i Instrumented,
    ticks: Vec<TickHooks>,
    /// A hold on every member's instance of a hook at once (the checker's unit of delay).
    held_id: Option<usize>,
    /// Snapshot instances that have revealed at least one version. A held snapshot not in this
    /// set has nothing to pin, so it is revealed as the ordinary schedule would (see the
    /// [module docs](self)).
    revealed: std::collections::BTreeSet<HookInstance>,
    /// The counts so far.
    pub counts: Counts,
    /// `file:line` of each bound hook as the simulator reports it, by hook id.
    pub locations: BTreeMap<usize, String>,
    /// Hook ids that had something waiting at some point in this run.
    pub seen: std::collections::BTreeSet<usize>,
    /// How many times a tick containing the held hook ran, for another hook's sake, while the
    /// held hook had something waiting. Zero means the hold took effect purely by parking.
    pub holds_applied: u64,
}

impl<'i> Schedule<'i> {
    /// Puts every bound hook into `auto_pause` and prepares the per-tick groups. Must be called
    /// inside the simulation thunk, after the instance is launched.
    pub fn new(instrumented: &'i Instrumented) -> Self {
        let mut by_tick: BTreeMap<(LocationId, Option<u32>, LocationId), TickHooks> =
            BTreeMap::new();
        let mut locations = BTreeMap::new();
        for hook in &instrumented.hooks {
            for member in instrumented.members(&hook.root) {
                let control = hook_control(hook.id, member);
                control.set_auto_pause(true);
                control.set_hold(true);
                locations
                    .entry(hook.id)
                    .or_insert_with(|| line_of(control.location().location).to_owned());
                by_tick
                    .entry((hook.root.clone(), member, hook.tick.clone()))
                    .or_insert_with(|| TickHooks { hooks: Vec::new() })
                    .hooks
                    .push((
                        HookInstance {
                            id: hook.id,
                            member,
                        },
                        hook.kind,
                    ));
            }
        }
        Schedule {
            instrumented,
            ticks: by_tick.into_values().collect(),
            held_id: None,
            revealed: Default::default(),
            counts: Counts::default(),
            locations,
            seen: Default::default(),
            holds_applied: 0,
        }
    }

    /// Starts holding every instance of hook `id`: it receives no decisions until
    /// [`Schedule::release`].
    pub fn hold(&mut self, id: usize) {
        self.held_id = Some(id);
    }

    /// Ends the hold; the next [`Schedule::settle`] moves everything that waited, in one decision
    /// per hook instance.
    pub fn release(&mut self) {
        self.held_id = None;
    }

    /// The identity the checker reports a hook under: `file:line#index [item]` for a batch,
    /// `file:line#index [snapshot item]` for a snapshot, where `index` is the hook's position
    /// within its tick (shown by reports only when two points share a line).
    pub fn identity(&self, id: usize) -> String {
        let hook = self
            .instrumented
            .hooks
            .iter()
            .find(|h| h.id == id)
            .expect("unknown hook id");
        let index = self
            .instrumented
            .hooks
            .iter()
            .filter(|h| h.tick == hook.tick)
            .position(|h| h.id == id)
            .unwrap_or(0);
        let location = self.locations.get(&id).cloned().unwrap_or_default();
        let item = strip_paths(&hook.item_type);
        if hook.kind.is_batch() {
            format!("{location}#{index} [{item}]")
        } else {
            format!("{location}#{index} [snapshot {item}]")
        }
    }

    fn is_held(&self, hook: &HookInstance) -> bool {
        self.held_id == Some(hook.id)
    }

    fn waiting(kind: HookKind, blob: &[u8]) -> usize {
        match kind {
            HookKind::Snapshot => {
                bincode::deserialize::<SnapshotStatus>(blob)
                    .unwrap()
                    .newer_versions
            }
            HookKind::KeyedSnapshot => {
                bincode::deserialize::<KeyedSnapshotStatus>(blob)
                    .unwrap()
                    .newer_versions
            }
            _ => bincode::deserialize::<BatchStatus>(blob).unwrap().buffered,
        }
    }

    /// Runs the simulation until nothing is waiting at any hook that is not held: see the
    /// [module docs](self). Also drains the send counters.
    pub async fn settle(&mut self) {
        let mut iterations = 0u64;
        loop {
            quiesce().await;
            let mut ran = false;
            for tick in &self.ticks {
                // What every hook of this tick has waiting, read before any decision is made.
                let status: Vec<(HookInstance, HookKind, usize)> = tick
                    .hooks
                    .iter()
                    .map(|(hook, kind)| {
                        let blob = hook_control(hook.id, hook.member).status_blob();
                        (hook.clone(), *kind, Self::waiting(*kind, &blob))
                    })
                    .collect();
                for (hook, _, waiting) in &status {
                    if *waiting > 0 {
                        self.seen.insert(hook.id);
                    }
                }
                // The tick runs if some hook that is not held has something waiting.
                let runs = status
                    .iter()
                    .any(|(hook, _, waiting)| *waiting > 0 && !self.is_held(hook));
                if !runs {
                    continue;
                }
                let mut decisions: Vec<(HookInstance, HookKind, usize)> = Vec::new();
                let mut held_waiting = false;
                for (hook, kind, waiting) in status {
                    let is_snapshot = !kind.is_batch();
                    let never_revealed = is_snapshot && !self.revealed.contains(&hook);
                    if waiting == 0 {
                        assert!(
                            !(kind == HookKind::Snapshot && never_revealed),
                            "the tick containing the snapshot at {} is about to run, but that snapshot has no version yet (a singleton must have a value before its tick runs)",
                            self.identity(hook.id)
                        );
                        continue;
                    }
                    if self.is_held(&hook) && !never_revealed {
                        held_waiting = true;
                        continue;
                    }
                    decisions.push((hook, kind, waiting));
                }
                if held_waiting {
                    self.holds_applied += 1;
                }
                for (hook, kind, waiting) in decisions {
                    if kind.is_batch() {
                        *self.counts.admitted.entry(hook.clone()).or_default() += waiting as u64;
                    } else {
                        self.revealed.insert(hook.clone());
                    }
                    install_decision(hook.id, hook.member, kind.advance_blob()).await;
                }
                ran = true;
                iterations += 1;
                if iterations > SETTLE_ITERATION_CAP {
                    let (hook, _) = &tick.hooks[0];
                    panic!(
                        "the schedule has run {SETTLE_ITERATION_CAP} ticks within one round without the simulation quiescing; the tick containing {}{} keeps finding input (a self-feeding loop with nothing to stop it)",
                        self.identity(hook.id),
                        hook.member.map(|m| format!(" on member {m}")).unwrap_or_default()
                    );
                }
                break;
            }
            if !ran {
                break;
            }
        }
        self.drain_sends();
    }

    /// Counts the messages sent since the last drain. Called after the simulation has quiesced,
    /// so every message a round produced has already reached its counting output; the drain
    /// therefore never needs to run the scheduler.
    fn drain_sends(&mut self) {
        for send in &self.instrumented.sends {
            for member in self.instrumented.members(&send.from) {
                let n = drain_delivered(send.port, member);
                if n > 0 {
                    *self
                        .counts
                        .sent
                        .entry((send.from.clone(), member))
                        .or_default() += n as u64;
                }
            }
        }
    }
}

/// Reduces a `file:line:col` location to `file:line`. A location without a trailing numeric
/// column is returned unchanged.
fn line_of(location: &str) -> &str {
    match location.rsplit_once(':') {
        Some((head, col))
            if !col.is_empty()
                && col.bytes().all(|b| b.is_ascii_digit())
                && head.rsplit_once(':').is_some_and(|(_, line)| {
                    !line.is_empty() && line.bytes().all(|b| b.is_ascii_digit())
                }) =>
        {
            head
        }
        _ => location,
    }
}

/// Removes `path::` prefixes from a type name, so `(u64, a::b::Op)` reads `(u64, Op)`.
fn strip_paths(type_name: &str) -> String {
    let mut out = String::with_capacity(type_name.len());
    let mut ident = String::new();
    let mut rest = type_name;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix("::") {
            ident.clear();
            rest = after;
            continue;
        }
        let c = rest.chars().next().unwrap();
        rest = &rest[c.len_utf8()..];
        if c.is_alphanumeric() || c == '_' {
            ident.push(c);
        } else {
            out.push_str(&ident);
            ident.clear();
            out.push(c);
        }
    }
    out.push_str(&ident);
    out
}
