use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

use proc_macro2::Span;
use quote::quote;
use stageleft::runtime_support::{FreeVariableWithContext, QuoteTokens};
use stageleft::{QuotedWithContext, quote_type};

use super::dynamic::LocationId;
use super::{Location, MemberId};
use crate::compile::builder::FlowState;
use crate::location::member_id::TaglessMemberId;
use crate::staging_util::{Invariant, get_this_crate};

pub struct Cluster<'a, ClusterTag> {
    pub(crate) id: usize,
    pub(crate) flow_state: FlowState,
    pub(crate) _phantom: Invariant<'a, ClusterTag>,
}

impl<C> Debug for Cluster<'_, C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Cluster({})", self.id)
    }
}

impl<C> Eq for Cluster<'_, C> {}
impl<C> PartialEq for Cluster<'_, C> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && FlowState::ptr_eq(&self.flow_state, &other.flow_state)
    }
}

impl<C> Clone for Cluster<'_, C> {
    fn clone(&self) -> Self {
        Cluster {
            id: self.id,
            flow_state: self.flow_state.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<'a, C> super::dynamic::DynLocation for Cluster<'a, C> {
    fn id(&self) -> LocationId {
        LocationId::Cluster(self.id)
    }

    fn flow_state(&self) -> &FlowState {
        &self.flow_state
    }

    fn is_top_level() -> bool {
        true
    }
}

impl<'a, C> Location<'a> for Cluster<'a, C> {
    type Root = Cluster<'a, C>;

    fn root(&self) -> Self::Root {
        self.clone()
    }
}

pub struct ClusterIds<'a> {
    pub id: usize,
    pub _phantom: PhantomData<&'a ()>,
}

impl<'a> Clone for ClusterIds<'a> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            _phantom: Default::default(),
        }
    }
}

impl<'a, Ctx> FreeVariableWithContext<Ctx> for ClusterIds<'a> {
    type O = &'a [TaglessMemberId];

    fn to_tokens(self, _ctx: &Ctx) -> QuoteTokens
    where
        Self: Sized,
    {
        let ident = syn::Ident::new(
            &format!("__hydro_lang_cluster_ids_{}", self.id),
            Span::call_site(),
        );

        QuoteTokens {
            prelude: None,
            expr: Some(quote! { #ident }),
        }
    }
}

impl<'a, Ctx> QuotedWithContext<'a, &'a [TaglessMemberId], Ctx> for ClusterIds<'a> {}

pub trait IsCluster {
    type Tag;
}

impl<C> IsCluster for Cluster<'_, C> {
    type Tag = C;
}

/// A free variable representing the cluster's own ID. When spliced in
/// a quoted snippet that will run on a cluster, this turns into a [`MemberId`].
pub static CLUSTER_SELF_ID: ClusterSelfId = ClusterSelfId { _private: &() };

#[derive(Clone, Copy)]
pub struct ClusterSelfId<'a> {
    _private: &'a (),
}

impl<'a, L> FreeVariableWithContext<L> for ClusterSelfId<'a>
where
    L: Location<'a>,
    <L as Location<'a>>::Root: IsCluster,
{
    type O = MemberId<<<L as Location<'a>>::Root as IsCluster>::Tag>;

    fn to_tokens(self, ctx: &L) -> QuoteTokens
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

        QuoteTokens {
            prelude: None,
            expr: Some(
                quote! { #root::location::MemberId::<#c_type>::from_tagless((#ident).clone()) },
            ),
        }
    }
}

impl<'a, L> QuotedWithContext<'a, MemberId<<<L as Location<'a>>::Root as IsCluster>::Tag>, L>
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
    use crate::nondet::nondet;
    #[cfg(feature = "sim")]
    use crate::prelude::FlowBuilder;

    #[cfg(feature = "sim")]
    #[test]
    fn sim_cluster_self_id() {
        let flow = FlowBuilder::new();
        let cluster1 = flow.cluster::<()>();
        let cluster2 = flow.cluster::<()>();

        let node = flow.process::<()>();

        let out_recv = cluster1
            .source_iter(q!(vec![CLUSTER_SELF_ID]))
            .send_bincode(&node)
            .values()
            .interleave(
                cluster2
                    .source_iter(q!(vec![CLUSTER_SELF_ID]))
                    .send_bincode(&node)
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

        let flow = FlowBuilder::new();
        let cluster = flow.cluster::<()>();
        let node = flow.process::<()>();

        let out_recv = cluster
            .source_iter(q!(vec![1, 2, 3]))
            .batch(&cluster.tick(), nondet!(/** test */))
            .count()
            .all_ticks()
            .send_bincode(&node)
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
        let flow = FlowBuilder::new();
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
