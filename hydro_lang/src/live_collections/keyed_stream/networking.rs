//! Networking APIs for [`KeyedStream`].

use serde::Serialize;
use serde::de::DeserializeOwned;
use stageleft::quote_type;

use super::KeyedStream;
use crate::compile::ir::{DebugInstantiate, HydroNode};
use crate::live_collections::boundedness::{Boundedness, Unbounded};
use crate::live_collections::stream::Stream;
use crate::live_collections::stream::networking::{deserialize_bincode, serialize_bincode};
#[cfg(stageleft_runtime)]
use crate::location::dynamic::DynLocation;
use crate::location::{Cluster, MemberId, Process};

#[expect(missing_docs, reason = "TODO")]
impl<'a, T, L, L2, B: Boundedness, O, R> KeyedStream<MemberId<L2>, T, Process<'a, L>, B, O, R> {
    pub fn demux_bincode(
        self,
        other: &Cluster<'a, L2>,
    ) -> Stream<T, Cluster<'a, L2>, Unbounded, O, R>
    where
        T: Serialize + DeserializeOwned,
    {
        let serialize_pipeline = Some(serialize_bincode::<T>(true));

        let deserialize_pipeline = Some(deserialize_bincode::<T>(None));

        Stream::new(
            other.clone(),
            HydroNode::Network {
                serialize_fn: serialize_pipeline.map(|e| e.into()),
                instantiate_fn: DebugInstantiate::Building,
                deserialize_fn: deserialize_pipeline.map(|e| e.into()),
                input: Box::new(self.underlying.ir_node.into_inner()),
                metadata: other.new_node_metadata::<T>(),
            },
        )
    }
}

#[expect(missing_docs, reason = "TODO")]
impl<'a, T, L, L2, B: Boundedness, O, R> KeyedStream<MemberId<L2>, T, Cluster<'a, L>, B, O, R> {
    pub fn demux_bincode(
        self,
        other: &Cluster<'a, L2>,
    ) -> KeyedStream<MemberId<L>, T, Cluster<'a, L2>, Unbounded, O, R>
    where
        T: Serialize + DeserializeOwned,
    {
        let serialize_pipeline = Some(serialize_bincode::<T>(true));

        let deserialize_pipeline = Some(deserialize_bincode::<T>(Some(&quote_type::<L>())));

        let raw_stream: Stream<(MemberId<L>, T), Cluster<'a, L2>, Unbounded, O, R> = Stream::new(
            other.clone(),
            HydroNode::Network {
                serialize_fn: serialize_pipeline.map(|e| e.into()),
                instantiate_fn: DebugInstantiate::Building,
                deserialize_fn: deserialize_pipeline.map(|e| e.into()),
                input: Box::new(self.underlying.ir_node.into_inner()),
                metadata: other.new_node_metadata::<(MemberId<L>, T)>(),
            },
        );

        raw_stream.into_keyed()
    }
}
