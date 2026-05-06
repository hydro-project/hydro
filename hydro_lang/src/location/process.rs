//! Definition of the [`Process`] location type, representing a single-node
//! compute location in a distributed Hydro program.
//!
//! A [`Process`] is the simplest kind of location: it corresponds to exactly one
//! machine (or OS process) and all live collections placed on it are materialized
//! on that single node. Use a process when the computation does not need to be
//! replicated or partitioned across multiple nodes.
//!
//! Processes are created via [`FlowBuilder::process`](crate::compile::builder::FlowBuilder::process)
//! and are parameterized by a **tag type** (`ProcessTag`) that lets the type
//! system distinguish different processes at compile time.

use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

use super::{Location, LocationId};
use crate::compile::builder::FlowState;
use crate::location::{LocationKey, TopLevel};
use crate::staging_util::Invariant;

/// A single-node location in a distributed Hydro program.
///
/// `Process` represents exactly one machine (or OS process) and is one of the
/// core location types that implements the [`Location`] trait. Live collections
/// placed on a `Process` are materialized entirely on that single node.
///
/// The type parameter `ProcessTag` is a compile-time marker that differentiates
/// distinct processes in the same dataflow graph (e.g. `Process<'a, Leader>` vs
/// `Process<'a, Follower>`). It defaults to `()` when only one process is
/// needed.
///
/// # Creating a Process
/// ```rust
/// # use hydro_lang::prelude::*;
/// struct MyTag;
/// let mut flow = FlowBuilder::new();
/// let node = flow.process::<MyTag>();
/// # let _ = &node;
/// # let _ = flow.finalize();
/// ```
pub struct Process<'a, ProcessTag = ()> {
    pub(crate) key: LocationKey,
    pub(crate) flow_state: FlowState,
    pub(crate) _phantom: Invariant<'a, ProcessTag>,
}

impl<P> Debug for Process<'_, P> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Process({})", self.key)
    }
}

impl<P> Eq for Process<'_, P> {}
impl<P> PartialEq for Process<'_, P> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key && FlowState::ptr_eq(&self.flow_state, &other.flow_state)
    }
}

impl<P> Clone for Process<'_, P> {
    fn clone(&self) -> Self {
        Process {
            key: self.key,
            flow_state: self.flow_state.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<'a, P> super::dynamic::DynLocation for Process<'a, P> {
    fn dyn_id(&self) -> LocationId {
        LocationId::Process(self.key)
    }

    fn flow_state(&self) -> &FlowState {
        &self.flow_state
    }

    fn is_top_level() -> bool {
        true
    }

    fn multiversioned(&self) -> bool {
        false // processes are always single-versioned
    }

    fn cluster_consistency() -> Option<super::dynamic::ClusterConsistency> {
        None
    }
}

impl<'a, P> Location<'a> for Process<'a, P> {
    type Root = Self;

    type DropConsistency = Self;

    fn consistency() -> Option<super::dynamic::ClusterConsistency> {
        None
    }

    fn root(&self) -> Self::Root {
        self.clone()
    }

    fn drop_consistency(&self) -> Self::DropConsistency {
        self.clone()
    }

    fn from_drop_consistency(l2: Self::DropConsistency) -> Self {
        l2
    }
}

impl<'a, P> TopLevel<'a> for Process<'a, P> {}

#[cfg(feature = "sim")]
impl<'a, P> Process<'a, P> {
    /// Sets up a simulated input port on this location for testing.
    ///
    /// Returns a handle to send messages to the location as well as a stream
    /// of received messages. This is only available when the `sim` feature is enabled.
    pub fn sim_input<
        T,
        O: crate::live_collections::stream::Ordering,
        R: crate::live_collections::stream::Retries,
    >(
        &self,
    ) -> (
        crate::sim::SimSender<T, O, R>,
        crate::live_collections::stream::Stream<
            T,
            Self,
            crate::live_collections::boundedness::Unbounded,
            O,
            R,
        >,
    )
    where
        T: serde::Serialize + serde::de::DeserializeOwned,
    {
        use crate::location::dynamic::DynLocation;

        let external_location: super::External<'a, ()> = super::External {
            key: LocationKey::FIRST,
            flow_state: self.flow_state().clone(),
            _phantom: PhantomData,
        };

        let (external, stream) = self.source_external_bincode(&external_location);

        (crate::sim::SimSender(external.port_id, PhantomData), stream)
    }

    /// Sets up a simulated atomic input port on this process for testing.
    ///
    /// Unlike [`Self::sim_input`], this returns a [`super::Atomic`] stream and a
    /// [`crate::sim::SimAtomicSender`] that synchronously sends data to that stream. The sender
    /// guarantees that after a value is sent, it will be immediately read by any downstream
    /// consumers without any buffering.
    pub fn sim_atomic_input<
        T,
        O: crate::live_collections::stream::Ordering,
        R: crate::live_collections::stream::Retries,
    >(
        &self,
    ) -> (
        crate::sim::SimAtomicSender<T, O, R>,
        crate::live_collections::stream::Stream<
            T,
            super::Atomic<Self>,
            crate::live_collections::boundedness::Unbounded,
            O,
            R,
        >,
    )
    where
        T: 'a + serde::Serialize + serde::de::DeserializeOwned,
    {
        use std::marker::PhantomData;

        use stageleft::quote_type;
        use tokio_util::codec::LengthDelimitedCodec;

        use crate::compile::ir::{DebugInstantiate, HydroNode, HydroRoot};
        use crate::live_collections::boundedness::Unbounded;
        use crate::live_collections::stream::Stream;
        use crate::location::dynamic::DynLocation;
        use crate::location::tick::Atomic;
        use crate::location::{Location, LocationKey, NetworkHint, Tick};
        use crate::staging_util::get_this_crate;

        let id = self.flow_state().borrow_mut().next_clock_id();
        let atomic_location = Atomic {
            tick: Tick {
                id,
                l: self.clone(),
            },
        };

        let external_location: super::External<'a, ()> = super::External {
            key: LocationKey::FIRST,
            flow_state: self.flow_state().clone(),
            _phantom: PhantomData,
        };

        let next_external_port_id = self.flow_state().borrow_mut().next_external_port();

        let root = get_this_crate();
        let in_t_type = quote_type::<T>();

        let deser_fn: syn::Expr = syn::parse_quote! {
            |res| {
                let b = res.unwrap();
                #root::runtime_support::bincode::deserialize::<#in_t_type>(&b).unwrap()
            }
        };

        // Create the source stream at the Atomic location directly
        let stream: Stream<T, Atomic<Self>, Unbounded, O, R> = Stream::new(
            atomic_location.clone(),
            HydroNode::ExternalInput {
                from_external_key: external_location.key,
                from_port_id: next_external_port_id,
                from_many: false,
                codec_type: quote_type::<LengthDelimitedCodec>().into(),
                port_hint: NetworkHint::Auto,
                instantiate_fn: DebugInstantiate::Building,
                deserialize_fn: Some(deser_fn.into()),
                metadata: atomic_location.new_node_metadata(Stream::<
                    T,
                    Atomic<Self>,
                    Unbounded,
                    O,
                    R,
                >::collection_kind()),
            },
        );

        // Wire up a dummy send side (empty stream) so the external port is paired
        let empty_stream: Stream<T, Self, _, _, _> = self.source_iter(stageleft::q!([]));
        let out_t_type = quote_type::<T>();
        let ser_fn: syn::Expr = syn::parse_quote! {
            #root::runtime_support::stageleft::runtime_support::fn1_type_hint::<#out_t_type, _>(
                |b| #root::runtime_support::bincode::serialize(&b).unwrap().into()
            )
        };
        self.flow_state()
            .borrow_mut()
            .push_root(HydroRoot::SendExternal {
                to_external_key: external_location.key,
                to_port_id: next_external_port_id,
                to_many: false,
                unpaired: false,
                serialize_fn: Some(ser_fn.into()),
                instantiate_fn: DebugInstantiate::Building,
                input: Box::new(empty_stream.ir_node.replace(HydroNode::Placeholder)),
                op_metadata: crate::compile::ir::HydroIrOpMetadata::new(),
            });

        (
            crate::sim::SimAtomicSender(crate::sim::SimSender(next_external_port_id, PhantomData)),
            stream,
        )
    }
}
