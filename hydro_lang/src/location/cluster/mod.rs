use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

use proc_macro2::Span;
use quote::quote;
use stageleft::runtime_support::{FreeVariableWithContext, QuoteTokens};
use stageleft::{QuotedWithContext, quote_type};

use super::{Location, LocationId};
use crate::builder::FlowState;
use crate::staging_util::{Invariant, get_this_crate};

pub mod cluster_id;
pub use cluster_id::ClusterId;

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

impl<'a, C> Location<'a> for Cluster<'a, C> {
    type Root = Cluster<'a, C>;

    fn root(&self) -> Self::Root {
        self.clone()
    }

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

pub struct ClusterIds<'a, C> {
    pub(crate) id: usize,
    pub(crate) _phantom: Invariant<'a, C>,
}

impl<C> Clone for ClusterIds<'_, C> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<C> Copy for ClusterIds<'_, C> {}

impl<'a, C: 'a, Ctx> FreeVariableWithContext<Ctx> for ClusterIds<'a, C> {
    type O = &'a Vec<ClusterId<C>>;

    fn to_tokens(self, _ctx: &Ctx) -> QuoteTokens
    where
        Self: Sized,
    {
        let ident = syn::Ident::new(
            &format!("__hydro_lang_cluster_ids_{}", self.id),
            Span::call_site(),
        );
        let root = get_this_crate();
        let c_type = quote_type::<C>();

        QuoteTokens {
            prelude: None,
            expr: Some(
                quote! { unsafe { ::std::mem::transmute::<_, &[#root::ClusterId<#c_type>]>(#ident) } },
            ),
        }
    }
}

impl<'a, C, Ctx> QuotedWithContext<'a, &'a Vec<ClusterId<C>>, Ctx> for ClusterIds<'a, C> {}

pub trait IsCluster {
    type Tag;
}

impl<C> IsCluster for Cluster<'_, C> {
    type Tag = C;
}

/// A free variable representing the cluster's own ID. When spliced in
/// a quoted snippet that will run on a cluster, this turns into a [`ClusterId`].
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
    type O = ClusterId<<<L as Location<'a>>::Root as IsCluster>::Tag>;

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
            expr: Some(quote! { #root::ClusterId::<#c_type>::from_raw(#ident) }),
        }
    }
}

impl<'a, L> QuotedWithContext<'a, ClusterId<<<L as Location<'a>>::Root as IsCluster>::Tag>, L>
    for ClusterSelfId<'a>
where
    L: Location<'a>,
    <L as Location<'a>>::Root: IsCluster,
{
}
