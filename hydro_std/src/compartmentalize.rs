use hydro_lang::boundedness::Boundedness;
use hydro_lang::location::MemberId;
use hydro_lang::location::cluster::CLUSTER_SELF_ID;
use hydro_lang::*;
use location::NoTick;
use serde::Serialize;
use serde::de::DeserializeOwned;
use stageleft::IntoQuotedMut;

pub trait PartitionStream<'a, T, C1, C2, Order> {
    fn send_partitioned<F: Fn((MemberId<C2>, T)) -> (MemberId<C2>, T) + 'a>(
        self,
        other: &Cluster<'a, C2>,
        dist_policy: impl IntoQuotedMut<'a, F, Cluster<'a, C1>>,
    ) -> Stream<T, Cluster<'a, C2>, Unbounded, NoOrder>
    where
        T: Clone + Serialize + DeserializeOwned;
}

impl<'a, T, C1, C2, Order> PartitionStream<'a, T, C1, C2, Order>
    for Stream<(MemberId<C2>, T), Cluster<'a, C1>, Unbounded, Order>
{
    fn send_partitioned<F: Fn((MemberId<C2>, T)) -> (MemberId<C2>, T) + 'a>(
        self,
        other: &Cluster<'a, C2>,
        dist_policy: impl IntoQuotedMut<'a, F, Cluster<'a, C1>>,
    ) -> Stream<T, Cluster<'a, C2>, Unbounded, NoOrder>
    where
        T: Clone + Serialize + DeserializeOwned,
    {
        self.map(dist_policy).demux_bincode(other).values()
    }
}

pub trait DecoupleClusterStream<'a, T, C1, B, Order> {
    fn decouple_cluster<C2: 'a>(
        self,
        other: &Cluster<'a, C2>,
    ) -> Stream<T, Cluster<'a, C2>, Unbounded, Order>
    where
        T: Clone + Serialize + DeserializeOwned;
}

impl<'a, T, C1, B: Boundedness, Order> DecoupleClusterStream<'a, T, C1, B, Order>
    for Stream<T, Cluster<'a, C1>, B, Order>
{
    fn decouple_cluster<C2: 'a>(
        self,
        other: &Cluster<'a, C2>,
    ) -> Stream<T, Cluster<'a, C2>, Unbounded, Order>
    where
        T: Clone + Serialize + DeserializeOwned,
    {
        let sent = self
            .map(q!(move |b| (
                MemberId::from_raw(CLUSTER_SELF_ID.raw_id),
                b.clone()
            )))
            .demux_bincode(other)
            .values();

        sent.assume_ordering(
            nondet!(/** this is safe because we are only receiving from one sender */),
        )
    }
}

pub trait DecoupleProcessStream<'a, T, L: Location<'a> + NoTick, B, Order> {
    fn decouple_process<P2>(
        self,
        other: &Process<'a, P2>,
    ) -> Stream<T, Process<'a, P2>, Unbounded, Order>
    where
        T: Clone + Serialize + DeserializeOwned;
}

impl<'a, T, L, B: Boundedness, Order> DecoupleProcessStream<'a, T, Process<'a, L>, B, Order>
    for Stream<T, Process<'a, L>, B, Order>
{
    fn decouple_process<P2>(
        self,
        other: &Process<'a, P2>,
    ) -> Stream<T, Process<'a, P2>, Unbounded, Order>
    where
        T: Clone + Serialize + DeserializeOwned,
    {
        self.send_bincode(other)
    }
}
