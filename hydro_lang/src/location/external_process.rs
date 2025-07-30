use std::marker::PhantomData;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::builder::FlowState;
use crate::staging_util::Invariant;

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

pub struct ExternalBincodeSink<Type>
where
    Type: Serialize,
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
