use std::cell::RefCell;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::rc::Rc;

use internal::TokenStream;
use proc_macro2::Span;
use quote::quote;
use runtime_support::FreeVariable;
use stageleft::*;

use super::staging_util::get_this_crate;
use crate::ir::HfPlusLeaf;
use crate::location::{Cluster, ExternalProcess, Process};
use crate::{ClusterId, RuntimeContext};

pub mod built;
pub mod deploy;

pub struct FlowStateInner {
    /// Tracks the leaves of the dataflow IR. This is referenced by
    /// `Stream` and `HfCycle` to build the IR. The inner option will
    /// be set to `None` when this builder is finalized.
    pub(crate) leaves: Option<Vec<HfPlusLeaf>>,

    /// Counter for generating unique external output identifiers.
    pub(crate) next_external_out: usize,

    /// Counters for generating identifiers for cycles.
    pub(crate) cycle_counts: HashMap<usize, usize>,
}

pub type FlowState = Rc<RefCell<FlowStateInner>>;

pub struct ClusterIds<'a, C> {
    pub(crate) id: usize,
    pub(crate) _phantom: PhantomData<&'a mut &'a C>,
}

impl<'a, C> Clone for ClusterIds<'a, C> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, C> Copy for ClusterIds<'a, C> {}

impl<'a, C> FreeVariable<&'a Vec<ClusterId<C>>> for ClusterIds<'a, C> {
    fn to_tokens(self) -> (Option<TokenStream>, Option<TokenStream>)
    where
        Self: Sized,
    {
        let ident = syn::Ident::new(
            &format!("__hydroflow_plus_cluster_ids_{}", self.id),
            Span::call_site(),
        );
        let root = get_this_crate();
        let c_type = quote_type::<C>();
        (
            None,
            Some(
                quote! { unsafe { ::std::mem::transmute::<_, &::std::vec::Vec<#root::ClusterId<#c_type>>>(#ident) } },
            ),
        )
    }
}

impl<'a, C> Quoted<'a, &'a Vec<ClusterId<C>>> for ClusterIds<'a, C> {}

pub(crate) struct ClusterSelfId<'a, C> {
    pub(crate) id: usize,
    pub(crate) _phantom: PhantomData<&'a mut &'a C>,
}

impl<'a, C> Clone for ClusterSelfId<'a, C> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, C> Copy for ClusterSelfId<'a, C> {}

impl<'a, C> FreeVariable<ClusterId<C>> for ClusterSelfId<'a, C> {
    fn to_tokens(self) -> (Option<TokenStream>, Option<TokenStream>)
    where
        Self: Sized,
    {
        let ident = syn::Ident::new(
            &format!("__hydroflow_plus_cluster_self_id_{}", self.id),
            Span::call_site(),
        );
        let root = get_this_crate();
        let c_type: syn::Type = quote_type::<C>();
        (
            None,
            Some(quote! { #root::ClusterId::<#c_type>::from_raw(#ident) }),
        )
    }
}

impl<'a, C> Quoted<'a, ClusterId<C>> for ClusterSelfId<'a, C> {}

pub struct FlowBuilder<'a> {
    flow_state: FlowState,
    nodes: RefCell<Vec<usize>>,
    clusters: RefCell<Vec<usize>>,

    next_node_id: RefCell<usize>,

    /// Tracks whether this flow has been finalized; it is an error to
    /// drop without finalizing.
    finalized: bool,

    /// 'a on a FlowBuilder is used to ensure that staged code does not
    /// capture more data that it is allowed to; 'a is generated at the
    /// entrypoint of the staged code and we keep it invariant here
    /// to enforce the appropriate constraints
    _phantom: PhantomData<&'a mut &'a ()>,
}

impl<'a> Drop for FlowBuilder<'a> {
    fn drop(&mut self) {
        if !self.finalized {
            panic!("Dropped FlowBuilder without finalizing, you may have forgotten to call `with_default_optimize`, `optimize_with`, or `finalize`.");
        }
    }
}

impl<'a> QuotedContext for FlowBuilder<'a> {
    fn create() -> Self {
        FlowBuilder::new()
    }
}

impl<'a> FlowBuilder<'a> {
    #[expect(
        clippy::new_without_default,
        reason = "call `new` explicitly, not `default`"
    )]
    pub fn new() -> FlowBuilder<'a> {
        FlowBuilder {
            flow_state: Rc::new(RefCell::new(FlowStateInner {
                leaves: Some(vec![]),
                next_external_out: 0,
                cycle_counts: HashMap::new(),
            })),
            nodes: RefCell::new(vec![]),
            clusters: RefCell::new(vec![]),
            next_node_id: RefCell::new(0),
            finalized: false,
            _phantom: PhantomData,
        }
    }

    pub fn finalize(mut self) -> built::BuiltFlow<'a> {
        self.finalized = true;

        built::BuiltFlow {
            ir: self.flow_state.borrow_mut().leaves.take().unwrap(),
            processes: self.nodes.replace(vec![]),
            clusters: self.clusters.replace(vec![]),
            used: false,
            _phantom: PhantomData,
        }
    }

    pub fn with_default_optimize(self) -> built::BuiltFlow<'a> {
        self.finalize().with_default_optimize()
    }

    pub fn optimize_with(
        self,
        f: impl FnOnce(Vec<HfPlusLeaf>) -> Vec<HfPlusLeaf>,
    ) -> built::BuiltFlow<'a> {
        self.finalize().optimize_with(f)
    }

    pub fn flow_state(&self) -> &FlowState {
        &self.flow_state
    }

    pub fn process<P>(&self) -> Process<'a, P> {
        let mut next_node_id = self.next_node_id.borrow_mut();
        let id = *next_node_id;
        *next_node_id += 1;

        self.nodes.borrow_mut().push(id);

        Process {
            id,
            flow_state: self.flow_state().clone(),
            _phantom: PhantomData,
        }
    }

    pub fn external_process<P>(&self) -> ExternalProcess<'a, P> {
        let mut next_node_id = self.next_node_id.borrow_mut();
        let id = *next_node_id;
        *next_node_id += 1;

        self.nodes.borrow_mut().push(id);

        ExternalProcess {
            id,
            flow_state: self.flow_state().clone(),
            _phantom: PhantomData,
        }
    }

    pub fn cluster<C>(&self) -> Cluster<'a, C> {
        let mut next_node_id = self.next_node_id.borrow_mut();
        let id = *next_node_id;
        *next_node_id += 1;

        self.clusters.borrow_mut().push(id);

        Cluster {
            id,
            flow_state: self.flow_state().clone(),
            _phantom: PhantomData,
        }
    }

    pub fn runtime_context(&self) -> RuntimeContext<'a> {
        RuntimeContext {
            _phantom: PhantomData,
        }
    }
}
