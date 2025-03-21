use std::marker::PhantomData;

use bytes::Bytes;
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::{Location, LocationId, NoTick};
use crate::builder::FlowState;
use crate::ir::{DebugInstantiate, HydroNode, HydroSource};
use crate::staging_util::Invariant;
use crate::{Stream, Unbounded};

pub struct ExternalBytesPort {
    #[cfg_attr(
        not(feature = "build"),
        expect(unused, reason = "unused without feature")
    )]
    pub(crate) process_id: usize,
    #[cfg_attr(
        not(feature = "build"),
        expect(unused, reason = "unused without feature")
    )]
    pub(crate) port_id: usize,
}

pub struct ExternalBincodeSink<T: Serialize> {
    #[cfg_attr(
        not(feature = "build"),
        expect(unused, reason = "unused without feature")
    )]
    pub(crate) process_id: usize,
    #[cfg_attr(
        not(feature = "build"),
        expect(unused, reason = "unused without feature")
    )]
    pub(crate) port_id: usize,
    pub(crate) _phantom: PhantomData<T>,
}

pub struct ExternalBincodeStream<T: DeserializeOwned> {
    #[cfg_attr(
        not(feature = "build"),
        expect(unused, reason = "unused without feature")
    )]
    pub(crate) process_id: usize,
    #[cfg_attr(
        not(feature = "build"),
        expect(unused, reason = "unused without feature")
    )]
    pub(crate) port_id: usize,
    pub(crate) _phantom: PhantomData<T>,
}

pub struct ExternalProcess<'a, P> {
    pub(crate) id: usize,

    pub(crate) flow_state: FlowState,

    pub(crate) _phantom: Invariant<'a, P>,
}

impl<P> Clone for ExternalProcess<'_, P> {
    fn clone(&self) -> Self {
        ExternalProcess {
            id: self.id,
            flow_state: self.flow_state.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<'a, P> Location<'a> for ExternalProcess<'a, P> {
    type Root = Self;

    fn root(&self) -> Self::Root {
        self.clone()
    }

    fn id(&self) -> LocationId {
        LocationId::ExternalProcess(self.id)
    }

    fn flow_state(&self) -> &FlowState {
        &self.flow_state
    }

    fn is_top_level() -> bool {
        true
    }
}

impl<'a, P> ExternalProcess<'a, P> {
    pub fn source_external_bytes<L: Location<'a> + NoTick>(
        &self,
        to: &L,
    ) -> (ExternalBytesPort, Stream<Bytes, L, Unbounded>) {
        let next_external_port_id = {
            let mut flow_state = self.flow_state.borrow_mut();
            let id = flow_state.next_external_out;
            flow_state.next_external_out += 1;
            id
        };

        let deser_expr: syn::Expr = syn::parse_quote!(|b| b.unwrap().freeze());

        (
            ExternalBytesPort {
                process_id: self.id,
                port_id: next_external_port_id,
            },
            Stream::new(
                to.clone(),
                HydroNode::Persist {
                    inner: Box::new(HydroNode::Network {
                        from_key: Some(next_external_port_id),
                        to_location: to.id(),
                        to_key: None,
                        serialize_fn: None,
                        instantiate_fn: DebugInstantiate::Building,
                        deserialize_fn: Some(deser_expr.into()),
                        input: Box::new(HydroNode::Source {
                            source: HydroSource::ExternalNetwork(),
                            location_kind: LocationId::ExternalProcess(self.id),
                            metadata: self.new_node_metadata::<Bytes>(),
                        }),
                        metadata: to.new_node_metadata::<Bytes>(),
                    }),
                    metadata: to.new_node_metadata::<Bytes>(),
                },
            ),
        )
    }

    pub fn source_external_bincode<L: Location<'a> + NoTick, T: Serialize + DeserializeOwned>(
        &self,
        to: &L,
    ) -> (ExternalBincodeSink<T>, Stream<T, L, Unbounded>) {
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
                to.clone(),
                HydroNode::Persist {
                    inner: Box::new(HydroNode::Network {
                        from_key: Some(next_external_port_id),
                        to_location: to.id(),
                        to_key: None,
                        serialize_fn: None,
                        instantiate_fn: DebugInstantiate::Building,
                        deserialize_fn: Some(crate::stream::deserialize_bincode::<T>(None).into()),
                        input: Box::new(HydroNode::Source {
                            source: HydroSource::ExternalNetwork(),
                            location_kind: LocationId::ExternalProcess(self.id),
                            metadata: self.new_node_metadata::<T>(),
                        }),
                        metadata: to.new_node_metadata::<T>(),
                    }),
                    metadata: to.new_node_metadata::<T>(),
                },
            ),
        )
    }
}
