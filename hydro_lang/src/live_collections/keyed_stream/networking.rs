//! Networking APIs for [`KeyedStream`].

use serde::Serialize;
use serde::de::DeserializeOwned;
use stageleft::quote_type;

use super::KeyedStream;
use crate::compile::ir::{DebugInstantiate, HydroNode};
use crate::live_collections::boundedness::{Boundedness, Unbounded};
use crate::live_collections::stream::networking::{deserialize_bincode, serialize_bincode};
use crate::live_collections::stream::{Ordering, Retries, Stream};
#[cfg(stageleft_runtime)]
use crate::location::dynamic::DynLocation;
use crate::location::{Cluster, MemberId, Process};

impl<'a, T, L, L2, B: Boundedness, O: Ordering, R: Retries>
    KeyedStream<MemberId<L2>, T, Process<'a, L>, B, O, R>
{
    /// Sends each group of this stream to a specific member of a cluster, with the [`MemberId`] key
    /// identifying the recipient for each group and using [`bincode`] to serialize/deserialize messages.
    ///
    /// Each key must be a `MemberId<L2>` and each value must be a `T` where the key specifies
    /// which cluster member should receive the data. Unlike [`Stream::broadcast_bincode`], this
    /// API allows precise targeting of specific cluster members rather than broadcasting to
    /// all members.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, p2| {
    /// let p1 = flow.process::<()>();
    /// let workers: Cluster<()> = flow.cluster::<()>();
    /// let numbers: Stream<_, Process<_>, _> = p1.source_iter(q!(vec![0, 1, 2, 3]));
    /// let on_worker: Stream<_, Cluster<_>, _> = numbers
    ///     .map(q!(|x| (hydro_lang::location::MemberId::from_raw(x), x)))
    ///     .into_keyed()
    ///     .demux_bincode(&workers);
    /// # on_worker.send_bincode(&p2).entries()
    /// // if there are 4 members in the cluster, each receives one element
    /// // - MemberId::<()>(0): [0]
    /// // - MemberId::<()>(1): [1]
    /// // - MemberId::<()>(2): [2]
    /// // - MemberId::<()>(3): [3]
    /// # }, |mut stream| async move {
    /// # let mut results = Vec::new();
    /// # for w in 0..4 {
    /// #     results.push(format!("{:?}", stream.next().await.unwrap()));
    /// # }
    /// # results.sort();
    /// # assert_eq!(results, vec!["(MemberId::<()>(0), 0)", "(MemberId::<()>(1), 1)", "(MemberId::<()>(2), 2)", "(MemberId::<()>(3), 3)"]);
    /// # }));
    /// ```
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
                input: Box::new(self.ir_node.into_inner()),
                metadata: other.new_node_metadata(
                    Stream::<T, Cluster<'a, L2>, Unbounded, O, R>::collection_kind(),
                ),
            },
        )
    }
}

impl<'a, T, L, L2, B: Boundedness, O: Ordering, R: Retries>
    KeyedStream<MemberId<L2>, T, Cluster<'a, L>, B, O, R>
{
    /// Sends each group of this stream at each source member to a specific member of a destination
    /// cluster, with the [`MemberId`] key identifying the recipient for each group and using
    /// [`bincode`] to serialize/deserialize messages.
    ///
    /// Each key must be a `MemberId<L2>` and each value must be a `T` where the key specifies
    /// which cluster member should receive the data. Unlike [`Stream::broadcast_bincode`], this
    /// API allows precise targeting of specific cluster members rather than broadcasting to all
    /// members.
    ///
    /// Each cluster member sends its local stream elements, and they are collected at each
    /// destination member as a [`KeyedStream`] where keys identify the source cluster member.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, p2| {
    /// # type Source = ();
    /// # type Destination = ();
    /// let source: Cluster<Source> = flow.cluster::<Source>();
    /// let to_send: KeyedStream<_, _, Cluster<_>, _> = source
    ///     .source_iter(q!(vec![0, 1, 2, 3]))
    ///     .map(q!(|x| (hydro_lang::location::MemberId::from_raw(x), x)))
    ///     .into_keyed();
    /// let destination: Cluster<Destination> = flow.cluster::<Destination>();
    /// let all_received = to_send.demux_bincode(&destination); // KeyedStream<MemberId<Source>, i32, ...>
    /// # all_received.entries().send_bincode(&p2).entries()
    /// # }, |mut stream| async move {
    /// // if there are 4 members in the destination cluster, each receives one message from each source member
    /// // - Destination(0): { Source(0): [0], Source(1): [0], ... }
    /// // - Destination(1): { Source(0): [1], Source(1): [1], ... }
    /// // - ...
    /// # let mut results = Vec::new();
    /// # for w in 0..16 {
    /// #     results.push(format!("{:?}", stream.next().await.unwrap()));
    /// # }
    /// # results.sort();
    /// # assert_eq!(results, vec![
    /// #   "(MemberId::<()>(0), (MemberId::<()>(0), 0))", "(MemberId::<()>(0), (MemberId::<()>(1), 0))", "(MemberId::<()>(0), (MemberId::<()>(2), 0))", "(MemberId::<()>(0), (MemberId::<()>(3), 0))",
    /// #   "(MemberId::<()>(1), (MemberId::<()>(0), 1))", "(MemberId::<()>(1), (MemberId::<()>(1), 1))", "(MemberId::<()>(1), (MemberId::<()>(2), 1))", "(MemberId::<()>(1), (MemberId::<()>(3), 1))",
    /// #   "(MemberId::<()>(2), (MemberId::<()>(0), 2))", "(MemberId::<()>(2), (MemberId::<()>(1), 2))", "(MemberId::<()>(2), (MemberId::<()>(2), 2))", "(MemberId::<()>(2), (MemberId::<()>(3), 2))",
    /// #   "(MemberId::<()>(3), (MemberId::<()>(0), 3))", "(MemberId::<()>(3), (MemberId::<()>(1), 3))", "(MemberId::<()>(3), (MemberId::<()>(2), 3))", "(MemberId::<()>(3), (MemberId::<()>(3), 3))"
    /// # ]);
    /// # }));
    /// ```
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
                input: Box::new(self.ir_node.into_inner()),
                metadata: other.new_node_metadata(Stream::<
                    (MemberId<L>, T),
                    Cluster<'a, L2>,
                    Unbounded,
                    O,
                    R,
                >::collection_kind()),
            },
        );

        raw_stream.into_keyed()
    }
}

// impl<'a, T, L, L2, B: Boundedness, O: Ordering, R: Retries>
//     KeyedStream<MemberId<L2>, T, Process<'a, L>, B, O, R>
// {
//     /// Sends each group of this stream to a specific member of a cluster, with the [`MemberId`] key
//     /// identifying the recipient for each group and using [`bincode`] to serialize/deserialize messages.
//     ///
//     /// Each key must be a `MemberId<L2>` and each value must be a `T` where the key specifies
//     /// which cluster member should receive the data. Unlike [`Stream::broadcast_bincode`], this
//     /// API allows precise targeting of specific cluster members rather than broadcasting to
//     /// all members.
//     ///
//     /// # Example
//     /// ```rust
//     /// # use hydro_lang::prelude::*;
//     /// # use futures::StreamExt;
//     /// # tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, p2| {
//     /// let p1 = flow.process::<()>();
//     /// let workers: Cluster<()> = flow.cluster::<()>();
//     /// let numbers: Stream<_, Process<_>, _> = p1.source_iter(q!(vec![0, 1, 2, 3]));
//     /// let on_worker: Stream<_, Cluster<_>, _> = numbers
//     ///     .map(q!(|x| (hydro_lang::location::MemberId::from_raw(x), x)))
//     ///     .into_keyed()
//     ///     .demux_bincode(&workers);
//     /// # on_worker.send_bincode(&p2).entries()
//     /// // if there are 4 members in the cluster, each receives one element
//     /// // - MemberId::<()>(0): [0]
//     /// // - MemberId::<()>(1): [1]
//     /// // - MemberId::<()>(2): [2]
//     /// // - MemberId::<()>(3): [3]
//     /// # }, |mut stream| async move {
//     /// # let mut results = Vec::new();
//     /// # for w in 0..4 {
//     /// #     results.push(format!("{:?}", stream.next().await.unwrap()));
//     /// # }
//     /// # results.sort();
//     /// # assert_eq!(results, vec!["(MemberId::<()>(0), 0)", "(MemberId::<()>(1), 1)", "(MemberId::<()>(2), 2)", "(MemberId::<()>(3), 3)"]);
//     /// # }));
//     /// ```
//     pub fn demux_bincode(
//         self,
//         other: &Cluster<'a, L2>,
//     ) -> Stream<T, Cluster<'a, L2>, Unbounded, O, R>
//     where
//         T: Serialize + DeserializeOwned,
//     {
//         let serialize_pipeline = Some(serialize_bincode::<T>(true));

//         let deserialize_pipeline = Some(deserialize_bincode::<T>(None));

//         Stream::new(
//             other.clone(),
//             HydroNode::Network {
//                 serialize_fn: serialize_pipeline.map(|e| e.into()),
//                 instantiate_fn: DebugInstantiate::Building,
//                 deserialize_fn: deserialize_pipeline.map(|e| e.into()),
//                 input: Box::new(self.ir_node.into_inner()),
//                 metadata: other.new_node_metadata(
//                     Stream::<T, Cluster<'a, L2>, Unbounded, O, R>::collection_kind(),
//                 ),
//             },
//         )
//     }
// }

// impl<'a, T, L, L2, B: Boundedness, O: Ordering, R: Retries>
//     KeyedStream<MemberId<L2>, T, Cluster<'a, L>, B, O, R>
// {
//     /// Sends each group of this stream at each source member to a specific member of a destination
//     /// cluster, with the [`MemberId`] key identifying the recipient for each group and using
//     /// [`bincode`] to serialize/deserialize messages.
//     ///
//     /// Each key must be a `MemberId<L2>` and each value must be a `T` where the key specifies
//     /// which cluster member should receive the data. Unlike [`Stream::broadcast_bincode`], this
//     /// API allows precise targeting of specific cluster members rather than broadcasting to all
//     /// members.
//     ///
//     /// Each cluster member sends its local stream elements, and they are collected at each
//     /// destination member as a [`KeyedStream`] where keys identify the source cluster member.
//     ///
//     /// # Example
//     /// ```rust
//     /// # use hydro_lang::prelude::*;
//     /// # use futures::StreamExt;
//     /// # tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, p2| {
//     /// # type Source = ();
//     /// # type Destination = ();
//     /// let source: Cluster<Source> = flow.cluster::<Source>();
//     /// let to_send: KeyedStream<_, _, Cluster<_>, _> = source
//     ///     .source_iter(q!(vec![0, 1, 2, 3]))
//     ///     .map(q!(|x| (hydro_lang::location::MemberId::from_raw(x), x)))
//     ///     .into_keyed();
//     /// let destination: Cluster<Destination> = flow.cluster::<Destination>();
//     /// let all_received = to_send.demux_bincode(&destination); // KeyedStream<MemberId<Source>, i32, ...>
//     /// # all_received.entries().send_bincode(&p2).entries()
//     /// # }, |mut stream| async move {
//     /// // if there are 4 members in the destination cluster, each receives one message from each source member
//     /// // - Destination(0): { Source(0): [0], Source(1): [0], ... }
//     /// // - Destination(1): { Source(0): [1], Source(1): [1], ... }
//     /// // - ...
//     /// # let mut results = Vec::new();
//     /// # for w in 0..16 {
//     /// #     results.push(format!("{:?}", stream.next().await.unwrap()));
//     /// # }
//     /// # results.sort();
//     /// # assert_eq!(results, vec![
//     /// #   "(MemberId::<()>(0), (MemberId::<()>(0), 0))", "(MemberId::<()>(0), (MemberId::<()>(1), 0))", "(MemberId::<()>(0), (MemberId::<()>(2), 0))", "(MemberId::<()>(0), (MemberId::<()>(3), 0))",
//     /// #   "(MemberId::<()>(1), (MemberId::<()>(0), 1))", "(MemberId::<()>(1), (MemberId::<()>(1), 1))", "(MemberId::<()>(1), (MemberId::<()>(2), 1))", "(MemberId::<()>(1), (MemberId::<()>(3), 1))",
//     /// #   "(MemberId::<()>(2), (MemberId::<()>(0), 2))", "(MemberId::<()>(2), (MemberId::<()>(1), 2))", "(MemberId::<()>(2), (MemberId::<()>(2), 2))", "(MemberId::<()>(2), (MemberId::<()>(3), 2))",
//     /// #   "(MemberId::<()>(3), (MemberId::<()>(0), 3))", "(MemberId::<()>(3), (MemberId::<()>(1), 3))", "(MemberId::<()>(3), (MemberId::<()>(2), 3))", "(MemberId::<()>(3), (MemberId::<()>(3), 3))"
//     /// # ]);
//     /// # }));
//     /// ```
//     pub fn demux_bincode(
//         self,
//         other: &Cluster<'a, L2>,
//     ) -> KeyedStream<MemberId<L>, T, Cluster<'a, L2>, Unbounded, O, R>
//     where
//         T: Serialize + DeserializeOwned,
//     {
//         let serialize_pipeline = Some(serialize_bincode::<T>(true));

//         let deserialize_pipeline = Some(deserialize_bincode::<T>(Some(&quote_type::<L>())));

//         let raw_stream: Stream<(MemberId<L>, T), Cluster<'a, L2>, Unbounded, O, R> = Stream::new(
//             other.clone(),
//             HydroNode::Network {
//                 serialize_fn: serialize_pipeline.map(|e| e.into()),
//                 instantiate_fn: DebugInstantiate::Building,
//                 deserialize_fn: deserialize_pipeline.map(|e| e.into()),
//                 input: Box::new(self.ir_node.into_inner()),
//                 metadata: other.new_node_metadata(Stream::<
//                     (MemberId<L>, T),
//                     Cluster<'a, L2>,
//                     Unbounded,
//                     O,
//                     R,
//                 >::collection_kind()),
//             },
//         );

//         raw_stream.into_keyed()
//     }
// }
