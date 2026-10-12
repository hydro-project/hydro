//! Entry points for a library that drives a simulation under a schedule of its own, through the
//! simulator's scripted hooks, without changing the program under test.
//!
//! The simulator's scripting facility ([`super::hooks`]) is written for a test author who binds
//! handles to the operators of a program they wrote. A tool that analyses programs it did not
//! write needs the same control, but by hook id rather than by typed handle, and before
//! compilation it needs to choose which operators are scripted at all. This module exposes
//! exactly that: the flow's IR and cluster sizing before compilation, a fresh output port and a
//! way to count what arrives at it, and the per-instance control a bound hook offers (pause,
//! hold, status, and installing a serialized decision). Everything here is a thin wrapper over
//! state the simulator already keeps; nothing changes how a simulation without a caller of this
//! module behaves.
//!
//! Decisions and statuses cross this boundary as `bincode` blobs of the decision and status
//! types in [`super::runtime`] (`BatchDecision`, `SnapshotDecision`, `BatchStatus`, and so on),
//! which is also how the typed handles in [`super::hooks`] carry them.

use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

use crate::compile::builder::ExternalPortId;
use crate::compile::ir::{DebugExpr, HydroNode, HydroRoot, transform_bottom_up};
use crate::live_collections::stream::{ExactlyOnce, TotalOrder};
use crate::location::LocationKey;
use crate::sim::compiled::script_ctx;
use crate::sim::flow::SimFlow;
use crate::sim::hooks::DecisionFuture;
use crate::sim::runtime::{HookLocationMeta, ScriptedHookControl};
use crate::sim::{SimClusterReceiver, SimReceiver};

impl<'a> SimFlow<'a> {
    /// The flow's IR, for a pass run before [`SimFlow::compiled`]. A pass may set
    /// `HydroIrOpMetadata::sim_hook_id` on a `use::batch` or `use::snapshot` operator to have the
    /// simulator bind a scripted hook to it (see [`hook_control`]), and may add roots.
    pub fn ir_mut(&mut self) -> &mut Vec<HydroRoot> {
        &mut self.ir
    }

    /// The number of members of each cluster, as set by [`SimFlow::with_cluster_size`].
    pub fn cluster_sizes(&self) -> Vec<(LocationKey, usize)> {
        #[expect(
            clippy::disallowed_methods,
            reason = "the result is sorted, so the map's iteration order is not observable"
        )]
        let mut sizes: Vec<(LocationKey, usize)> = self
            .cluster_max_sizes
            .iter()
            .map(|(key, size)| (key, *size))
            .collect();
        sizes.sort();
        sizes
    }

    /// `count` external port ids no operator in the flow uses yet, for a pass that adds
    /// `HydroRoot::SendExternal` or `HydroNode::ExternalInput` operators of its own. The ids are
    /// unused as of this call; a second call before the first batch is put into the IR returns
    /// the same ids.
    pub fn unused_external_ports(&mut self, count: usize) -> Vec<ExternalPortId> {
        let max_port: RefCell<Option<usize>> = RefCell::new(None);
        transform_bottom_up(
            &mut self.ir,
            &mut |root| {
                if let HydroRoot::SendExternal { to_port_id, .. } = root {
                    let mut m = max_port.borrow_mut();
                    *m = (*m).max(Some(to_port_id.into_inner()));
                }
            },
            &mut |node| {
                if let HydroNode::ExternalInput { from_port_id, .. } = node {
                    let mut m = max_port.borrow_mut();
                    *m = (*m).max(Some(from_port_id.into_inner()));
                }
            },
            false,
        );
        let first = max_port.into_inner().map_or(0, |m| m + 1);
        (first..first + count)
            .map(<ExternalPortId as crate::Countable>::from_count)
            .collect()
    }
}

/// The `serialize_fn` for a `HydroRoot::SendExternal` of `T` added by a pass: encodes each item
/// with the simulator's default codec, as the simulator does for the outputs a program declares
/// with `sim_output`.
pub fn bincode_serialize_fn<T: serde::Serialize + serde::de::DeserializeOwned>() -> DebugExpr {
    crate::sim::codec::staged_serialize::<T, crate::sim::codec::BincodeCodec>().into()
}

/// Takes every item already delivered to the external output `port` (from a process when
/// `member` is `None`, otherwise from that member of the sending cluster) and returns how many
/// there were. Does not run the scheduler, so it must be called inside a simulation thunk after
/// [`super::quiesce`], when everything the program has produced has reached its outputs. Items
/// are discarded; this is for callers that count rather than read.
pub fn drain_delivered(port: ExternalPortId, member: Option<u32>) -> usize {
    fn discard(_: &[u8]) {}
    if let Some(member) = member {
        let receiver: SimClusterReceiver<(), TotalOrder, ExactlyOnce> =
            SimClusterReceiver(port, PhantomData, discard);
        receiver.drain_delivered(member)
    } else {
        let receiver: SimReceiver<(), TotalOrder, ExactlyOnce> =
            SimReceiver(port, PhantomData, discard);
        receiver.drain_delivered()
    }
}

/// The control of one scripted hook instance, by id and member rather than by typed handle. See
/// [`hook_control`].
pub struct HookControl(Rc<RefCell<dyn ScriptedHookControl>>);

impl HookControl {
    /// Where the hook's operator is in the program's source.
    pub fn location(&self) -> HookLocationMeta {
        self.0.borrow().location_meta()
    }

    /// Whether the hook buffers on purpose while it has no decision, instead of being reported
    /// as forgotten by the scheduler's boundary scan. A hook under `auto_pause` returns to the
    /// paused state after every decision it consumes.
    pub fn set_auto_pause(&self, auto_pause: bool) {
        self.0.borrow_mut().set_auto_pause(auto_pause);
    }

    /// Sets the hold flag directly (the state `pause()` on a typed handle produces).
    pub fn set_hold(&self, hold: bool) {
        self.0.borrow_mut().set_hold(hold);
    }

    /// The hook's pending-input status, `bincode`-serialized: a `BatchStatus` for a batch of a
    /// stream or keyed stream, a `SnapshotStatus` for a snapshot of a singleton, a
    /// `KeyedSnapshotStatus` for a snapshot of a keyed singleton.
    pub fn status_blob(&self) -> Vec<u8> {
        self.0.borrow().status_blob()
    }
}

/// The control of the scripted hook with id `hook_id` on cluster member `member` (`None` for a
/// hook on a process). Must be called inside a simulation thunk after the instance is launched.
/// Panics if no such hook instance exists.
pub fn hook_control(hook_id: usize, member: Option<u32>) -> HookControl {
    HookControl(script_ctx().control(hook_id, member))
}

/// Installs a `bincode`-serialized decision for the scripted hook with id `hook_id` on cluster
/// member `member`, under the same group protocol as a typed handle's decision call: awaiting
/// the future suspends until the decision has taken its place in the schedule, and panics if it
/// never can. The blob must encode the decision type the hook expects (`BatchDecision<T>` for a
/// batch of a stream of `T`, and so on); a unit-payload variant such as `BatchDecision::<()>::All`
/// encodes identically for every `T`.
pub fn install_decision(hook_id: usize, member: Option<u32>, decision_blob: Vec<u8>) -> DecisionFuture {
    DecisionFuture::from_blob(hook_id, member, decision_blob)
}
