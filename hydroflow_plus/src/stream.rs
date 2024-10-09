use std::cell::RefCell;
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::Deref;
use std::rc::Rc;

use hydroflow::bytes::Bytes;
use hydroflow::futures::Sink;
use hydroflow_lang::parse::Pipeline;
use serde::de::DeserializeOwned;
use serde::Serialize;
use stageleft::{q, IntoQuotedMut, Quoted};
use syn::parse_quote;

use super::staging_util::get_this_crate;
use crate::builder::{self, FlowState};
use crate::cycle::{CycleCollection, CycleComplete};
use crate::ir::{DebugInstantiate, HfPlusLeaf, HfPlusNode, HfPlusSource, TeeNode};
use crate::location::{
    CanSend, ExternalBincodeStream, ExternalBytesPort, ExternalProcess, Location, LocationId,
};
use crate::{Cluster, ClusterId, Optional, Process, Singleton};

/// Marks the stream as being unbounded, which means that it is not
/// guaranteed to be complete in finite time.
pub enum Unbounded {}

/// Marks the stream as being bounded, which means that it is guaranteed
/// to be complete in finite time.
pub enum Bounded {}

/// Marks the stream as existing outside of a clock domain.
pub enum NoTick {}
/// Marks the stream as being inside the single global clock domain.
pub enum Tick {}

/// An infinite stream of elements of type `T`.
///
/// Type Parameters:
/// - `'a`: the lifetime of the final Hydroflow graph, which constraints
///   which values can be captured in closures passed to operators
/// - `T`: the type of elements in the stream
/// - `W`: the boundedness of the stream, which is either [`Bounded`]
///    or [`Unbounded`]
/// - `C`: the tick domain of the stream, which is either [`Tick`] or
///   [`NoTick`]
/// - `N`: the type of the node that the stream is materialized on
pub struct Stream<T, W, C, N> {
    location_kind: LocationId,

    flow_state: FlowState,
    pub(crate) ir_node: RefCell<HfPlusNode>,

    _phantom: PhantomData<(T, N, W, C)>,
}

impl<'a, T, N: Location<'a>> CycleComplete<'a, Tick> for Stream<T, Bounded, Tick, N> {
    fn complete(self, ident: syn::Ident) {
        let me = self.defer_tick();
        me.flow_state.borrow_mut().leaves.as_mut().expect("Attempted to add a leaf to a flow that has already been finalized. No leaves can be added after the flow has been compiled.").push(HfPlusLeaf::CycleSink {
            ident,
            location_kind: me.location_kind,
            input: Box::new(me.ir_node.into_inner()),
        });
    }
}

impl<'a, T, N: Location<'a>> CycleCollection<'a, Tick> for Stream<T, Bounded, Tick, N> {
    type Location = N;

    fn create_source(ident: syn::Ident, flow_state: FlowState, l: LocationId) -> Self {
        Stream::new(
            l,
            flow_state,
            HfPlusNode::CycleSource {
                ident,
                location_kind: l,
            },
        )
    }
}

impl<'a, T, W, N: Location<'a>> CycleComplete<'a, NoTick> for Stream<T, W, NoTick, N> {
    fn complete(self, ident: syn::Ident) {
        self.flow_state.borrow_mut().leaves.as_mut().expect("Attempted to add a leaf to a flow that has already been finalized. No leaves can be added after the flow has been compiled.").push(HfPlusLeaf::CycleSink {
            ident,
            location_kind: self.location_kind,
            input: Box::new(HfPlusNode::Unpersist(Box::new(self.ir_node.into_inner()))),
        });
    }
}

impl<'a, T, W, N: Location<'a>> CycleCollection<'a, NoTick> for Stream<T, W, NoTick, N> {
    type Location = N;

    fn create_source(ident: syn::Ident, flow_state: FlowState, l: LocationId) -> Self {
        Stream::new(
            l,
            flow_state,
            HfPlusNode::Persist(Box::new(HfPlusNode::CycleSource {
                ident,
                location_kind: l,
            })),
        )
    }
}

impl<'a, T, W, C, N: Location<'a>> Stream<T, W, C, N> {
    pub(crate) fn new(
        location_kind: LocationId,
        flow_state: FlowState,
        ir_node: HfPlusNode,
    ) -> Self {
        Stream {
            location_kind,
            flow_state,
            ir_node: RefCell::new(ir_node),
            _phantom: PhantomData,
        }
    }
}

impl<'a, T: Clone, W, C, N: Location<'a>> Clone for Stream<T, W, C, N> {
    fn clone(&self) -> Self {
        if !matches!(self.ir_node.borrow().deref(), HfPlusNode::Tee { .. }) {
            let orig_ir_node = self.ir_node.replace(HfPlusNode::Placeholder);
            *self.ir_node.borrow_mut() = HfPlusNode::Tee {
                inner: TeeNode(Rc::new(RefCell::new(orig_ir_node))),
            };
        }

        if let HfPlusNode::Tee { inner } = self.ir_node.borrow().deref() {
            Stream {
                location_kind: self.location_kind,
                flow_state: self.flow_state.clone(),
                ir_node: HfPlusNode::Tee {
                    inner: TeeNode(inner.0.clone()),
                }
                .into(),
                _phantom: PhantomData,
            }
        } else {
            unreachable!()
        }
    }
}

impl<'a, T, W, C, N: Location<'a>> Stream<T, W, C, N> {
    pub fn map<U, F: Fn(T) -> U + 'a>(self, f: impl IntoQuotedMut<'a, F>) -> Stream<U, W, C, N> {
        Stream::new(
            self.location_kind,
            self.flow_state,
            HfPlusNode::Map {
                f: f.splice_fn1().into(),
                input: Box::new(self.ir_node.into_inner()),
            },
        )
    }

    pub fn flat_map<U, I: IntoIterator<Item = U>, F: Fn(T) -> I + 'a>(
        self,
        f: impl IntoQuotedMut<'a, F>,
    ) -> Stream<U, W, C, N> {
        Stream::new(
            self.location_kind,
            self.flow_state,
            HfPlusNode::FlatMap {
                f: f.splice_fn1().into(),
                input: Box::new(self.ir_node.into_inner()),
            },
        )
    }

    pub fn filter<F: Fn(&T) -> bool + 'a>(
        self,
        f: impl IntoQuotedMut<'a, F>,
    ) -> Stream<T, W, C, N> {
        Stream::new(
            self.location_kind,
            self.flow_state,
            HfPlusNode::Filter {
                f: f.splice_fn1_borrow().into(),
                input: Box::new(self.ir_node.into_inner()),
            },
        )
    }

    pub fn filter_map<U, F: Fn(T) -> Option<U> + 'a>(
        self,
        f: impl IntoQuotedMut<'a, F>,
    ) -> Stream<U, W, C, N> {
        Stream::new(
            self.location_kind,
            self.flow_state,
            HfPlusNode::FilterMap {
                f: f.splice_fn1().into(),
                input: Box::new(self.ir_node.into_inner()),
            },
        )
    }

    pub fn cross_singleton<O>(
        self,
        other: impl Into<Optional<O, Bounded, C, N>>,
    ) -> Stream<(T, O), W, C, N>
    where
        O: Clone,
    {
        let other: Optional<O, Bounded, C, N> = other.into();
        if self.location_kind != other.location_kind {
            panic!("cross_singleton must be called on streams on the same node");
        }

        Stream::new(
            self.location_kind,
            self.flow_state,
            HfPlusNode::CrossSingleton(
                Box::new(self.ir_node.into_inner()),
                Box::new(other.ir_node.into_inner()),
            ),
        )
    }

    // TODO(shadaj): should allow for differing windows, using strongest one
    pub fn cross_product<O>(self, other: Stream<O, W, C, N>) -> Stream<(T, O), W, C, N>
    where
        T: Clone,
        O: Clone,
    {
        if self.location_kind != other.location_kind {
            panic!("cross_product must be called on streams on the same node");
        }

        Stream::new(
            self.location_kind,
            self.flow_state,
            HfPlusNode::CrossProduct(
                Box::new(self.ir_node.into_inner()),
                Box::new(other.ir_node.into_inner()),
            ),
        )
    }

    pub fn union(self, other: Stream<T, W, C, N>) -> Stream<T, W, C, N> {
        if self.location_kind != other.location_kind {
            panic!("union must be called on streams on the same node");
        }

        Stream::new(
            self.location_kind,
            self.flow_state,
            HfPlusNode::Union(
                Box::new(self.ir_node.into_inner()),
                Box::new(other.ir_node.into_inner()),
            ),
        )
    }

    pub fn unique(self) -> Stream<T, W, C, N>
    where
        T: Eq + Hash,
    {
        Stream::new(
            self.location_kind,
            self.flow_state,
            HfPlusNode::Unique(Box::new(self.ir_node.into_inner())),
        )
    }

    pub fn dest_sink<S: Unpin + Sink<T> + 'a>(self, sink: impl Quoted<'a, S>) {
        self.flow_state.borrow_mut().leaves.as_mut().expect("Attempted to add a leaf to a flow that has already been finalized. No leaves can be added after the flow has been compiled.").push(HfPlusLeaf::DestSink {
            sink: sink.splice_typed().into(),
            input: Box::new(self.ir_node.into_inner()),
        });
    }
}

impl<'a, T, N: Location<'a>> Stream<T, Bounded, Tick, N> {
    pub fn all_ticks(self) -> Stream<T, Unbounded, NoTick, N> {
        Stream::new(
            self.location_kind,
            self.flow_state,
            HfPlusNode::Persist(Box::new(self.ir_node.into_inner())),
        )
    }

    pub fn persist(self) -> Stream<T, Bounded, Tick, N>
    where
        T: Clone,
    {
        Stream::new(
            self.location_kind,
            self.flow_state,
            HfPlusNode::Persist(Box::new(self.ir_node.into_inner())),
        )
    }

    pub fn defer_tick(self) -> Stream<T, Bounded, Tick, N> {
        Stream::new(
            self.location_kind,
            self.flow_state,
            HfPlusNode::DeferTick(Box::new(self.ir_node.into_inner())),
        )
    }

    pub fn inspect<F: Fn(&T) + 'a>(
        self,
        f: impl IntoQuotedMut<'a, F>,
    ) -> Stream<T, Bounded, Tick, N> {
        Stream::new(
            self.location_kind,
            self.flow_state,
            HfPlusNode::Inspect {
                f: f.splice_fn1_borrow().into(),
                input: Box::new(self.ir_node.into_inner()),
            },
        )
    }

    pub fn first(self) -> Optional<T, Bounded, Tick, N> {
        Optional::new(
            self.location_kind,
            self.flow_state,
            self.ir_node.into_inner(),
        )
    }

    /// Allow this stream through if the other stream has elements, otherwise the output is empty.
    pub fn continue_if<U>(
        self,
        signal: Optional<U, Bounded, Tick, N>,
    ) -> Stream<T, Bounded, Tick, N> {
        self.cross_singleton(signal.map(q!(|_u| ())))
            .map(q!(|(d, _signal)| d))
    }

    /// Allow this stream through if the other stream is empty, otherwise the output is empty.
    pub fn continue_unless<U>(
        self,
        other: Optional<U, Bounded, Tick, N>,
    ) -> Stream<T, Bounded, Tick, N> {
        self.continue_if(other.into_stream().count().filter(q!(|c| *c == 0)))
    }

    pub fn enumerate(self) -> Stream<(usize, T), Bounded, Tick, N> {
        Stream::new(
            self.location_kind,
            self.flow_state,
            HfPlusNode::Enumerate(Box::new(self.ir_node.into_inner())),
        )
    }

    pub fn fold<A, I: Fn() -> A + 'a, F: Fn(&mut A, T)>(
        self,
        init: impl IntoQuotedMut<'a, I>,
        comb: impl IntoQuotedMut<'a, F>,
    ) -> Singleton<A, Bounded, Tick, N> {
        Singleton::new(
            self.location_kind,
            self.flow_state,
            HfPlusNode::Fold {
                init: init.splice_fn0().into(),
                acc: comb.splice_fn2_borrow_mut().into(),
                input: Box::new(self.ir_node.into_inner()),
            },
        )
    }

    pub fn reduce<F: Fn(&mut T, T) + 'a>(
        self,
        comb: impl IntoQuotedMut<'a, F>,
    ) -> Optional<T, Bounded, Tick, N> {
        Optional::new(
            self.location_kind,
            self.flow_state,
            HfPlusNode::Reduce {
                f: comb.splice_fn2_borrow_mut().into(),
                input: Box::new(self.ir_node.into_inner()),
            },
        )
    }

    pub fn max(self) -> Optional<T, Bounded, Tick, N>
    where
        T: Ord,
    {
        self.reduce(q!(|curr, new| {
            if new > *curr {
                *curr = new;
            }
        }))
    }

    pub fn min(self) -> Optional<T, Bounded, Tick, N>
    where
        T: Ord,
    {
        self.reduce(q!(|curr, new| {
            if new < *curr {
                *curr = new;
            }
        }))
    }

    pub fn sort(self) -> Stream<T, Bounded, Tick, N>
    where
        T: Ord,
    {
        Stream::new(
            self.location_kind,
            self.flow_state,
            HfPlusNode::Sort(Box::new(self.ir_node.into_inner())),
        )
    }

    pub fn count(self) -> Singleton<usize, Bounded, Tick, N> {
        self.fold(q!(|| 0usize), q!(|count, _| *count += 1))
    }

    pub fn delta(self) -> Stream<T, Bounded, Tick, N> {
        Stream::new(
            self.location_kind,
            self.flow_state,
            HfPlusNode::Delta(Box::new(self.ir_node.into_inner())),
        )
    }
}

impl<'a, T, W, N: Location<'a>> Stream<T, W, NoTick, N> {
    pub fn tick_batch(self) -> Stream<T, Bounded, Tick, N> {
        Stream::new(
            self.location_kind,
            self.flow_state,
            HfPlusNode::Unpersist(Box::new(self.ir_node.into_inner())),
        )
    }

    pub fn tick_prefix(self) -> Stream<T, Bounded, Tick, N>
    where
        T: Clone,
    {
        self.tick_batch().persist()
    }

    pub fn inspect<F: Fn(&T) + 'a>(self, f: impl IntoQuotedMut<'a, F>) -> Stream<T, W, NoTick, N> {
        Stream::new(
            self.location_kind,
            self.flow_state,
            HfPlusNode::Persist(Box::new(HfPlusNode::Inspect {
                f: f.splice_fn1_borrow().into(),
                input: Box::new(HfPlusNode::Unpersist(Box::new(self.ir_node.into_inner()))),
            })),
        )
    }
}

impl<'a, T, W, N: Location<'a>> Stream<T, W, NoTick, N> {
    pub fn for_each<F: Fn(T) + 'a>(self, f: impl IntoQuotedMut<'a, F>) {
        self.flow_state.borrow_mut().leaves.as_mut().expect("Attempted to add a leaf to a flow that has already been finalized. No leaves can be added after the flow has been compiled.").push(HfPlusLeaf::ForEach {
            input: Box::new(HfPlusNode::Unpersist(Box::new(self.ir_node.into_inner()))),
            f: f.splice_fn1().into(),
        });
    }
}

impl<'a, T, N: Location<'a>> Stream<T, Unbounded, NoTick, N> {
    pub fn sample_every(
        self,
        duration: impl Quoted<'a, std::time::Duration> + Copy + 'a,
    ) -> Stream<T, Unbounded, NoTick, N> {
        let interval = duration.splice_typed();

        let samples = Stream::<tokio::time::Instant, Bounded, Tick, N>::new(
            self.location_kind,
            self.flow_state.clone(),
            HfPlusNode::Source {
                source: HfPlusSource::Interval(interval.into()),
                location_kind: self.location_kind,
            },
        );

        self.tick_batch().continue_if(samples.first()).all_ticks()
    }

    pub fn fold<A, I: Fn() -> A + 'a, F: Fn(&mut A, T)>(
        self,
        init: impl IntoQuotedMut<'a, I>,
        comb: impl IntoQuotedMut<'a, F>,
    ) -> Singleton<A, Unbounded, NoTick, N> {
        // unbounded singletons are represented as a stream
        // which produces all values from all ticks every tick,
        // so delta will always give the lastest aggregation
        Singleton::new(
            self.location_kind,
            self.flow_state,
            HfPlusNode::Persist(Box::new(HfPlusNode::Fold {
                init: init.splice_fn0().into(),
                acc: comb.splice_fn2_borrow_mut().into(),
                input: Box::new(self.ir_node.into_inner()),
            })),
        )
    }

    pub fn reduce<F: Fn(&mut T, T) + 'a>(
        self,
        comb: impl IntoQuotedMut<'a, F>,
    ) -> Optional<T, Unbounded, NoTick, N> {
        Optional::new(
            self.location_kind,
            self.flow_state,
            HfPlusNode::Persist(Box::new(HfPlusNode::Reduce {
                f: comb.splice_fn2_borrow_mut().into(),
                input: Box::new(self.ir_node.into_inner()),
            })),
        )
    }

    pub fn max(self) -> Optional<T, Unbounded, NoTick, N>
    where
        T: Ord,
    {
        self.reduce(q!(|curr, new| {
            if new > *curr {
                *curr = new;
            }
        }))
    }

    pub fn min(self) -> Optional<T, Unbounded, NoTick, N>
    where
        T: Ord,
    {
        self.reduce(q!(|curr, new| {
            if new < *curr {
                *curr = new;
            }
        }))
    }
}

impl<'a, T, C, N: Location<'a>> Stream<T, Bounded, C, N> {
    pub fn filter_not_in(self, other: Stream<T, Bounded, C, N>) -> Stream<T, Bounded, C, N>
    where
        T: Eq + Hash,
    {
        if self.location_kind != other.location_kind {
            panic!("union must be called on streams on the same node");
        }

        Stream::new(
            self.location_kind,
            self.flow_state,
            HfPlusNode::Difference(
                Box::new(self.ir_node.into_inner()),
                Box::new(other.ir_node.into_inner()),
            ),
        )
    }
}

impl<'a, T: Clone, W, C, N: Location<'a>> Stream<&T, W, C, N> {
    pub fn cloned(self) -> Stream<T, W, C, N> {
        self.map(q!(|d| d.clone()))
    }
}

impl<'a, K, V1, W, C, N: Location<'a>> Stream<(K, V1), W, C, N> {
    // TODO(shadaj): figure out window semantics
    pub fn join<W2, V2>(self, n: Stream<(K, V2), W2, C, N>) -> Stream<(K, (V1, V2)), W, C, N>
    where
        K: Eq + Hash,
    {
        if self.location_kind != n.location_kind {
            panic!("join must be called on streams on the same node");
        }

        Stream::new(
            self.location_kind,
            self.flow_state,
            HfPlusNode::Join(
                Box::new(self.ir_node.into_inner()),
                Box::new(n.ir_node.into_inner()),
            ),
        )
    }

    pub fn anti_join<W2>(self, n: Stream<K, W2, C, N>) -> Stream<(K, V1), W, C, N>
    where
        K: Eq + Hash,
    {
        if self.location_kind != n.location_kind {
            panic!("anti_join must be called on streams on the same node");
        }

        Stream::new(
            self.location_kind,
            self.flow_state,
            HfPlusNode::AntiJoin(
                Box::new(self.ir_node.into_inner()),
                Box::new(n.ir_node.into_inner()),
            ),
        )
    }
}

impl<'a, K: Eq + Hash, V, N: Location<'a>> Stream<(K, V), Bounded, Tick, N> {
    pub fn fold_keyed<A, I: Fn() -> A + 'a, F: Fn(&mut A, V) + 'a>(
        self,
        init: impl IntoQuotedMut<'a, I>,
        comb: impl IntoQuotedMut<'a, F>,
    ) -> Stream<(K, A), Bounded, Tick, N> {
        Stream::new(
            self.location_kind,
            self.flow_state,
            HfPlusNode::FoldKeyed {
                init: init.splice_fn0().into(),
                acc: comb.splice_fn2_borrow_mut().into(),
                input: Box::new(self.ir_node.into_inner()),
            },
        )
    }

    pub fn reduce_keyed<F: Fn(&mut V, V) + 'a>(
        self,
        comb: impl IntoQuotedMut<'a, F>,
    ) -> Stream<(K, V), Bounded, Tick, N> {
        Stream::new(
            self.location_kind,
            self.flow_state,
            HfPlusNode::ReduceKeyed {
                f: comb.splice_fn2_borrow_mut().into(),
                input: Box::new(self.ir_node.into_inner()),
            },
        )
    }
}

fn serialize_bincode<T: Serialize>(is_demux: bool) -> Pipeline {
    let root = get_this_crate();

    let t_type: syn::Type = stageleft::quote_type::<T>();

    if is_demux {
        parse_quote! {
            map(|(id, data): (#root::ClusterId<_>, #t_type)| {
                (id.raw_id, #root::runtime_support::bincode::serialize::<#t_type>(&data).unwrap().into())
            })
        }
    } else {
        parse_quote! {
            map(|data| {
                #root::runtime_support::bincode::serialize::<#t_type>(&data).unwrap().into()
            })
        }
    }
}

pub(super) fn deserialize_bincode<T: DeserializeOwned>(tagged: Option<syn::Type>) -> Pipeline {
    let root = get_this_crate();

    let t_type: syn::Type = stageleft::quote_type::<T>();

    if let Some(c_type) = tagged {
        parse_quote! {
            map(|res| {
                let (id, b) = res.unwrap();
                (#root::ClusterId::<#c_type>::from_raw(id), #root::runtime_support::bincode::deserialize::<#t_type>(&b).unwrap())
            })
        }
    } else {
        parse_quote! {
            map(|res| {
                #root::runtime_support::bincode::deserialize::<#t_type>(&res.unwrap()).unwrap()
            })
        }
    }
}

impl<'a, T, W, N: Location<'a>> Stream<T, W, NoTick, N> {
    pub fn decouple_process<P2>(
        self,
        other: &Process<'a, P2>,
    ) -> Stream<T, Unbounded, NoTick, Process<'a, P2>>
    where
        N: CanSend<'a, Process<'a, P2>, In<T> = T, Out<T> = T>,
        T: Clone + Serialize + DeserializeOwned,
    {
        self.send_bincode::<Process<'a, P2>, T>(other)
    }

    pub fn decouple_cluster<C2, Tag>(
        self,
        other: &Cluster<'a, C2>,
    ) -> Stream<T, Unbounded, NoTick, Cluster<'a, C2>>
    where
        N: CanSend<'a, Cluster<'a, C2>, In<T> = (ClusterId<C2>, T), Out<T> = (Tag, T)>,
        T: Clone + Serialize + DeserializeOwned,
    {
        let self_node_id = match self.location_kind {
            LocationId::Cluster(cluster_id) => builder::ClusterSelfId {
                id: cluster_id,
                _phantom: PhantomData,
            },
            _ => panic!("decouple_cluster must be called on a cluster"),
        };

        self.map(q!(move |b| (self_node_id, b.clone())))
            .send_bincode_interleaved(other)
    }

    pub fn send_bincode<N2: Location<'a>, CoreType>(
        self,
        other: &N2,
    ) -> Stream<N::Out<CoreType>, Unbounded, NoTick, N2>
    where
        N: CanSend<'a, N2, In<CoreType> = T>,
        CoreType: Serialize + DeserializeOwned,
    {
        let serialize_pipeline = Some(serialize_bincode::<CoreType>(N::is_demux()));

        let deserialize_pipeline = Some(deserialize_bincode::<CoreType>(N::tagged_type()));

        Stream::new(
            other.id(),
            self.flow_state,
            HfPlusNode::Network {
                from_location: self.location_kind,
                from_key: None,
                to_location: other.id(),
                to_key: None,
                serialize_pipeline,
                instantiate_fn: DebugInstantiate::Building(),
                deserialize_pipeline,
                input: Box::new(self.ir_node.into_inner()),
            },
        )
    }

    pub fn send_bincode_external<N2: 'a, CoreType>(
        self,
        other: &ExternalProcess<N2>,
    ) -> ExternalBincodeStream<N::Out<CoreType>>
    where
        N: CanSend<'a, ExternalProcess<'a, N2>, In<CoreType> = T, Out<CoreType> = CoreType>,
        CoreType: Serialize + DeserializeOwned,
        // for now, we restirct Out<CoreType> to be CoreType, which means no tagged cluster -> external
    {
        let serialize_pipeline = Some(serialize_bincode::<CoreType>(N::is_demux()));

        let mut flow_state_borrow = self.flow_state.borrow_mut();

        let external_key = flow_state_borrow.next_external_out;
        flow_state_borrow.next_external_out += 1;

        let leaves = flow_state_borrow.leaves.as_mut().expect("Attempted to add a leaf to a flow that has already been finalized. No leaves can be added after the flow has been compiled()");

        let dummy_f: syn::Expr = syn::parse_quote!(());

        leaves.push(HfPlusLeaf::ForEach {
            f: dummy_f.into(),
            input: Box::new(HfPlusNode::Network {
                from_location: self.location_kind,
                from_key: None,
                to_location: other.id(),
                to_key: Some(external_key),
                serialize_pipeline,
                instantiate_fn: DebugInstantiate::Building(),
                deserialize_pipeline: None,
                input: Box::new(self.ir_node.into_inner()),
            }),
        });

        ExternalBincodeStream {
            process_id: other.id,
            port_id: external_key,
            _phantom: PhantomData,
        }
    }

    pub fn send_bytes<N2: Location<'a>>(
        self,
        other: &N2,
    ) -> Stream<N::Out<Bytes>, Unbounded, NoTick, N2>
    where
        N: CanSend<'a, N2, In<Bytes> = T>,
    {
        let root = get_this_crate();
        Stream::new(
            other.id(),
            self.flow_state,
            HfPlusNode::Network {
                from_location: self.location_kind,
                from_key: None,
                to_location: other.id(),
                to_key: None,
                serialize_pipeline: None,
                instantiate_fn: DebugInstantiate::Building(),
                deserialize_pipeline: if let Some(c_type) = N::tagged_type() {
                    Some(
                        parse_quote!(map(|(id, b)| (#root::ClusterId<#c_type>::from_raw(id), b.unwrap().freeze()))),
                    )
                } else {
                    Some(parse_quote!(map(|b| b.unwrap().freeze())))
                },
                input: Box::new(self.ir_node.into_inner()),
            },
        )
    }

    pub fn send_bytes_external<N2: 'a>(self, other: &ExternalProcess<N2>) -> ExternalBytesPort
    where
        N: CanSend<'a, ExternalProcess<'a, N2>, In<Bytes> = T, Out<Bytes> = Bytes>,
    {
        let mut flow_state_borrow = self.flow_state.borrow_mut();
        let external_key = flow_state_borrow.next_external_out;
        flow_state_borrow.next_external_out += 1;

        let leaves = flow_state_borrow.leaves.as_mut().expect("Attempted to add a leaf to a flow that has already been finalized. No leaves can be added after the flow has been compiled()");

        let dummy_f: syn::Expr = syn::parse_quote!(());

        leaves.push(HfPlusLeaf::ForEach {
            f: dummy_f.into(),
            input: Box::new(HfPlusNode::Network {
                from_location: self.location_kind,
                from_key: None,
                to_location: other.id(),
                to_key: Some(external_key),
                serialize_pipeline: None,
                instantiate_fn: DebugInstantiate::Building(),
                deserialize_pipeline: None,
                input: Box::new(self.ir_node.into_inner()),
            }),
        });

        ExternalBytesPort {
            process_id: other.id,
            port_id: external_key,
        }
    }

    pub fn send_bincode_interleaved<N2: Location<'a>, Tag, CoreType>(
        self,
        other: &N2,
    ) -> Stream<CoreType, Unbounded, NoTick, N2>
    where
        N: CanSend<'a, N2, In<CoreType> = T, Out<CoreType> = (Tag, CoreType)>,
        CoreType: Serialize + DeserializeOwned,
    {
        self.send_bincode::<N2, CoreType>(other).map(q!(|(_, b)| b))
    }

    pub fn send_bytes_interleaved<N2: Location<'a>, Tag>(
        self,
        other: &N2,
    ) -> Stream<Bytes, Unbounded, NoTick, N2>
    where
        N: CanSend<'a, N2, In<Bytes> = T, Out<Bytes> = (Tag, Bytes)>,
    {
        self.send_bytes::<N2>(other).map(q!(|(_, b)| b))
    }

    pub fn broadcast_bincode<C2>(
        self,
        other: &Cluster<'a, C2>,
    ) -> Stream<N::Out<T>, Unbounded, NoTick, Cluster<'a, C2>>
    where
        N: CanSend<'a, Cluster<'a, C2>, In<T> = (ClusterId<C2>, T)>,
        T: Clone + Serialize + DeserializeOwned,
    {
        let ids = other.members();

        self.flat_map(q!(|b| ids.iter().map(move |id| (
            ::std::clone::Clone::clone(id),
            ::std::clone::Clone::clone(&b)
        ))))
        .send_bincode(other)
    }

    pub fn broadcast_bincode_interleaved<C2, Tag>(
        self,
        other: &Cluster<'a, C2>,
    ) -> Stream<T, Unbounded, NoTick, Cluster<'a, C2>>
    where
        N: CanSend<'a, Cluster<'a, C2>, In<T> = (ClusterId<C2>, T), Out<T> = (Tag, T)> + 'a,
        T: Clone + Serialize + DeserializeOwned,
    {
        self.broadcast_bincode(other).map(q!(|(_, b)| b))
    }

    pub fn broadcast_bytes<C2>(
        self,
        other: &Cluster<'a, C2>,
    ) -> Stream<N::Out<Bytes>, Unbounded, NoTick, Cluster<'a, C2>>
    where
        N: CanSend<'a, Cluster<'a, C2>, In<Bytes> = (ClusterId<C2>, T)> + 'a,
        T: Clone,
    {
        let ids = other.members();

        self.flat_map(q!(|b| ids.iter().map(move |id| (
            ::std::clone::Clone::clone(id),
            ::std::clone::Clone::clone(&b)
        ))))
        .send_bytes(other)
    }

    pub fn broadcast_bytes_interleaved<C2, Tag>(
        self,
        other: &Cluster<'a, C2>,
    ) -> Stream<Bytes, Unbounded, NoTick, Cluster<'a, C2>>
    where
        N: CanSend<'a, Cluster<'a, C2>, In<Bytes> = (ClusterId<C2>, T), Out<Bytes> = (Tag, Bytes)>
            + 'a,
        T: Clone,
    {
        self.broadcast_bytes(other).map(q!(|(_, b)| b))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use hydro_deploy::{Deployment, Host};
    use hydroflow::futures::StreamExt;
    use serde::{Deserialize, Serialize};
    use stageleft::q;

    use crate::deploy::TrybuildHost;
    use crate::location::Location;
    use crate::FlowBuilder;

    struct P1 {}
    struct P2 {}

    #[derive(Serialize, Deserialize, Debug)]
    struct SendOverNetwork {
        n: u32,
    }

    #[tokio::test]
    async fn first_ten_distributed() {
        let mut deployment = Deployment::new();

        let flow = FlowBuilder::new();
        let first_node = flow.process::<P1>();
        let second_node = flow.process::<P2>();
        let external = flow.external_process::<P2>();

        let numbers = first_node.source_iter(q!(0..10));
        let out_port = numbers
            .map(q!(|n| SendOverNetwork { n }))
            .send_bincode(&second_node)
            .send_bincode_external(&external);

        let nodes = flow
            .with_default_optimize()
            .with_process(&first_node, TrybuildHost::new(deployment.Localhost()))
            .with_process(&second_node, TrybuildHost::new(deployment.Localhost()))
            .with_external(&external, deployment.Localhost() as Arc<dyn Host>)
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut external_out = nodes.connect_source_bincode(out_port).await;

        deployment.start().await.unwrap();

        for i in 0..10 {
            assert_eq!(external_out.next().await.unwrap().n, i);
        }
    }
}
