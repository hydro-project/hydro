use std::fmt::Debug;
use std::marker::PhantomData;
use std::time::Duration;

use futures::stream::Stream as FuturesStream;
use proc_macro2::Span;
use serde::{Deserialize, Serialize};
use stageleft::{QuotedWithContext, q};

use super::builder::FlowState;
use crate::backtrace::get_backtrace;
use crate::cycle::{CycleCollection, ForwardRef, ForwardRefMarker};
use crate::ir::{HydroIrMetadata, HydroNode, HydroSource};
use crate::stream::ExactlyOnce;
use crate::{Singleton, Stream, TotalOrder, Unbounded};

pub mod external_process;
pub use external_process::ExternalProcess;

pub mod process;
pub use process::Process;

pub mod cluster;
pub use cluster::{Cluster, ClusterId};

pub mod can_send;
pub use can_send::CanSend;

pub mod tick;
pub use tick::{Atomic, NoTick, Tick};

#[derive(PartialEq, Eq, Clone, Debug, Hash, Serialize, Deserialize)]
pub enum LocationId {
    Process(usize),
    Cluster(usize),
    Tick(usize, Box<LocationId>),
    ExternalProcess(usize),
}

impl LocationId {
    pub fn root(&self) -> &LocationId {
        match self {
            LocationId::Process(_) => self,
            LocationId::Cluster(_) => self,
            LocationId::Tick(_, id) => id.root(),
            LocationId::ExternalProcess(_) => self,
        }
    }

    pub fn raw_id(&self) -> usize {
        match self {
            LocationId::Process(id) => *id,
            LocationId::Cluster(id) => *id,
            LocationId::Tick(_, _) => panic!("cannot get raw id for tick"),
            LocationId::ExternalProcess(id) => *id,
        }
    }

    pub fn swap_root(&mut self, new_root: LocationId) {
        match self {
            LocationId::Tick(_, id) => {
                id.swap_root(new_root);
            }
            _ => {
                *self = new_root;
            }
        }
    }
}

pub fn check_matching_location<'a, L: Location<'a>>(l1: &L, l2: &L) {
    assert_eq!(l1.id(), l2.id(), "locations do not match");
}

pub trait Location<'a>: Clone {
    type Root: Location<'a>;

    fn root(&self) -> Self::Root;

    fn id(&self) -> LocationId;

    fn flow_state(&self) -> &FlowState;

    fn is_top_level() -> bool;

    fn tick(&self) -> Tick<Self>
    where
        Self: NoTick,
    {
        let next_id = self.flow_state().borrow_mut().next_clock_id;
        self.flow_state().borrow_mut().next_clock_id += 1;
        Tick {
            id: next_id,
            l: self.clone(),
        }
    }

    fn next_node_id(&self) -> usize {
        let next_id = self.flow_state().borrow_mut().next_node_id;
        self.flow_state().borrow_mut().next_node_id += 1;
        next_id
    }

    #[inline(never)]
    fn new_node_metadata<T>(&self) -> HydroIrMetadata {
        HydroIrMetadata {
            location_kind: self.id(),
            backtrace: get_backtrace(2),
            output_type: Some(stageleft::quote_type::<T>().into()),
            cardinality: None,
            cpu_usage: None,
            network_recv_cpu_usage: None,
            id: None,
        }
    }

    fn spin(&self) -> Stream<(), Self, Unbounded, TotalOrder, ExactlyOnce>
    where
        Self: Sized + NoTick,
    {
        Stream::new(
            self.clone(),
            HydroNode::Persist {
                inner: Box::new(HydroNode::Source {
                    source: HydroSource::Spin(),
                    location_kind: self.id(),
                    metadata: self.new_node_metadata::<()>(),
                }),
                metadata: self.new_node_metadata::<()>(),
            },
        )
    }

    fn source_stream<T, E>(
        &self,
        e: impl QuotedWithContext<'a, E, Self>,
    ) -> Stream<T, Self, Unbounded, TotalOrder, ExactlyOnce>
    where
        E: FuturesStream<Item = T> + Unpin,
        Self: Sized + NoTick,
    {
        let e = e.splice_untyped_ctx(self);

        Stream::new(
            self.clone(),
            HydroNode::Persist {
                inner: Box::new(HydroNode::Source {
                    source: HydroSource::Stream(e.into()),
                    location_kind: self.id(),
                    metadata: self.new_node_metadata::<T>(),
                }),
                metadata: self.new_node_metadata::<T>(),
            },
        )
    }

    fn source_iter<T, E>(
        &self,
        e: impl QuotedWithContext<'a, E, Self>,
    ) -> Stream<T, Self, Unbounded, TotalOrder, ExactlyOnce>
    where
        E: IntoIterator<Item = T>,
        Self: Sized + NoTick,
    {
        // TODO(shadaj): we mark this as unbounded because we do not yet have a representation
        // for bounded top-level streams, and this is the only way to generate one
        let e = e.splice_untyped_ctx(self);

        Stream::new(
            self.clone(),
            HydroNode::Persist {
                inner: Box::new(HydroNode::Source {
                    source: HydroSource::Iter(e.into()),
                    location_kind: self.id(),
                    metadata: self.new_node_metadata::<T>(),
                }),
                metadata: self.new_node_metadata::<T>(),
            },
        )
    }

    fn singleton<T>(&self, e: impl QuotedWithContext<'a, T, Self>) -> Singleton<T, Self, Unbounded>
    where
        T: Clone,
        Self: Sized + NoTick,
    {
        // TODO(shadaj): we mark this as unbounded because we do not yet have a representation
        // for bounded top-level singletons, and this is the only way to generate one

        let e_arr = q!([e]);
        let e = e_arr.splice_untyped_ctx(self);

        // we do a double persist here because if the singleton shows up on every tick,
        // we first persist the source so that we store that value and then persist again
        // so that it grows every tick
        Singleton::new(
            self.clone(),
            HydroNode::Persist {
                inner: Box::new(HydroNode::Persist {
                    inner: Box::new(HydroNode::Source {
                        source: HydroSource::Iter(e.into()),
                        location_kind: self.id(),
                        metadata: self.new_node_metadata::<T>(),
                    }),
                    metadata: self.new_node_metadata::<T>(),
                }),
                metadata: self.new_node_metadata::<T>(),
            },
        )
    }

    /// Generates a stream with values emitted at a fixed interval, with
    /// each value being the current time (as an [`tokio::time::Instant`]).
    ///
    /// The clock source used is monotonic, so elements will be emitted in
    /// increasing order.
    ///
    /// # Safety
    /// Because this stream is generated by an OS timer, it will be
    /// non-deterministic because each timestamp will be arbitrary.
    unsafe fn source_interval(
        &self,
        interval: impl QuotedWithContext<'a, Duration, Self> + Copy + 'a,
    ) -> Stream<tokio::time::Instant, Self, Unbounded, TotalOrder, ExactlyOnce>
    where
        Self: Sized + NoTick,
    {
        self.source_stream(q!(tokio_stream::wrappers::IntervalStream::new(
            tokio::time::interval(interval)
        )))
    }

    /// Generates a stream with values emitted at a fixed interval (with an
    /// initial delay), with each value being the current time
    /// (as an [`tokio::time::Instant`]).
    ///
    /// The clock source used is monotonic, so elements will be emitted in
    /// increasing order.
    ///
    /// # Safety
    /// Because this stream is generated by an OS timer, it will be
    /// non-deterministic because each timestamp will be arbitrary.
    unsafe fn source_interval_delayed(
        &self,
        delay: impl QuotedWithContext<'a, Duration, Self> + Copy + 'a,
        interval: impl QuotedWithContext<'a, Duration, Self> + Copy + 'a,
    ) -> Stream<tokio::time::Instant, Self, Unbounded, TotalOrder, ExactlyOnce>
    where
        Self: Sized + NoTick,
    {
        self.source_stream(q!(tokio_stream::wrappers::IntervalStream::new(
            tokio::time::interval_at(tokio::time::Instant::now() + delay, interval)
        )))
    }

    fn forward_ref<S>(&self) -> (ForwardRef<'a, S>, S)
    where
        S: CycleCollection<'a, ForwardRefMarker, Location = Self>,
        Self: NoTick,
    {
        let next_id = self.flow_state().borrow_mut().next_cycle_id();
        let ident = syn::Ident::new(&format!("cycle_{}", next_id), Span::call_site());

        (
            ForwardRef {
                completed: false,
                ident: ident.clone(),
                expected_location: self.id(),
                _phantom: PhantomData,
            },
            S::create_source(ident, self.clone()),
        )
    }
}
