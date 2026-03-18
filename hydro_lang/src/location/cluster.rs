//! Definitions for clusters, which represent a group of identical processes.
//!
//! A [`Cluster`] is a multi-node location in the Hydro distributed programming model.
//! Unlike a [`super::Process`], which maps to a single machine, a cluster represents
//! a dynamically-sized set of machines that all run the same code. Each member of the
//! cluster is assigned a unique [`super::MemberId`] that can be used to address it.
//!
//! Clusters are useful for parallelism, replication, and sharding patterns. Data can
//! be broadcast to all members, sent to a specific member by ID, or scattered across
//! members.

use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

use proc_macro2::Span;
use quote::quote;
use stageleft::runtime_support::{FreeVariableWithContextWithProps, QuoteTokens};
use stageleft::{QuotedWithContextWithProps, quote_type};

use super::dynamic::LocationId;
use super::{Location, MemberId};
use crate::compile::builder::FlowState;
use crate::location::LocationKey;
use crate::location::member_id::TaglessMemberId;
use crate::staging_util::{Invariant, get_this_crate};

/// A multi-node location representing a group of identical processes.
///
/// Each member of the cluster runs the same dataflow program and is assigned a
/// unique [`MemberId`] that can be used to address it. The number of members
/// is determined at deployment time rather than at compile time.
///
/// The `ClusterTag` type parameter is a phantom tag used to distinguish between
/// different clusters in the type system, preventing accidental mixing of
/// member IDs across clusters.
pub struct Cluster<'a, ClusterTag> {
    pub(crate) key: LocationKey,
    pub(crate) flow_state: FlowState,
    pub(crate) _phantom: Invariant<'a, ClusterTag>,
}

impl<C> Debug for Cluster<'_, C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Cluster({})", self.key)
    }
}

impl<C> Eq for Cluster<'_, C> {}
impl<C> PartialEq for Cluster<'_, C> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key && FlowState::ptr_eq(&self.flow_state, &other.flow_state)
    }
}

impl<C> Clone for Cluster<'_, C> {
    fn clone(&self) -> Self {
        Cluster {
            key: self.key,
            flow_state: self.flow_state.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<'a, C> super::dynamic::DynLocation for Cluster<'a, C> {
    fn id(&self) -> LocationId {
        LocationId::Cluster(self.key)
    }

    fn flow_state(&self) -> &FlowState {
        &self.flow_state
    }

    fn is_top_level() -> bool {
        true
    }

    fn multiversioned(&self) -> bool {
        false // TODO(shadaj): enable multiversioning support for clusters
    }
}

impl<'a, C> Location<'a> for Cluster<'a, C> {
    type Root = Cluster<'a, C>;

    fn root(&self) -> Self::Root {
        self.clone()
    }
}

/// A free variable that resolves to the list of member IDs in a cluster at runtime.
///
/// When spliced into a quoted snippet, this provides access to the set of
/// [`TaglessMemberId`]s that belong to the cluster.
pub struct ClusterIds<'a> {
    /// The location key identifying which cluster this refers to.
    pub key: LocationKey,
    /// Phantom data binding the lifetime.
    pub _phantom: PhantomData<&'a ()>,
}

impl<'a> Clone for ClusterIds<'a> {
    fn clone(&self) -> Self {
        Self {
            key: self.key,
            _phantom: Default::default(),
        }
    }
}

impl<'a, Ctx> FreeVariableWithContextWithProps<Ctx, ()> for ClusterIds<'a> {
    type O = &'a [TaglessMemberId];

    fn to_tokens(self, _ctx: &Ctx) -> (QuoteTokens, ())
    where
        Self: Sized,
    {
        let ident = syn::Ident::new(
            &format!("__hydro_lang_cluster_ids_{}", self.key),
            Span::call_site(),
        );

        (
            QuoteTokens {
                prelude: None,
                expr: Some(quote! { #ident }),
            },
            (),
        )
    }
}

impl<'a, Ctx> QuotedWithContextWithProps<'a, &'a [TaglessMemberId], Ctx, ()> for ClusterIds<'a> {}

/// Marker trait implemented by [`Cluster`] locations, providing access to the cluster tag type.
pub trait IsCluster {
    /// The phantom tag type that distinguishes this cluster from others.
    type Tag;
}

impl<C> IsCluster for Cluster<'_, C> {
    type Tag = C;
}

/// A free variable representing the cluster's own ID. When spliced in
/// a quoted snippet that will run on a cluster, this turns into a [`MemberId`].
pub static CLUSTER_SELF_ID: ClusterSelfId = ClusterSelfId { _private: &() };

/// The concrete type behind [`CLUSTER_SELF_ID`].
///
/// This is a compile-time variable that, when spliced into a quoted snippet running
/// on a [`Cluster`], resolves to the [`MemberId`] of the current cluster member.
#[derive(Clone, Copy)]
pub struct ClusterSelfId<'a> {
    _private: &'a (),
}

impl<'a, L> FreeVariableWithContextWithProps<L, ()> for ClusterSelfId<'a>
where
    L: Location<'a>,
    <L as Location<'a>>::Root: IsCluster,
{
    type O = MemberId<<<L as Location<'a>>::Root as IsCluster>::Tag>;

    fn to_tokens(self, ctx: &L) -> (QuoteTokens, ())
    where
        Self: Sized,
    {
        let cluster_id = if let LocationId::Cluster(id) = ctx.root().id() {
            id
        } else {
            unreachable!()
        };

        let ident = syn::Ident::new(
            &format!("__hydro_lang_cluster_self_id_{}", cluster_id),
            Span::call_site(),
        );
        let root = get_this_crate();
        let c_type: syn::Type = quote_type::<<<L as Location<'a>>::Root as IsCluster>::Tag>();

        (
            QuoteTokens {
                prelude: None,
                expr: Some(
                    quote! { #root::__staged::location::MemberId::<#c_type>::from_tagless((#ident).clone()) },
                ),
            },
            (),
        )
    }
}

impl<'a, L>
    QuotedWithContextWithProps<'a, MemberId<<<L as Location<'a>>::Root as IsCluster>::Tag>, L, ()>
    for ClusterSelfId<'a>
where
    L: Location<'a>,
    <L as Location<'a>>::Root: IsCluster,
{
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "sim")]
    use stageleft::q;

    #[cfg(feature = "sim")]
    use super::CLUSTER_SELF_ID;
    #[cfg(feature = "sim")]
    use crate::location::{Location, MemberId, MembershipEvent};
    #[cfg(feature = "sim")]
    use crate::networking::TCP;
    #[cfg(feature = "sim")]
    use crate::nondet::nondet;
    #[cfg(feature = "sim")]
    use crate::prelude::FlowBuilder;

    #[cfg(feature = "sim")]
    #[test]
    fn sim_cluster_self_id() {
        let mut flow = FlowBuilder::new();
        let cluster1 = flow.cluster::<()>();
        let cluster2 = flow.cluster::<()>();

        let node = flow.process::<()>();

        let out_recv = cluster1
            .source_iter(q!(vec![CLUSTER_SELF_ID]))
            .send(&node, TCP.fail_stop().bincode())
            .values()
            .merge_unordered(
                cluster2
                    .source_iter(q!(vec![CLUSTER_SELF_ID]))
                    .send(&node, TCP.fail_stop().bincode())
                    .values(),
            )
            .sim_output();

        flow.sim()
            .with_cluster_size(&cluster1, 3)
            .with_cluster_size(&cluster2, 4)
            .exhaustive(async || {
                out_recv
                    .assert_yields_only_unordered([0, 1, 2, 0, 1, 2, 3].map(MemberId::from_raw_id))
                    .await
            });
    }

    #[cfg(feature = "sim")]
    #[test]
    fn sim_cluster_with_tick() {
        use std::collections::HashMap;

        let mut flow = FlowBuilder::new();
        let cluster = flow.cluster::<()>();
        let node = flow.process::<()>();

        let out_recv = cluster
            .source_iter(q!(vec![1, 2, 3]))
            .batch(&cluster.tick(), nondet!(/** test */))
            .count()
            .all_ticks()
            .send(&node, TCP.fail_stop().bincode())
            .entries()
            .map(q!(|(id, v)| (id, v)))
            .sim_output();

        let count = flow
            .sim()
            .with_cluster_size(&cluster, 2)
            .exhaustive(async || {
                let grouped = out_recv.collect_sorted::<Vec<_>>().await.into_iter().fold(
                    HashMap::new(),
                    |mut acc: HashMap<MemberId<()>, usize>, (id, v)| {
                        *acc.entry(id).or_default() += v;
                        acc
                    },
                );

                assert!(grouped.len() == 2);
                for (_id, v) in grouped {
                    assert!(v == 3);
                }
            });

        assert_eq!(count, 106);
        // not a square because we simulate all interleavings of ticks across 2 cluster members
        // eventually, we should be able to identify that the members are independent (because
        // there are no dataflow cycles) and avoid simulating redundant interleavings
    }

    #[cfg(feature = "sim")]
    #[test]
    fn sim_cluster_membership() {
        let mut flow = FlowBuilder::new();
        let cluster = flow.cluster::<()>();
        let node = flow.process::<()>();

        let out_recv = node
            .source_cluster_members(&cluster)
            .entries()
            .map(q!(|(id, v)| (id, v)))
            .sim_output();

        flow.sim()
            .with_cluster_size(&cluster, 2)
            .exhaustive(async || {
                out_recv
                    .assert_yields_only_unordered(vec![
                        (MemberId::from_raw_id(0), MembershipEvent::Joined),
                        (MemberId::from_raw_id(1), MembershipEvent::Joined),
                    ])
                    .await;
            });
    }
}
