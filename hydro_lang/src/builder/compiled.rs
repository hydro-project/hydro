use std::collections::BTreeMap;
use std::marker::PhantomData;

use dfir_lang::graph::DfirGraph;
use dfir_rs::scheduled::graph::Dfir;
use proc_macro2::TokenStream;
use quote::quote;
use stageleft::QuotedWithContext;
use stageleft::runtime_support::{FreeVariableWithContext, QuoteTokens};

use crate::Location;
use crate::staging_util::Invariant;

pub struct CompiledFlow<'a, ID> {
    pub(super) dfir: BTreeMap<usize, DfirGraph>,
    pub(super) extra_stmts: BTreeMap<usize, Vec<syn::Stmt>>,
    pub(super) _phantom: Invariant<'a, ID>,
}

impl<'a, ID> CompiledFlow<'a, ID> {
    pub fn dfir_for(&self, location: &impl Location<'a>) -> &DfirGraph {
        self.dfir.get(&location.id().raw_id()).unwrap()
    }

    pub fn all_dfir(&self) -> &BTreeMap<usize, DfirGraph> {
        &self.dfir
    }
}

impl<'a> CompiledFlow<'a, usize> {
    pub fn with_dynamic_id(
        self,
        id: impl QuotedWithContext<'a, usize, ()>,
    ) -> CompiledFlowWithId<'a> {
        let hydro_lang_crate = proc_macro_crate::crate_name("hydro_lang")
            .expect("hydro_lang should be present in `Cargo.toml`");
        let root = match hydro_lang_crate {
            proc_macro_crate::FoundCrate::Itself => quote! { hydro_lang::dfir_rs },
            proc_macro_crate::FoundCrate::Name(name) => {
                let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
                quote! { #ident::dfir_rs }
            }
        };

        let mut conditioned_tokens = None;
        for (subgraph_id, partitioned_graph) in self.dfir {
            let mut diagnostics = Vec::new();
            let tokens = partitioned_graph.as_code(&root, true, quote::quote!(), &mut diagnostics);
            let my_extra_stmts = self
                .extra_stmts
                .get(&subgraph_id)
                .cloned()
                .unwrap_or_default();

            if let Some(conditioned_tokens) = conditioned_tokens.as_mut() {
                *conditioned_tokens = syn::parse_quote! {
                    #conditioned_tokens else if __given_id == #subgraph_id {
                        #(#my_extra_stmts)*
                        #tokens
                    }
                };
            } else {
                conditioned_tokens = Some(syn::parse_quote! {
                    if __given_id == #subgraph_id {
                        #(#my_extra_stmts)*
                        #tokens
                    }
                });
            }
        }

        let conditioned_tokens: TokenStream = conditioned_tokens.unwrap();
        let id = id.splice_untyped();
        CompiledFlowWithId {
            tokens: syn::parse_quote!({
                let __given_id = #id;
                #conditioned_tokens else {
                    panic!("Invalid node id: {}", __given_id);
                }
            }),
            _phantom: PhantomData,
        }
    }
}

impl<'a, Ctx> QuotedWithContext<'a, Dfir<'a>, Ctx> for CompiledFlow<'a, ()> {}

impl<'a, Ctx> FreeVariableWithContext<Ctx> for CompiledFlow<'a, ()> {
    type O = Dfir<'a>;

    fn to_tokens(mut self, _ctx: &Ctx) -> QuoteTokens {
        let hydro_lang_crate = proc_macro_crate::crate_name("hydro_lang")
            .expect("hydro_lang should be present in `Cargo.toml`");
        let root = match hydro_lang_crate {
            proc_macro_crate::FoundCrate::Itself => quote! { hydro_lang::dfir_rs },
            proc_macro_crate::FoundCrate::Name(name) => {
                let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
                quote! { #ident::dfir_rs }
            }
        };

        if self.dfir.len() != 1 {
            panic!("Expected exactly one subgraph in the DFIR.");
        }

        let partitioned_graph = self.dfir.remove(&0).unwrap();

        let mut diagnostics = Vec::new();
        let tokens = partitioned_graph.as_code(&root, true, quote::quote!(), &mut diagnostics);

        QuoteTokens {
            prelude: None,
            expr: Some(tokens),
        }
    }
}

pub struct CompiledFlowWithId<'a> {
    tokens: TokenStream,
    _phantom: Invariant<'a>,
}

impl<'a, Ctx> QuotedWithContext<'a, Dfir<'a>, Ctx> for CompiledFlowWithId<'a> {}

impl<'a, Ctx> FreeVariableWithContext<Ctx> for CompiledFlowWithId<'a> {
    type O = Dfir<'a>;

    fn to_tokens(self, _ctx: &Ctx) -> QuoteTokens {
        QuoteTokens {
            prelude: None,
            expr: Some(self.tokens),
        }
    }
}
