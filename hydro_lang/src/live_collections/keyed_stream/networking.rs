//! Networking APIs for [`KeyedStream`].

use serde::Serialize;
use serde::de::DeserializeOwned;
use stageleft::{q, quote_type};

use super::KeyedStream;
use crate::compile::ir::{DebugInstantiate, HydroNode};
use crate::live_collections::boundedness::{Boundedness, Unbounded};
use crate::live_collections::stream::{Ordering, Retries, Stream};
#[cfg(stageleft_runtime)]
use crate::location::dynamic::DynLocation;
use crate::location::{Cluster, MemberId, Process};
use crate::networking::{NetworkFor, TCP};

impl<'a, T, L, L2, B: Boundedness, O: Ordering, R: Retries>
    KeyedStream<MemberId<L2>, T, Process<'a, L>, B, O, R>
{
    #[deprecated = "use KeyedStream::demux(..., TCP.fail_stop().bincode()) instead"]
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
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, p2| {
    /// let p1 = flow.process::<()>();
    /// let workers: Cluster<()> = flow.cluster::<()>();
    /// let numbers: Stream<_, Process<_>, _> = p1.source_iter(q!(vec![0, 1, 2, 3]));
    /// let on_worker: Stream<_, Cluster<_>, _> = numbers
    ///     .map(q!(|x| (hydro_lang::location::MemberId::from_raw_id(x), x)))
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
    /// # }
    /// ```
    pub fn demux_bincode(
        self,
        other: &Cluster<'a, L2>,
    ) -> Stream<T, Cluster<'a, L2>, Unbounded, O, R>
    where
        T: Serialize + DeserializeOwned,
    {
        self.demux(other, TCP.fail_stop().bincode())
    }

    /// Sends each group of this stream to a specific member of a cluster, with the [`MemberId`] key
    /// identifying the recipient for each group and using the configuration in `via` to set up the
    /// message transport.
    ///
    /// Each key must be a `MemberId<L2>` and each value must be a `T` where the key specifies
    /// which cluster member should receive the data. Unlike [`Stream::broadcast`], this
    /// API allows precise targeting of specific cluster members rather than broadcasting to
    /// all members.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, p2| {
    /// let p1 = flow.process::<()>();
    /// let workers: Cluster<()> = flow.cluster::<()>();
    /// let numbers: Stream<_, Process<_>, _> = p1.source_iter(q!(vec![0, 1, 2, 3]));
    /// let on_worker: Stream<_, Cluster<_>, _> = numbers
    ///     .map(q!(|x| (hydro_lang::location::MemberId::from_raw_id(x), x)))
    ///     .into_keyed()
    ///     .demux(&workers, TCP.fail_stop().bincode());
    /// # on_worker.send(&p2, TCP.fail_stop().bincode()).entries()
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
    /// # }
    /// ```
    pub fn demux<N: NetworkFor<T>>(
        self,
        to: &Cluster<'a, L2>,
        via: N,
    ) -> Stream<T, Cluster<'a, L2>, Unbounded, O, R>
    where
        T: Serialize + DeserializeOwned,
    {
        let serialize_pipeline = Some(N::serialize_thunk(true));

        let deserialize_pipeline = Some(N::deserialize_thunk(None));

        let name = via.name();
        if to.multiversioned() && name.is_none() {
            panic!(
                "Cannot send to a multiversioned location without a channel name. Please provide a name for the network."
            );
        }

        Stream::new(
            to.clone(),
            HydroNode::Network {
                name: name.map(ToOwned::to_owned),
                serialize_fn: serialize_pipeline.map(|e| e.into()),
                instantiate_fn: DebugInstantiate::Building,
                deserialize_fn: deserialize_pipeline.map(|e| e.into()),
                input: Box::new(self.ir_node.into_inner()),
                metadata: to.new_node_metadata(
                    Stream::<T, Cluster<'a, L2>, Unbounded, O, R>::collection_kind(),
                ),
            },
        )
    }
}

impl<'a, K, T, L, L2, B: Boundedness, O: Ordering, R: Retries>
    KeyedStream<(MemberId<L2>, K), T, Process<'a, L>, B, O, R>
{
    #[deprecated = "use KeyedStream::demux(..., TCP.fail_stop().bincode()) instead"]
    /// Sends each group of this stream to a specific member of a cluster. The input stream has a
    /// compound key where the first element is the recipient's [`MemberId`] and the second element
    /// is a key that will be sent along with the value, using [`bincode`] to serialize/deserialize
    /// messages.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, p2| {
    /// let p1 = flow.process::<()>();
    /// let workers: Cluster<()> = flow.cluster::<()>();
    /// let to_send: KeyedStream<_, _, Process<_>, _> = p1
    ///     .source_iter(q!(vec![0, 1, 2, 3]))
    ///     .map(q!(|x| ((hydro_lang::location::MemberId::from_raw_id(x), x), x + 123)))
    ///     .into_keyed();
    /// let on_worker: KeyedStream<_, _, Cluster<_>, _> = to_send.demux_bincode(&workers);
    /// # on_worker.entries().send_bincode(&p2).entries()
    /// // if there are 4 members in the cluster, each receives one element
    /// // - MemberId::<()>(0): { 0: [123] }
    /// // - MemberId::<()>(1): { 1: [124] }
    /// // - ...
    /// # }, |mut stream| async move {
    /// # let mut results = Vec::new();
    /// # for w in 0..4 {
    /// #     results.push(format!("{:?}", stream.next().await.unwrap()));
    /// # }
    /// # results.sort();
    /// # assert_eq!(results, vec!["(MemberId::<()>(0), (0, 123))", "(MemberId::<()>(1), (1, 124))", "(MemberId::<()>(2), (2, 125))", "(MemberId::<()>(3), (3, 126))"]);
    /// # }));
    /// # }
    /// ```
    pub fn demux_bincode(
        self,
        other: &Cluster<'a, L2>,
    ) -> KeyedStream<K, T, Cluster<'a, L2>, Unbounded, O, R>
    where
        K: Serialize + DeserializeOwned,
        T: Serialize + DeserializeOwned,
    {
        self.demux(other, TCP.fail_stop().bincode())
    }

    /// Sends each group of this stream to a specific member of a cluster. The input stream has a
    /// compound key where the first element is the recipient's [`MemberId`] and the second element
    /// is a key that will be sent along with the value, using the configuration in `via` to set up
    /// the message transport.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, p2| {
    /// let p1 = flow.process::<()>();
    /// let workers: Cluster<()> = flow.cluster::<()>();
    /// let to_send: KeyedStream<_, _, Process<_>, _> = p1
    ///     .source_iter(q!(vec![0, 1, 2, 3]))
    ///     .map(q!(|x| ((hydro_lang::location::MemberId::from_raw_id(x), x), x + 123)))
    ///     .into_keyed();
    /// let on_worker: KeyedStream<_, _, Cluster<_>, _> = to_send.demux(&workers, TCP.fail_stop().bincode());
    /// # on_worker.entries().send(&p2, TCP.fail_stop().bincode()).entries()
    /// // if there are 4 members in the cluster, each receives one element
    /// // - MemberId::<()>(0): { 0: [123] }
    /// // - MemberId::<()>(1): { 1: [124] }
    /// // - ...
    /// # }, |mut stream| async move {
    /// # let mut results = Vec::new();
    /// # for w in 0..4 {
    /// #     results.push(format!("{:?}", stream.next().await.unwrap()));
    /// # }
    /// # results.sort();
    /// # assert_eq!(results, vec!["(MemberId::<()>(0), (0, 123))", "(MemberId::<()>(1), (1, 124))", "(MemberId::<()>(2), (2, 125))", "(MemberId::<()>(3), (3, 126))"]);
    /// # }));
    /// # }
    /// ```
    pub fn demux<N: NetworkFor<(K, T)>>(
        self,
        to: &Cluster<'a, L2>,
        via: N,
    ) -> KeyedStream<K, T, Cluster<'a, L2>, Unbounded, O, R>
    where
        K: Serialize + DeserializeOwned,
        T: Serialize + DeserializeOwned,
    {
        let serialize_pipeline = Some(N::serialize_thunk(true));

        let deserialize_pipeline = Some(N::deserialize_thunk(None));

        let name = via.name();
        if to.multiversioned() && name.is_none() {
            panic!(
                "Cannot send to a multiversioned location without a channel name. Please provide a name for the network."
            );
        }

        KeyedStream::new(
            to.clone(),
            HydroNode::Network {
                name: name.map(ToOwned::to_owned),
                serialize_fn: serialize_pipeline.map(|e| e.into()),
                instantiate_fn: DebugInstantiate::Building,
                deserialize_fn: deserialize_pipeline.map(|e| e.into()),
                input: Box::new(
                    self.entries()
                        .map(q!(|((id, k), v)| (id, (k, v))))
                        .ir_node
                        .into_inner(),
                ),
                metadata: to.new_node_metadata(
                    KeyedStream::<K, T, Cluster<'a, L2>, Unbounded, O, R>::collection_kind(),
                ),
            },
        )
    }
}

impl<'a, T, L, L2, B: Boundedness, O: Ordering, R: Retries>
    KeyedStream<MemberId<L2>, T, Cluster<'a, L>, B, O, R>
{
    #[deprecated = "use KeyedStream::demux(..., TCP.fail_stop().bincode()) instead"]
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
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, p2| {
    /// # type Source = ();
    /// # type Destination = ();
    /// let source: Cluster<Source> = flow.cluster::<Source>();
    /// let to_send: KeyedStream<_, _, Cluster<_>, _> = source
    ///     .source_iter(q!(vec![0, 1, 2, 3]))
    ///     .map(q!(|x| (hydro_lang::location::MemberId::from_raw_id(x), x)))
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
    /// # }
    /// ```
    pub fn demux_bincode(
        self,
        other: &Cluster<'a, L2>,
    ) -> KeyedStream<MemberId<L>, T, Cluster<'a, L2>, Unbounded, O, R>
    where
        T: Serialize + DeserializeOwned,
    {
        self.demux(other, TCP.fail_stop().bincode())
    }

    /// Sends each group of this stream at each source member to a specific member of a destination
    /// cluster, with the [`MemberId`] key identifying the recipient for each group and using the
    /// configuration in `via` to set up the message transport.
    ///
    /// Each key must be a `MemberId<L2>` and each value must be a `T` where the key specifies
    /// which cluster member should receive the data. Unlike [`Stream::broadcast`], this
    /// API allows precise targeting of specific cluster members rather than broadcasting to all
    /// members.
    ///
    /// Each cluster member sends its local stream elements, and they are collected at each
    /// destination member as a [`KeyedStream`] where keys identify the source cluster member.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, p2| {
    /// # type Source = ();
    /// # type Destination = ();
    /// let source: Cluster<Source> = flow.cluster::<Source>();
    /// let to_send: KeyedStream<_, _, Cluster<_>, _> = source
    ///     .source_iter(q!(vec![0, 1, 2, 3]))
    ///     .map(q!(|x| (hydro_lang::location::MemberId::from_raw_id(x), x)))
    ///     .into_keyed();
    /// let destination: Cluster<Destination> = flow.cluster::<Destination>();
    /// let all_received = to_send.demux(&destination, TCP.fail_stop().bincode()); // KeyedStream<MemberId<Source>, i32, ...>
    /// # all_received.entries().send(&p2, TCP.fail_stop().bincode()).entries()
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
    /// # }
    /// ```
    pub fn demux<N: NetworkFor<T>>(
        self,
        to: &Cluster<'a, L2>,
        via: N,
    ) -> KeyedStream<MemberId<L>, T, Cluster<'a, L2>, Unbounded, O, R>
    where
        T: Serialize + DeserializeOwned,
    {
        let serialize_pipeline = Some(N::serialize_thunk(true));

        let deserialize_pipeline = Some(N::deserialize_thunk(Some(&quote_type::<L>())));

        let name = via.name();
        if to.multiversioned() && name.is_none() {
            panic!(
                "Cannot send to a multiversioned location without a channel name. Please provide a name for the network."
            );
        }

        let raw_stream: Stream<(MemberId<L>, T), Cluster<'a, L2>, Unbounded, O, R> = Stream::new(
            to.clone(),
            HydroNode::Network {
                name: name.map(ToOwned::to_owned),
                serialize_fn: serialize_pipeline.map(|e| e.into()),
                instantiate_fn: DebugInstantiate::Building,
                deserialize_fn: deserialize_pipeline.map(|e| e.into()),
                input: Box::new(self.ir_node.into_inner()),
                metadata: to.new_node_metadata(Stream::<
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

impl<'a, K, V, L, B: Boundedness, O: Ordering, R: Retries>
    KeyedStream<K, V, Cluster<'a, L>, B, O, R>
{
    #[expect(clippy::type_complexity, reason = "compound key types with ordering")]
    #[deprecated = "use KeyedStream::send(..., TCP.fail_stop().bincode()) instead"]
    /// "Moves" elements of this keyed stream from a cluster to a process by sending them over the
    /// network, using [`bincode`] to serialize/deserialize messages. The resulting [`KeyedStream`]
    /// has a compound key where the first element is the sender's [`MemberId`] and the second
    /// element is the original key.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, p2| {
    /// # type Source = ();
    /// # type Destination = ();
    /// let source: Cluster<Source> = flow.cluster::<Source>();
    /// let to_send: KeyedStream<_, _, Cluster<_>, _> = source
    ///     .source_iter(q!(vec![0, 1, 2, 3]))
    ///     .map(q!(|x| (x, x + 123)))
    ///     .into_keyed();
    /// let destination_process = flow.process::<Destination>();
    /// let all_received = to_send.send_bincode(&destination_process); // KeyedStream<(MemberId<Source>, i32), i32, ...>
    /// # all_received.entries().send_bincode(&p2)
    /// # }, |mut stream| async move {
    /// // if there are 4 members in the source cluster, the destination process receives four messages from each source member
    /// // {
    /// //     (MemberId<Source>(0), 0): [123], (MemberId<Source>(1), 0): [123], ...,
    /// //     (MemberId<Source>(0), 1): [124], (MemberId<Source>(1), 1): [124], ...,
    /// //     ...
    /// // }
    /// # let mut results = Vec::new();
    /// # for w in 0..16 {
    /// #     results.push(format!("{:?}", stream.next().await.unwrap()));
    /// # }
    /// # results.sort();
    /// # assert_eq!(results, vec![
    /// #   "((MemberId::<()>(0), 0), 123)",
    /// #   "((MemberId::<()>(0), 1), 124)",
    /// #   "((MemberId::<()>(0), 2), 125)",
    /// #   "((MemberId::<()>(0), 3), 126)",
    /// #   "((MemberId::<()>(1), 0), 123)",
    /// #   "((MemberId::<()>(1), 1), 124)",
    /// #   "((MemberId::<()>(1), 2), 125)",
    /// #   "((MemberId::<()>(1), 3), 126)",
    /// #   "((MemberId::<()>(2), 0), 123)",
    /// #   "((MemberId::<()>(2), 1), 124)",
    /// #   "((MemberId::<()>(2), 2), 125)",
    /// #   "((MemberId::<()>(2), 3), 126)",
    /// #   "((MemberId::<()>(3), 0), 123)",
    /// #   "((MemberId::<()>(3), 1), 124)",
    /// #   "((MemberId::<()>(3), 2), 125)",
    /// #   "((MemberId::<()>(3), 3), 126)",
    /// # ]);
    /// # }));
    /// # }
    /// ```
    pub fn send_bincode<L2>(
        self,
        other: &Process<'a, L2>,
    ) -> KeyedStream<(MemberId<L>, K), V, Process<'a, L2>, Unbounded, O, R>
    where
        K: Serialize + DeserializeOwned,
        V: Serialize + DeserializeOwned,
    {
        self.send(other, TCP.fail_stop().bincode())
    }

    #[expect(clippy::type_complexity, reason = "compound key types with ordering")]
    /// "Moves" elements of this keyed stream from a cluster to a process by sending them over the
    /// network, using the configuration in `via` to set up the message transport. The resulting
    /// [`KeyedStream`] has a compound key where the first element is the sender's [`MemberId`] and
    /// the second element is the original key.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, p2| {
    /// # type Source = ();
    /// # type Destination = ();
    /// let source: Cluster<Source> = flow.cluster::<Source>();
    /// let to_send: KeyedStream<_, _, Cluster<_>, _> = source
    ///     .source_iter(q!(vec![0, 1, 2, 3]))
    ///     .map(q!(|x| (x, x + 123)))
    ///     .into_keyed();
    /// let destination_process = flow.process::<Destination>();
    /// let all_received = to_send.send(&destination_process, TCP.fail_stop().bincode()); // KeyedStream<(MemberId<Source>, i32), i32, ...>
    /// # all_received.entries().send(&p2, TCP.fail_stop().bincode())
    /// # }, |mut stream| async move {
    /// // if there are 4 members in the source cluster, the destination process receives four messages from each source member
    /// // {
    /// //     (MemberId<Source>(0), 0): [123], (MemberId<Source>(1), 0): [123], ...,
    /// //     (MemberId<Source>(0), 1): [124], (MemberId<Source>(1), 1): [124], ...,
    /// //     ...
    /// // }
    /// # let mut results = Vec::new();
    /// # for w in 0..16 {
    /// #     results.push(format!("{:?}", stream.next().await.unwrap()));
    /// # }
    /// # results.sort();
    /// # assert_eq!(results, vec![
    /// #   "((MemberId::<()>(0), 0), 123)",
    /// #   "((MemberId::<()>(0), 1), 124)",
    /// #   "((MemberId::<()>(0), 2), 125)",
    /// #   "((MemberId::<()>(0), 3), 126)",
    /// #   "((MemberId::<()>(1), 0), 123)",
    /// #   "((MemberId::<()>(1), 1), 124)",
    /// #   "((MemberId::<()>(1), 2), 125)",
    /// #   "((MemberId::<()>(1), 3), 126)",
    /// #   "((MemberId::<()>(2), 0), 123)",
    /// #   "((MemberId::<()>(2), 1), 124)",
    /// #   "((MemberId::<()>(2), 2), 125)",
    /// #   "((MemberId::<()>(2), 3), 126)",
    /// #   "((MemberId::<()>(3), 0), 123)",
    /// #   "((MemberId::<()>(3), 1), 124)",
    /// #   "((MemberId::<()>(3), 2), 125)",
    /// #   "((MemberId::<()>(3), 3), 126)",
    /// # ]);
    /// # }));
    /// # }
    /// ```
    pub fn send<L2, N: NetworkFor<(K, V)>>(
        self,
        to: &Process<'a, L2>,
        via: N,
    ) -> KeyedStream<(MemberId<L>, K), V, Process<'a, L2>, Unbounded, O, R>
    where
        K: Serialize + DeserializeOwned,
        V: Serialize + DeserializeOwned,
    {
        let serialize_pipeline = Some(N::serialize_thunk(false));

        let deserialize_pipeline = Some(N::deserialize_thunk(Some(&quote_type::<L>())));

        let name = via.name();
        if to.multiversioned() && name.is_none() {
            panic!(
                "Cannot send to a multiversioned location without a channel name. Please provide a name for the network."
            );
        }

        let raw_stream: Stream<(MemberId<L>, (K, V)), Process<'a, L2>, Unbounded, O, R> =
            Stream::new(
                to.clone(),
                HydroNode::Network {
                    name: name.map(ToOwned::to_owned),
                    serialize_fn: serialize_pipeline.map(|e| e.into()),
                    instantiate_fn: DebugInstantiate::Building,
                    deserialize_fn: deserialize_pipeline.map(|e| e.into()),
                    input: Box::new(self.ir_node.into_inner()),
                    metadata: to.new_node_metadata(Stream::<
                        (MemberId<L>, (K, V)),
                        Cluster<'a, L2>,
                        Unbounded,
                        O,
                        R,
                    >::collection_kind()),
                },
            );

        raw_stream
            .map(q!(|(sender, (k, v))| ((sender, k), v)))
            .into_keyed()
    }
}
