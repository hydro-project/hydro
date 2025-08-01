use std::marker::PhantomData;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::builder::FlowState;
use crate::staging_util::Invariant;

pub enum NotMany {}
pub enum Many {}

pub struct ExternalBytesPort<Many = NotMany> {
    pub(crate) process_id: usize,
    pub(crate) port_id: usize,
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

pub struct ExternalBincodeSink<Type, Many = NotMany>
where
    Type: Serialize,
{
    pub(crate) process_id: usize,
    pub(crate) port_id: usize,
    pub(crate) _phantom: PhantomData<(Type, Many)>,
}

impl<T: Serialize> Clone for ExternalBincodeSink<T, Many> {
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
    pub(crate) port_id: usize,
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

pub struct ExternalBincodeStream<Type>
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
    pub(crate) port_id: usize,
    pub(crate) _phantom: PhantomData<Type>,
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
