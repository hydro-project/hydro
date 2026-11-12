use std::hash::Hash;

use hydro_lang::live_collections::OperatorContext;
use hydro_lang::live_collections::boundedness::Boundedness;
use hydro_lang::live_collections::stream::{NoOrder, Ordering};
use hydro_lang::location::cluster::CLUSTER_SELF_ID;
use hydro_lang::location::{Location, MemberId};
use hydro_lang::prelude::*;
use hydro_lang::properties::{NotProved, StreamMapFuncAlgebra};
use serde::Serialize;
use serde::de::DeserializeOwned;
use stageleft::IntoQuotedMut;

pub trait PartitionStream<'a, T, C1, C2, Order: Ordering, Id1 = u32, Id2 = u32> {
    fn send_partitioned<F>(
        self,
        other: &Cluster<'a, C2, Id2>,
        dist_policy: impl IntoQuotedMut<
            'a,
            F,
            OperatorContext<Cluster<'a, C1, Id1>, Unbounded>,
            StreamMapFuncAlgebra<
                (MemberId<C2, Id2>, T),
                Unbounded,
                NotProved,
                NotProved,
                hydro_lang::sim_hooks::OnCluster<C1>,
            >,
        >,
    ) -> Stream<T, Cluster<'a, C2, Id2>, Unbounded, NoOrder>
    where
        T: Clone + Serialize + DeserializeOwned,
        F: Fn((MemberId<C2, Id2>, T)) -> (MemberId<C2, Id2>, T) + 'a;
}

impl<'a, T, C1, Id1: Clone + Eq + Hash + 'a, C2, Id2: Clone + Eq + Hash + 'a, Order: Ordering>
    PartitionStream<'a, T, C1, C2, Order, Id1, Id2>
    for Stream<(MemberId<C2, Id2>, T), Cluster<'a, C1, Id1>, Unbounded, Order>
{
    fn send_partitioned<F>(
        self,
        other: &Cluster<'a, C2, Id2>,
        dist_policy: impl IntoQuotedMut<
            'a,
            F,
            OperatorContext<Cluster<'a, C1, Id1>, Unbounded>,
            StreamMapFuncAlgebra<
                (MemberId<C2, Id2>, T),
                Unbounded,
                NotProved,
                NotProved,
                hydro_lang::sim_hooks::OnCluster<C1>,
            >,
        >,
    ) -> Stream<T, Cluster<'a, C2, Id2>, Unbounded, NoOrder>
    where
        T: Clone + Serialize + DeserializeOwned,
        F: Fn((MemberId<C2, Id2>, T)) -> (MemberId<C2, Id2>, T) + 'a,
    {
        self.map(dist_policy)
            .demux(other, TCP.fail_stop().bincode())
            .values()
    }
}

pub trait DecoupleClusterStream<'a, T, C1, B, Order: Ordering, Id = u32> {
    fn decouple_cluster<C2: 'a>(
        self,
        other: &Cluster<'a, C2, Id>,
    ) -> Stream<T, Cluster<'a, C2, Id>, Unbounded, Order>
    where
        T: Clone + Serialize + DeserializeOwned,
        C1: 'a;
}

impl<'a, T, C1, B: Boundedness, Order: Ordering, Id: Clone + Eq + Hash + 'a>
    DecoupleClusterStream<'a, T, C1, B, Order, Id> for Stream<T, Cluster<'a, C1, Id>, B, Order>
where
    C1: 'a,
{
    fn decouple_cluster<C2: 'a>(
        self,
        other: &Cluster<'a, C2, Id>,
    ) -> Stream<T, Cluster<'a, C2, Id>, Unbounded, Order>
    where
        T: Clone + Serialize + DeserializeOwned,
        C1: 'a,
    {
        let sent = self
            .map(q!(move |b| (
                MemberId::from_tagless(CLUSTER_SELF_ID.clone().into_tagless()), // this is a seemingly round about way to convert from one member id tag to another.
                b
            )))
            .demux(other, TCP.fail_stop().bincode())
            .values();

        sent.assume_ordering(
            nondet!(/** this is safe because we are only receiving from one sender */),
        )
    }
}

pub trait DecoupleProcessStream<'a, T, L: Location<'a>, B, Order: Ordering> {
    fn decouple_process<P2>(
        self,
        other: &Process<'a, P2>,
    ) -> Stream<T, Process<'a, P2>, Unbounded, Order>
    where
        T: Clone + Serialize + DeserializeOwned;
}

impl<'a, T, L, B: Boundedness, Order: Ordering>
    DecoupleProcessStream<'a, T, Process<'a, L>, B, Order> for Stream<T, Process<'a, L>, B, Order>
{
    fn decouple_process<P2>(
        self,
        other: &Process<'a, P2>,
    ) -> Stream<T, Process<'a, P2>, Unbounded, Order>
    where
        T: Clone + Serialize + DeserializeOwned,
    {
        self.send(other, TCP.fail_stop().bincode())
    }
}
