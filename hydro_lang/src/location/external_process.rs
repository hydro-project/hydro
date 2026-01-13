use std::marker::PhantomData;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::compile::builder::{ExternalPortId, FlowState};
use crate::live_collections::stream::{ExactlyOnce, Ordering, Retries, TotalOrder};
use crate::staging_util::Invariant;

pub enum NotMany {}
pub enum Many {}

pub struct ExternalBytesPort<Many = NotMany> {
    pub(crate) process_id: usize,
    pub(crate) port_id: ExternalPortId,
    pub(crate) _phantom: PhantomData<Many>,
}

impl Clone for ExternalBytesPort<Many> {
    fn clone(&self) -> Self {
        Self {
            process_id: self.process_id,
            port_id: self.port_id,
            _phantom: Default::default(),
        }
    }
}

pub struct ExternalBincodeSink<
    Type,
    Many = NotMany,
    O: Ordering = TotalOrder,
    R: Retries = ExactlyOnce,
> where
    Type: Serialize,
{
    pub(crate) process_id: usize,
    pub(crate) port_id: ExternalPortId,
    pub(crate) _phantom: PhantomData<(Type, Many, O, R)>,
}

impl<T: Serialize, O: Ordering, R: Retries> Clone for ExternalBincodeSink<T, Many, O, R> {
    fn clone(&self) -> Self {
        Self {
            process_id: self.process_id,
            port_id: self.port_id,
            _phantom: Default::default(),
        }
    }
}

pub struct ExternalBincodeBidi<InType, OutType, Many = NotMany> {
    pub(crate) process_id: usize,
    pub(crate) port_id: ExternalPortId,
    pub(crate) _phantom: PhantomData<(InType, OutType, Many)>,
}

impl<InT, OutT> Clone for ExternalBincodeBidi<InT, OutT, Many> {
    fn clone(&self) -> Self {
        Self {
            process_id: self.process_id,
            port_id: self.port_id,
            _phantom: Default::default(),
        }
    }
}

pub struct ExternalBincodeStream<Type, O: Ordering = TotalOrder, R: Retries = ExactlyOnce>
where
    Type: DeserializeOwned,
{
    #[cfg_attr(
        not(feature = "build"),
        expect(unused, reason = "unused without feature")
    )]
    pub(crate) process_id: usize,
    #[cfg_attr(
        not(feature = "build"),
        expect(unused, reason = "unused without feature")
    )]
    pub(crate) port_id: ExternalPortId,
    pub(crate) _phantom: PhantomData<(Type, O, R)>,
}

pub struct External<'a, Tag> {
    pub(crate) id: usize,

    pub(crate) flow_state: FlowState,

    pub(crate) _phantom: Invariant<'a, Tag>,
}

impl<P> Clone for External<'_, P> {
    fn clone(&self) -> Self {
        External {
            id: self.id,
            flow_state: self.flow_state.clone(),
            _phantom: PhantomData,
        }
    }
}
