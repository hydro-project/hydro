use std::fmt::Debug;
use std::marker::PhantomData;
use std::time::Duration;

use bytes::BytesMut;
use futures::stream::Stream as FuturesStream;
use proc_macro2::Span;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use stageleft::{QuotedWithContext, q, quote_type};

use super::builder::FlowState;
use crate::backtrace::get_backtrace;
use crate::cycle::{CycleCollection, ForwardRef, ForwardRefMarker};
use crate::ir::{DebugInstantiate, HydroIrMetadata, HydroNode, HydroSource};
use crate::location::external_process::{ExternalBincodeSink, ExternalBytesPort, Many};
use crate::staging_util::get_this_crate;
use crate::stream::ExactlyOnce;
use crate::{NoOrder, Singleton, Stream, TotalOrder, Unbounded};

pub mod external_process;
pub use external_process::External;

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
}

impl LocationId {
    pub fn root(&self) -> &LocationId {
        match self {
            LocationId::Process(_) => self,
            LocationId::Cluster(_) => self,
            LocationId::Tick(_, id) => id.root(),
        }
    }

    pub fn is_root(&self) -> bool {
        match self {
            LocationId::Process(_) | LocationId::Cluster(_) => true,
            LocationId::Tick(_, _) => false,
        }
    }

    pub fn raw_id(&self) -> usize {
        match self {
            LocationId::Process(id) => *id,
            LocationId::Cluster(id) => *id,
            LocationId::Tick(_, _) => panic!("cannot get raw id for tick"),
        }
    }

    pub fn swap_root(&mut self, new_root: LocationId) {
        match self {
            LocationId::Tick(_, id) => {
                id.swap_root(new_root);
            }
            _ => {
                assert!(new_root.is_root());
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
            output_type: Some(quote_type::<T>().into()),
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
                    metadata: self.new_node_metadata::<T>(),
                }),
                metadata: self.new_node_metadata::<T>(),
            },
        )
    }

    fn source_external_bytes<L>(
        &self,
        from: &External<L>,
    ) -> (
        ExternalBytesPort,
        Stream<std::io::Result<BytesMut>, Self, Unbounded, TotalOrder, ExactlyOnce>,
    )
    where
        Self: Sized + NoTick,
    {
        let next_external_port_id = {
            let mut flow_state = from.flow_state.borrow_mut();
            let id = flow_state.next_external_out;
            flow_state.next_external_out += 1;
            id
        };

        (
            ExternalBytesPort {
                process_id: from.id,
                port_id: next_external_port_id,
                _phantom: Default::default()
            },
            Stream::new(
                self.clone(),
                HydroNode::Persist {
                    inner: Box::new(HydroNode::ExternalInput {
                        from_external_id: from.id,
                        from_key: next_external_port_id,
                        from_many: false,
                        instantiate_fn: DebugInstantiate::Building,
                        deserialize_fn: None,
                        metadata: self.new_node_metadata::<std::io::Result<BytesMut>>(),
                    }),
                    metadata: self.new_node_metadata::<std::io::Result<BytesMut>>(),
                },
            ),
        )
    }

    fn source_external_many_bytes<L>(
        &self,
        from: &External<L>,
    ) -> (
        ExternalBytesPort<Many>,
        Stream<std::io::Result<(u64, BytesMut)>, Self, Unbounded, NoOrder, ExactlyOnce>,
    )
    where
        Self: Sized + NoTick,
    {
        let next_external_port_id = {
            let mut flow_state = from.flow_state.borrow_mut();
            let id = flow_state.next_external_out;
            flow_state.next_external_out += 1;
            id
        };

        (
            ExternalBytesPort {
                process_id: from.id,
                port_id: next_external_port_id,
                _phantom: PhantomData,
            },
            Stream::new(
                self.clone(),
                HydroNode::Persist {
                    inner: Box::new(HydroNode::ExternalInput {
                        from_external_id: from.id,
                        from_key: next_external_port_id,
                        from_many: true,
                        instantiate_fn: DebugInstantiate::Building,
                        deserialize_fn: None,
                        metadata: self.new_node_metadata::<std::io::Result<(u64, BytesMut)>>(),
                    }),
                    metadata: self.new_node_metadata::<std::io::Result<(u64, BytesMut)>>(),
                },
            ),
        )
    }

    fn source_external_bincode<L, T>(
        &self,
        from: &External<L>,
    ) -> (
        ExternalBincodeSink<T>,
        Stream<T, Self, Unbounded, TotalOrder, ExactlyOnce>,
    )
    where
        Self: Sized + NoTick,
        T: Serialize + DeserializeOwned,
    {
        let next_external_port_id = {
            let mut flow_state = from.flow_state.borrow_mut();
            let id = flow_state.next_external_out;
            flow_state.next_external_out += 1;
            id
        };

        (
            ExternalBincodeSink {
                process_id: from.id,
                port_id: next_external_port_id,
                _phantom: PhantomData,
            },
            Stream::new(
                self.clone(),
                HydroNode::Persist {
                    inner: Box::new(HydroNode::ExternalInput {
                        from_external_id: from.id,
                        from_key: next_external_port_id,
                        from_many: false,
                        instantiate_fn: DebugInstantiate::Building,
                        deserialize_fn: Some(crate::stream::deserialize_bincode::<T>(None).into()),
                        metadata: self.new_node_metadata::<T>(),
                    }),
                    metadata: self.new_node_metadata::<T>(),
                },
            ),
        )
    }

    fn source_external_many_bincode<L, T>(
        &self,
        from: &External<L>,
    ) -> (
        ExternalBincodeSink<T, Many>,
        Stream<(u64, T), Self, Unbounded, NoOrder, ExactlyOnce>,
    )
    where
        Self: Sized + NoTick,
        T: Serialize + DeserializeOwned,
    {
        let next_external_port_id = {
            let mut flow_state = from.flow_state.borrow_mut();
            let id = flow_state.next_external_out;
            flow_state.next_external_out += 1;
            id
        };

        let root = get_this_crate();
        // let c_type = quote_type::<L>();
        let t_type = quote_type::<T>();

        let deser_fn: syn::Expr = syn::parse_quote! {
            |res| {
                let (id, b) = res.unwrap();
                (id, #root::runtime_support::bincode::deserialize::<#t_type>(&b).unwrap())
            }
        };

        (
            ExternalBincodeSink {
                process_id: from.id,
                port_id: next_external_port_id,
                _phantom: PhantomData,
            },
            Stream::new(
                self.clone(),
                HydroNode::Persist {
                    inner: Box::new(HydroNode::ExternalInput {
                        from_external_id: from.id,
                        from_key: next_external_port_id,
                        from_many: true,
                        instantiate_fn: DebugInstantiate::Building,
                        deserialize_fn: Some(deser_fn.into()),
                        metadata: self.new_node_metadata::<(u64, T)>(),
                    }),
                    metadata: self.new_node_metadata::<(u64, T)>(),
                },
            ),
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

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use futures::{SinkExt, StreamExt};
    use hydro_deploy::Deployment;
    use stageleft::q;

    use crate::{FlowBuilder, Location};

    #[tokio::test]
    async fn external_bytes() {
        let mut deployment = Deployment::new();

        let flow = FlowBuilder::new();
        let first_node = flow.process::<()>();
        let external = flow.external::<()>();

        let (in_port, input) = first_node.source_external_bytes(&external);
        let out = input.map(q!(|r| r.unwrap())).send_bincode_external(&external);

        let nodes = flow
            .with_process(&first_node, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut external_in = nodes.connect_sink_bytes(in_port).await;
        let mut external_out = nodes.connect_source_bincode(out).await;

        deployment.start().await.unwrap();

        external_in.send(vec![1, 2, 3].into()).await.unwrap();

        assert_eq!(
            external_out.next().await.unwrap(),
            vec![1, 2, 3]
        );
    }

    #[tokio::test]
    async fn multi_external_source() {
        let mut deployment = Deployment::new();

        let flow = FlowBuilder::new();
        let first_node = flow.process::<()>();
        let external = flow.external::<()>();

        let (in_port, input) = first_node.source_external_many_bincode(&external);
        let out = input.send_bincode_external(&external);

        let nodes = flow
            .with_process(&first_node, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut external_in_1 = nodes.connect_sink_bincode(in_port.clone()).await;
        let mut external_in_2 = nodes.connect_sink_bincode(in_port).await;
        let external_out = nodes.connect_source_bincode(out).await;

        deployment.start().await.unwrap();

        external_in_1.send(123).await.unwrap();
        external_in_2.send(456).await.unwrap();

        assert_eq!(
            external_out.take(2).collect::<HashSet<_>>().await,
            vec![(0, 123), (1, 456)].into_iter().collect()
        );
    }

    #[tokio::test]
    async fn second_connection_only_multi_source() {
        let mut deployment = Deployment::new();

        let flow = FlowBuilder::new();
        let first_node = flow.process::<()>();
        let external = flow.external::<()>();

        let (in_port, input) = first_node.source_external_many_bincode(&external);
        let out = input.send_bincode_external(&external);

        let nodes = flow
            .with_process(&first_node, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        // intentionally skipped to test stream waking logic
        let mut _external_in_1 = nodes.connect_sink_bincode(in_port.clone()).await;
        let mut external_in_2 = nodes.connect_sink_bincode(in_port).await;
        let mut external_out = nodes.connect_source_bincode(out).await;

        deployment.start().await.unwrap();

        external_in_2.send(456).await.unwrap();

        assert_eq!(external_out.next().await.unwrap(), (1, 456));
    }

    #[tokio::test]
    async fn multi_external_bytes() {
        let mut deployment = Deployment::new();

        let flow = FlowBuilder::new();
        let first_node = flow.process::<()>();
        let external = flow.external::<()>();

        let (in_port, input) = first_node.source_external_many_bytes(&external);
        let out = input.map(q!(|r| r.unwrap())).send_bincode_external(&external);

        let nodes = flow
            .with_process(&first_node, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut external_in_1 = nodes.connect_sink_bytes(in_port.clone()).await;
        let mut external_in_2 = nodes.connect_sink_bytes(in_port).await;
        let external_out = nodes.connect_source_bincode(out).await;

        deployment.start().await.unwrap();

        external_in_1.send(vec![1, 2, 3].into()).await.unwrap();
        external_in_2.send(vec![4, 5].into()).await.unwrap();

        assert_eq!(
            external_out.take(2).collect::<HashSet<_>>().await,
            vec![(0, (&[1u8, 2, 3] as &[u8]).into()), (1, (&[4u8, 5] as &[u8]).into())].into_iter().collect()
        );
    }
}
