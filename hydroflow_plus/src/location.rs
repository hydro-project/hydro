use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::marker::PhantomData;
use std::time::Duration;

use hydroflow::bytes::Bytes;
use hydroflow::futures::stream::Stream as FuturesStream;
use hydroflow::{tokio, tokio_stream};
use proc_macro2::Span;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use stageleft::{q, quote_type, Quoted};

use super::builder::{ClusterIds, ClusterSelfId, FlowState};
use crate::cycle::{CycleCollection, CycleCollectionWithInitial};
use crate::ir::{HfPlusNode, HfPlusSource};
use crate::{Bounded, HfCycle, NoTick, Optional, Singleton, Stream, Tick, Unbounded};

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum LocationId {
    Process(usize),
    Cluster(usize),
    ExternalProcess(usize),
}

pub trait Location<'a> {
    fn id(&self) -> LocationId;

    fn flow_state(&self) -> &FlowState;

    fn spin(&self) -> Stream<(), Unbounded, NoTick, Self>
    where
        Self: Sized,
    {
        Stream::new(
            self.id(),
            self.flow_state().clone(),
            HfPlusNode::Persist(Box::new(HfPlusNode::Source {
                source: HfPlusSource::Spin(),
                location_kind: self.id(),
            })),
        )
    }

    fn spin_batch(
        &self,
        batch_size: impl Quoted<'a, usize> + Copy + 'a,
    ) -> Stream<(), Bounded, Tick, Self>
    where
        Self: Sized,
    {
        self.spin()
            .flat_map(q!(move |_| 0..batch_size))
            .map(q!(|_| ()))
            .tick_batch()
    }

    fn source_stream<T, E: FuturesStream<Item = T> + Unpin>(
        &self,
        e: impl Quoted<'a, E>,
    ) -> Stream<T, Unbounded, NoTick, Self>
    where
        Self: Sized,
    {
        let e = e.splice_untyped();

        Stream::new(
            self.id(),
            self.flow_state().clone(),
            HfPlusNode::Persist(Box::new(HfPlusNode::Source {
                source: HfPlusSource::Stream(e.into()),
                location_kind: self.id(),
            })),
        )
    }

    fn source_iter<T, E: IntoIterator<Item = T>>(
        &self,
        e: impl Quoted<'a, E>,
    ) -> Stream<T, Bounded, NoTick, Self>
    where
        Self: Sized,
    {
        let e = e.splice_untyped();

        Stream::new(
            self.id(),
            self.flow_state().clone(),
            HfPlusNode::Persist(Box::new(HfPlusNode::Source {
                source: HfPlusSource::Iter(e.into()),
                location_kind: self.id(),
            })),
        )
    }

    fn singleton<T: Clone>(&self, e: impl Quoted<'a, T>) -> Singleton<T, Bounded, NoTick, Self>
    where
        Self: Sized,
    {
        let e_arr = q!([e]);
        let e = e_arr.splice_untyped();

        // we do a double persist here because if the singleton shows up on every tick,
        // we first persist the source so that we store that value and then persist again
        // so that it grows every tick
        Singleton::new(
            self.id(),
            self.flow_state().clone(),
            HfPlusNode::Persist(Box::new(HfPlusNode::Persist(Box::new(
                HfPlusNode::Source {
                    source: HfPlusSource::Iter(e.into()),
                    location_kind: self.id(),
                },
            )))),
        )
    }

    fn singleton_first_tick<T: Clone>(
        &self,
        e: impl Quoted<'a, T>,
    ) -> Optional<T, Bounded, Tick, Self>
    where
        Self: Sized,
    {
        let e_arr = q!([e]);
        let e = e_arr.splice_untyped();

        Optional::new(
            self.id(),
            self.flow_state().clone(),
            HfPlusNode::Source {
                source: HfPlusSource::Iter(e.into()),
                location_kind: self.id(),
            },
        )
    }

    fn source_interval(
        &self,
        interval: impl Quoted<'a, Duration> + Copy + 'a,
    ) -> Optional<(), Unbounded, NoTick, Self>
    where
        Self: Sized,
    {
        let interval = interval.splice_untyped();

        Optional::new(
            self.id(),
            self.flow_state().clone(),
            HfPlusNode::Persist(Box::new(HfPlusNode::Source {
                source: HfPlusSource::Interval(interval.into()),
                location_kind: self.id(),
            })),
        )
    }

    fn source_interval_delayed(
        &self,
        delay: impl Quoted<'a, Duration> + Copy + 'a,
        interval: impl Quoted<'a, Duration> + Copy + 'a,
    ) -> Optional<tokio::time::Instant, Unbounded, NoTick, Self>
    where
        Self: Sized,
    {
        self.source_stream(q!(tokio_stream::wrappers::IntervalStream::new(
            tokio::time::interval_at(tokio::time::Instant::now() + delay, interval)
        )))
        .tick_batch()
        .first()
        .latest()
    }

    fn tick_cycle<S: CycleCollection<'a, Tick, Location = Self>>(
        &self,
    ) -> (HfCycle<'a, Tick, S>, S) {
        let next_id = {
            let on_id = match self.id() {
                LocationId::Process(id) => id,
                LocationId::Cluster(id) => id,
                LocationId::ExternalProcess(_) => panic!(),
            };

            let mut flow_state = self.flow_state().borrow_mut();
            let next_id_entry = flow_state.cycle_counts.entry(on_id).or_default();

            let id = *next_id_entry;
            *next_id_entry += 1;
            id
        };

        let ident = syn::Ident::new(&format!("cycle_{}", next_id), Span::call_site());

        (
            HfCycle {
                ident: ident.clone(),
                _phantom: PhantomData,
            },
            S::create_source(ident, self.flow_state().clone(), self.id()),
        )
    }

    fn cycle<S: CycleCollection<'a, NoTick, Location = Self>>(
        &self,
    ) -> (HfCycle<'a, NoTick, S>, S) {
        let next_id = {
            let on_id = match self.id() {
                LocationId::Process(id) => id,
                LocationId::Cluster(id) => id,
                LocationId::ExternalProcess(_) => panic!(),
            };

            let mut flow_state = self.flow_state().borrow_mut();
            let next_id_entry = flow_state.cycle_counts.entry(on_id).or_default();

            let id = *next_id_entry;
            *next_id_entry += 1;
            id
        };

        let ident = syn::Ident::new(&format!("cycle_{}", next_id), Span::call_site());

        (
            HfCycle {
                ident: ident.clone(),
                _phantom: PhantomData,
            },
            S::create_source(ident, self.flow_state().clone(), self.id()),
        )
    }

    fn tick_cycle_with_initial<S: CycleCollectionWithInitial<'a, Tick, Location = Self>>(
        &self,
        initial: S,
    ) -> (HfCycle<'a, Tick, S>, S) {
        let next_id = {
            let on_id = match self.id() {
                LocationId::Process(id) => id,
                LocationId::Cluster(id) => id,
                LocationId::ExternalProcess(_) => panic!(),
            };

            let mut flow_state = self.flow_state().borrow_mut();
            let next_id_entry = flow_state.cycle_counts.entry(on_id).or_default();

            let id = *next_id_entry;
            *next_id_entry += 1;
            id
        };

        let ident = syn::Ident::new(&format!("cycle_{}", next_id), Span::call_site());

        (
            HfCycle {
                ident: ident.clone(),
                _phantom: PhantomData,
            },
            S::create_source(ident, self.flow_state().clone(), initial, self.id()),
        )
    }
}

pub struct ExternalBytesPort {
    pub(crate) process_id: usize,
    pub(crate) port_id: usize,
}

pub struct ExternalBincodeSink<T: Serialize> {
    pub(crate) process_id: usize,
    pub(crate) port_id: usize,
    pub(crate) _phantom: PhantomData<T>,
}

pub struct ExternalBincodeStream<T: DeserializeOwned> {
    pub(crate) process_id: usize,
    pub(crate) port_id: usize,
    pub(crate) _phantom: PhantomData<T>,
}

pub struct ExternalProcess<'a, P> {
    pub(crate) id: usize,

    pub(crate) flow_state: FlowState,

    pub(crate) _phantom: PhantomData<&'a &'a mut P>,
}

impl<'a, P> Clone for ExternalProcess<'a, P> {
    fn clone(&self) -> Self {
        ExternalProcess {
            id: self.id,
            flow_state: self.flow_state.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<'a, P> Location<'a> for ExternalProcess<'a, P> {
    fn id(&self) -> LocationId {
        LocationId::ExternalProcess(self.id)
    }

    fn flow_state(&self) -> &FlowState {
        &self.flow_state
    }
}

impl<'a, P> ExternalProcess<'a, P> {
    pub fn source_external_bytes<L: Location<'a>>(
        &self,
        to: &L,
    ) -> (ExternalBytesPort, Stream<Bytes, Unbounded, NoTick, L>) {
        let next_external_port_id = {
            let mut flow_state = self.flow_state.borrow_mut();
            let id = flow_state.next_external_out;
            flow_state.next_external_out += 1;
            id
        };

        (
            ExternalBytesPort {
                process_id: self.id,
                port_id: next_external_port_id,
            },
            Stream::new(
                to.id(),
                self.flow_state().clone(),
                HfPlusNode::Persist(Box::new(HfPlusNode::Network {
                    from_location: LocationId::ExternalProcess(self.id),
                    from_key: Some(next_external_port_id),
                    to_location: to.id(),
                    to_key: None,
                    serialize_pipeline: None,
                    instantiate_fn: crate::ir::DebugInstantiate::Building(),
                    deserialize_pipeline: Some(syn::parse_quote!(map(|b| b.unwrap().freeze()))),
                    input: Box::new(HfPlusNode::Source {
                        source: HfPlusSource::ExternalNetwork(),
                        location_kind: LocationId::ExternalProcess(self.id),
                    }),
                })),
            ),
        )
    }

    pub fn source_external_bincode<L: Location<'a>, T: Serialize + DeserializeOwned>(
        &self,
        to: &L,
    ) -> (ExternalBincodeSink<T>, Stream<T, Unbounded, NoTick, L>) {
        let next_external_port_id = {
            let mut flow_state = self.flow_state.borrow_mut();
            let id = flow_state.next_external_out;
            flow_state.next_external_out += 1;
            id
        };

        (
            ExternalBincodeSink {
                process_id: self.id,
                port_id: next_external_port_id,
                _phantom: PhantomData,
            },
            Stream::new(
                to.id(),
                self.flow_state().clone(),
                HfPlusNode::Persist(Box::new(HfPlusNode::Network {
                    from_location: LocationId::ExternalProcess(self.id),
                    from_key: Some(next_external_port_id),
                    to_location: to.id(),
                    to_key: None,
                    serialize_pipeline: None,
                    instantiate_fn: crate::ir::DebugInstantiate::Building(),
                    deserialize_pipeline: Some(crate::stream::deserialize_bincode::<T>(None)),
                    input: Box::new(HfPlusNode::Source {
                        source: HfPlusSource::ExternalNetwork(),
                        location_kind: LocationId::ExternalProcess(self.id),
                    }),
                })),
            ),
        )
    }
}

pub struct Process<'a, P> {
    pub(crate) id: usize,
    pub(crate) flow_state: FlowState,
    pub(crate) _phantom: PhantomData<&'a &'a mut P>,
}

impl<'a, P> Clone for Process<'a, P> {
    fn clone(&self) -> Self {
        Process {
            id: self.id,
            flow_state: self.flow_state.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<'a, P> Location<'a> for Process<'a, P> {
    fn id(&self) -> LocationId {
        LocationId::Process(self.id)
    }

    fn flow_state(&self) -> &FlowState {
        &self.flow_state
    }
}

#[repr(transparent)]
pub struct ClusterId<C> {
    pub raw_id: u32,
    pub(crate) _phantom: PhantomData<C>,
}

impl<C> Debug for ClusterId<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ClusterId::<{}>({})",
            std::any::type_name::<C>(),
            self.raw_id
        )
    }
}

impl<C> Display for ClusterId<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ClusterId::<{}>({})",
            std::any::type_name::<C>(),
            self.raw_id
        )
    }
}

impl<C> Clone for ClusterId<C> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<C> Copy for ClusterId<C> {}

impl<C> Serialize for ClusterId<C> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        self.raw_id.serialize(serializer)
    }
}

impl<'de, C> Deserialize<'de> for ClusterId<C> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        u32::deserialize(deserializer).map(|id| ClusterId {
            raw_id: id,
            _phantom: PhantomData,
        })
    }
}

impl<C> PartialEq for ClusterId<C> {
    fn eq(&self, other: &Self) -> bool {
        self.raw_id == other.raw_id
    }
}

impl<C> Eq for ClusterId<C> {}

impl<C> PartialOrd for ClusterId<C> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<C> Ord for ClusterId<C> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.raw_id.cmp(&other.raw_id)
    }
}

impl<C> Hash for ClusterId<C> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.raw_id.hash(state)
    }
}

impl<C> ClusterId<C> {
    pub fn from_raw(id: u32) -> Self {
        ClusterId {
            raw_id: id,
            _phantom: PhantomData,
        }
    }
}

pub struct Cluster<'a, C> {
    pub(crate) id: usize,
    pub(crate) flow_state: FlowState,
    pub(crate) _phantom: PhantomData<&'a &'a mut C>,
}

impl<'a, C> Cluster<'a, C> {
    pub fn self_id(&self) -> impl Quoted<'a, ClusterId<C>> + Copy + 'a {
        ClusterSelfId {
            id: self.id,
            _phantom: PhantomData,
        }
    }

    pub fn members(&self) -> impl Quoted<'a, &'a Vec<ClusterId<C>>> + Copy + 'a {
        ClusterIds {
            id: self.id,
            _phantom: PhantomData,
        }
    }
}

impl<'a, C> Clone for Cluster<'a, C> {
    fn clone(&self) -> Self {
        Cluster {
            id: self.id,
            flow_state: self.flow_state.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<'a, C> Location<'a> for Cluster<'a, C> {
    fn id(&self) -> LocationId {
        LocationId::Cluster(self.id)
    }

    fn flow_state(&self) -> &FlowState {
        &self.flow_state
    }
}

pub trait CanSend<'a, To: Location<'a>>: Location<'a> {
    type In<T>;
    type Out<T>;

    fn is_demux() -> bool;
    fn tagged_type() -> Option<syn::Type>;
}

impl<'a, P1, P2> CanSend<'a, Process<'a, P2>> for Process<'a, P1> {
    type In<T> = T;
    type Out<T> = T;

    fn is_demux() -> bool {
        false
    }

    fn tagged_type() -> Option<syn::Type> {
        None
    }
}

impl<'a, P1, C2> CanSend<'a, Cluster<'a, C2>> for Process<'a, P1> {
    type In<T> = (ClusterId<C2>, T);
    type Out<T> = T;

    fn is_demux() -> bool {
        true
    }

    fn tagged_type() -> Option<syn::Type> {
        None
    }
}

impl<'a, C1, P2> CanSend<'a, Process<'a, P2>> for Cluster<'a, C1> {
    type In<T> = T;
    type Out<T> = (ClusterId<C1>, T);

    fn is_demux() -> bool {
        false
    }

    fn tagged_type() -> Option<syn::Type> {
        Some(quote_type::<C1>())
    }
}

impl<'a, C1, C2> CanSend<'a, Cluster<'a, C2>> for Cluster<'a, C1> {
    type In<T> = (ClusterId<C2>, T);
    type Out<T> = (ClusterId<C1>, T);

    fn is_demux() -> bool {
        true
    }

    fn tagged_type() -> Option<syn::Type> {
        Some(quote_type::<C1>())
    }
}

impl<'a, P1, E2> CanSend<'a, ExternalProcess<'a, E2>> for Process<'a, P1> {
    type In<T> = T;
    type Out<T> = T;

    fn is_demux() -> bool {
        false
    }

    fn tagged_type() -> Option<syn::Type> {
        None
    }
}
