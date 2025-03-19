use hydro_lang::*;
use location::{CanSend, NoTick};
use serde::Serialize;
use serde::de::DeserializeOwned;
use stageleft::IntoQuotedMut;
use stream::MinOrder;

pub trait PartitionStream<'a, T, C1, C2, Order> {
    fn send_partitioned<Tag, F: Fn((ClusterId<C2>, T)) -> (ClusterId<C2>, T) + 'a>(
        self,
        other: &Cluster<'a, C2>,
        dist_policy: impl IntoQuotedMut<'a, F, Cluster<'a, C1>>,
    ) -> Stream<T, Cluster<'a, C2>, Unbounded, NoOrder>
    where
        Cluster<'a, C1>: Location<'a, Root = Cluster<'a, C1>>,
        Cluster<'a, C1>:
            CanSend<'a, Cluster<'a, C2>, In<T> = (ClusterId<C2>, T), Out<T> = (Tag, T)>,
        T: Clone + Serialize + DeserializeOwned,
        Order: MinOrder<
                <Cluster<'a, C1> as CanSend<'a, Cluster<'a, C2>>>::OutStrongestOrder<Order>,
                Min = NoOrder,
            >;
}

impl<'a, T, C1, C2, Order> PartitionStream<'a, T, C1, C2, Order>
    for Stream<(ClusterId<C2>, T), Cluster<'a, C1>, Unbounded, Order>
{
    fn send_partitioned<Tag, F: Fn((ClusterId<C2>, T)) -> (ClusterId<C2>, T) + 'a>(
        self,
        other: &Cluster<'a, C2>,
        dist_policy: impl IntoQuotedMut<'a, F, Cluster<'a, C1>>,
    ) -> Stream<T, Cluster<'a, C2>, Unbounded, NoOrder>
    where
        Cluster<'a, C1>: Location<'a, Root = Cluster<'a, C1>>,
        Cluster<'a, C1>:
            CanSend<'a, Cluster<'a, C2>, In<T> = (ClusterId<C2>, T), Out<T> = (Tag, T)>,
        T: Clone + Serialize + DeserializeOwned,
        Order: MinOrder<
                <Cluster<'a, C1> as CanSend<'a, Cluster<'a, C2>>>::OutStrongestOrder<Order>,
                Min = NoOrder,
            >,
    {
        self.map(dist_policy).send_bincode_anonymous(other)
    }
}

pub trait DecoupleClusterStream<'a, T, C1, B, Order> {
    fn decouple_cluster<C2: 'a, Tag>(
        self,
        other: &Cluster<'a, C2>,
    ) -> Stream<T, Cluster<'a, C2>, Unbounded, Order>
    where
        Cluster<'a, C1>: Location<'a, Root = Cluster<'a, C1>>,
        Cluster<'a, C1>:
            CanSend<'a, Cluster<'a, C2>, In<T> = (ClusterId<C2>, T), Out<T> = (Tag, T)>,
        T: Clone + Serialize + DeserializeOwned,
        Order:
            MinOrder<<Cluster<'a, C1> as CanSend<'a, Cluster<'a, C2>>>::OutStrongestOrder<Order>>;
}

impl<'a, T, C1, B, Order> DecoupleClusterStream<'a, T, C1, B, Order>
    for Stream<T, Cluster<'a, C1>, B, Order>
{
    fn decouple_cluster<C2: 'a, Tag>(
        self,
        other: &Cluster<'a, C2>,
    ) -> Stream<T, Cluster<'a, C2>, Unbounded, Order>
    where
        Cluster<'a, C1>: Location<'a, Root = Cluster<'a, C1>>,
        Cluster<'a, C1>:
            CanSend<'a, Cluster<'a, C2>, In<T> = (ClusterId<C2>, T), Out<T> = (Tag, T)>,
        T: Clone + Serialize + DeserializeOwned,
        Order:
            MinOrder<<Cluster<'a, C1> as CanSend<'a, Cluster<'a, C2>>>::OutStrongestOrder<Order>>,
    {
        let sent = self
            .map(q!(move |b| (
                ClusterId::from_raw(CLUSTER_SELF_ID.raw_id),
                b.clone()
            )))
            .send_bincode_anonymous(other);

        unsafe {
            // SAFETY: this is safe because we are mapping clusters 1:1
            sent.assume_ordering()
        }
    }
}

pub trait DecoupleProcessStream<'a, T, L: Location<'a> + NoTick, B, Order> {
    fn decouple_process<P2>(
        self,
        other: &Process<'a, P2>,
    ) -> Stream<T, Process<'a, P2>, Unbounded, Order>
    where
        L::Root: CanSend<'a, Process<'a, P2>, In<T> = T, Out<T> = T>,
        T: Clone + Serialize + DeserializeOwned,
        Order: MinOrder<
                <L::Root as CanSend<'a, Process<'a, P2>>>::OutStrongestOrder<Order>,
                Min = Order,
            >;
}

impl<'a, T, L: Location<'a> + NoTick, B, Order> DecoupleProcessStream<'a, T, L, B, Order>
    for Stream<T, L, B, Order>
{
    fn decouple_process<P2>(
        self,
        other: &Process<'a, P2>,
    ) -> Stream<T, Process<'a, P2>, Unbounded, Order>
    where
        L::Root: CanSend<'a, Process<'a, P2>, In<T> = T, Out<T> = T>,
        T: Clone + Serialize + DeserializeOwned,
        Order: MinOrder<
                <L::Root as CanSend<'a, Process<'a, P2>>>::OutStrongestOrder<Order>,
                Min = Order,
            >,
    {
        self.send_bincode::<Process<'a, P2>, T>(other)
    }
}
