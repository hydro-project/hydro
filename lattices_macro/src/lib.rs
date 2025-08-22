//! Macros for the `lattices` crate.
//!
//! See [`[derive(Lattice)`](Lattice).
#![warn(missing_docs)]

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::punctuated::Punctuated;
use syn::visit_mut::VisitMut;
use syn::{
    Field, FieldsNamed, FieldsUnnamed, Generics, Ident, Index, ItemStruct, Member, Token,
    WhereClause, WherePredicate, parse_macro_input,
};

/// Tokens to reference the `lattices` crate.
fn root() -> TokenStream {
    use std::env::{VarError, var as env_var};

    use proc_macro_crate::FoundCrate;

    if let Ok(FoundCrate::Itself) = proc_macro_crate::crate_name("lattices_macro") {
        return quote! { lattices };
    }

    let lattices_crate_name = env!("CARGO_PKG_NAME").strip_suffix("_macro").unwrap();
    let lattices_crate_ident = lattices_crate_name.replace('-', "_");
    let lattices_crate = proc_macro_crate::crate_name(lattices_crate_name)
        .unwrap_or_else(|_| panic!("`{lattices_crate_name}` should be present in `Cargo.toml`"));
    match lattices_crate {
        FoundCrate::Itself => {
            if Err(VarError::NotPresent) == env_var("CARGO_BIN_NAME")
                && Ok(&*lattices_crate_ident) == env_var("CARGO_CRATE_NAME").as_deref()
            {
                // In the crate itself, including unit tests.
                quote! { crate }
            } else {
                // In an integration test, example, bench, etc.
                let ident = Ident::new(&lattices_crate_ident, Span::call_site());
                quote! { ::#ident }
            }
        }
        FoundCrate::Name(name) => {
            let ident = Ident::new(&name, Span::call_site());
            quote! { ::#ident }
        }
    }
}

/// Renames the generics and returns the updated `WherePredicate`s.
fn rename_generics(
    item_struct: &mut ItemStruct,
    rename: impl FnMut(&Ident) -> Ident,
) -> Vec<WherePredicate> {
    struct RenameGenerics<F> {
        rename: F,
        names: Vec<Ident>,
        pub triggered: bool,
    }
    impl<F> VisitMut for RenameGenerics<F>
    where
        F: FnMut(&Ident) -> Ident,
    {
        fn visit_ident_mut(&mut self, i: &mut Ident) {
            if self.names.contains(i) {
                *i = (self.rename)(i);
                self.triggered = true;
            }
        }
    }

    let names = item_struct
        .generics
        .type_params()
        .map(|type_param| type_param.ident.clone())
        .collect();
    let mut visit = RenameGenerics {
        rename,
        names,
        triggered: false,
    };

    let mut out = Vec::new();
    if let Some(where_clause) = &mut item_struct.generics.where_clause {
        for where_predicate in where_clause.predicates.iter_mut() {
            visit.visit_where_predicate_mut(where_predicate);
            if std::mem::take(&mut visit.triggered) {
                out.push(where_predicate.clone());
            }
        }
    }
    for type_param in item_struct.generics.type_params_mut() {
        visit.visit_type_param_mut(type_param);
    }
    for field in item_struct.fields.iter_mut() {
        visit.visit_type_mut(&mut field.ty);
    }
    out
}

/// Ensures that `punctuated` has trailing punctuation (or is empty).
fn ensure_trailing<T, P>(punctuated: &mut Punctuated<T, P>)
where
    P: Default,
{
    if !punctuated.empty_or_trailing() {
        punctuated.push_punct(Default::default());
    }
}

#[doc = include_str!("../README.md")]
#[proc_macro_derive(Lattice)]
pub fn derive_lattice_macro(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive_lattice(&process_item_struct(parse_macro_input!(item))).into()
}
/// Derives lattice `Merge`.
///
/// See [`#[derive(Lattice)]`](`Lattice`) for more info.
#[proc_macro_derive(Merge)]
pub fn derive_merge_macro(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive_merge(&process_item_struct(parse_macro_input!(item))).into()
}
/// Derives [`PartialEq`], [`PartialOrd`], and `LatticeOrd` together.
///
/// See [`#[derive(Lattice)]`](`Lattice`) for more info.
#[proc_macro_derive(LatticeOrd)]
pub fn derive_lattice_ord_macro(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive_lattice_ord(&process_item_struct(parse_macro_input!(item))).into()
}
/// Derives lattice `IsBot`.
///
/// See [`#[derive(Lattice)]`](`Lattice`) for more info.
#[proc_macro_derive(IsBot)]
pub fn derive_is_bot_macro(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive_is_bot(&process_item_struct(parse_macro_input!(item))).into()
}
/// Derives lattice `IsTop`.
///
/// See [`#[derive(Lattice)]`](`Lattice`) for more info.
#[proc_macro_derive(IsTop)]
pub fn derive_is_top_macro(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive_is_top(&process_item_struct(parse_macro_input!(item))).into()
}
/// Derives `LatticeFrom`.
///
/// See [`#[derive(Lattice)]`](`Lattice`) for more info.
#[proc_macro_derive(LatticeFrom)]
pub fn derive_lattice_from_macro(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive_lattice_from(&process_item_struct(parse_macro_input!(item))).into()
}

/// [`process_item_struct`] return value helper struct.
struct ProcessItemStruct {
    root: TokenStream,
    item_struct: ItemStruct,
    item_struct_renamed: ItemStruct,
    self_where_predicates: Punctuated<WherePredicate, Token![,]>,
    both_where_predicates: Punctuated<WherePredicate, Token![,]>,
    field_names: Vec<Member>,
    combined_generics: Generics,
}
/// Helper for common pre-processing code shared between macros.
fn process_item_struct(item_struct: ItemStruct) -> ProcessItemStruct {
    let mut item_struct_renamed = item_struct.clone();
    let extra_where_predicates = rename_generics(&mut item_struct_renamed, |ident| {
        format_ident!("__{}Other", ident)
    });

    // Basic `where` predicates, no extras.
    let mut self_where_predicates = item_struct
        .generics
        .where_clause
        .clone()
        .map(|WhereClause { predicates, .. }| predicates)
        .unwrap_or_default();
    ensure_trailing(&mut self_where_predicates);
    // Basic `where` predicates for combined original and renamed parameters.
    let mut both_where_predicates = self_where_predicates.clone();
    both_where_predicates.extend(extra_where_predicates);
    ensure_trailing(&mut both_where_predicates);

    // Fields.
    let field_names = match &item_struct.fields {
        syn::Fields::Named(FieldsNamed { named, .. }) => named
            .iter()
            .map(|Field { ident, .. }| Member::Named(ident.clone().unwrap()))
            .collect::<Vec<_>>(),
        syn::Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => (0..(unnamed.len() as u32))
            .map(|index| {
                Member::Unnamed(Index {
                    index,
                    span: Span::call_site(),
                })
            })
            .collect(),
        syn::Fields::Unit => Vec::new(),
    };

    // Extend the original generics.
    let mut combined_generics = item_struct.generics.clone();
    combined_generics
        .params
        .extend(item_struct_renamed.generics.params.clone());

    ProcessItemStruct {
        root: root(),
        item_struct,
        item_struct_renamed,
        self_where_predicates,
        both_where_predicates,
        field_names,
        combined_generics,
    }
}

/// See [`derive_lattice_macro`].
fn derive_lattice(process_item_struct: &ProcessItemStruct) -> TokenStream {
    let mut out = TokenStream::new();
    out.extend(derive_merge(process_item_struct));
    out.extend(derive_lattice_ord(process_item_struct));
    out.extend(derive_is_bot(process_item_struct));
    out.extend(derive_is_top(process_item_struct));
    out.extend(derive_lattice_from(process_item_struct));
    out
}

/// See [`derive_merge_macro`].
fn derive_merge(
    ProcessItemStruct {
        root,
        item_struct,
        item_struct_renamed,
        self_where_predicates: _,
        both_where_predicates,
        field_names,
        combined_generics,
    }: &ProcessItemStruct,
) -> TokenStream {
    let merge_where_predicates = item_struct
        .fields
        .iter()
        .zip(item_struct_renamed.fields.iter())
        .map(|(field_self, field_othr)| {
            let ty_self = &field_self.ty;
            let ty_othr = &field_othr.ty;
            quote! {
                #ty_self: #root::Merge<#ty_othr>
            }
        });

    let ident = &item_struct.ident;
    let (_, ty_generics_self, _) = item_struct.generics.split_for_impl();
    let (_, ty_generics_othr, _) = item_struct_renamed.generics.split_for_impl();
    let (impl_generics_both, _, _) = combined_generics.split_for_impl();
    quote! {
        impl #impl_generics_both #root::Merge<#ident #ty_generics_othr> for #ident #ty_generics_self
        where
            #both_where_predicates
            #( #merge_where_predicates ),*
        {
            fn merge(&mut self, other: #ident #ty_generics_othr) -> bool {
                let mut changed = false;
                #(
                    changed |= #root::Merge::merge(&mut self.#field_names, other.#field_names);
                )*
                changed
            }
        }
    }
}

/// See [`derive_lattice_ord_macro`].
fn derive_lattice_ord(
    ProcessItemStruct {
        root,
        item_struct,
        item_struct_renamed,
        self_where_predicates: _,
        both_where_predicates,
        field_names,
        combined_generics,
    }: &ProcessItemStruct,
) -> TokenStream {
    // PartialEq.
    let pareq_where_predicates = item_struct
        .fields
        .iter()
        .zip(item_struct_renamed.fields.iter())
        .map(|(field_self, field_othr)| {
            let ty_self = &field_self.ty;
            let ty_othr = &field_othr.ty;
            quote! {
                #ty_self: ::core::cmp::PartialEq<#ty_othr>
            }
        });
    // PartialOrd and LatticeOrd.
    let compare_where_predicates = item_struct
        .fields
        .iter()
        .zip(item_struct_renamed.fields.iter())
        .map(|(field_self, field_othr)| {
            let ty_self = &field_self.ty;
            let ty_othr = &field_othr.ty;
            quote! {
                #ty_self: ::core::cmp::PartialOrd<#ty_othr>
            }
        })
        .collect::<Vec<_>>();

    let ident = &item_struct.ident;
    let (_, ty_generics_self, _) = item_struct.generics.split_for_impl();
    let (_, ty_generics_othr, _) = item_struct_renamed.generics.split_for_impl();
    let (impl_generics_both, _, _) = combined_generics.split_for_impl();
    quote! {
        impl #impl_generics_both ::core::cmp::PartialEq<#ident #ty_generics_othr> for #ident #ty_generics_self
        where
            #both_where_predicates
            #( #pareq_where_predicates ),*
        {
            fn eq(&self, other: &#ident #ty_generics_othr) -> bool {
                #(
                    if !::core::cmp::PartialEq::eq(&self.#field_names, &other.#field_names) {
                        return false;
                    }
                )*
                true
            }
        }

        impl #impl_generics_both ::core::cmp::PartialOrd<#ident #ty_generics_othr> for #ident #ty_generics_self
        where
            #both_where_predicates
            #( #compare_where_predicates ),*
        {
            fn partial_cmp(&self, other: &#ident #ty_generics_othr) -> ::core::option::Option<::core::cmp::Ordering> {
                let mut self_any_greater = false;
                let mut othr_any_greater = false;
                #(
                    // `?` short-circuits `None` (uncomparable).
                    match ::core::cmp::PartialOrd::partial_cmp(&self.#field_names, &other.#field_names)? {
                        ::core::cmp::Ordering::Less => {
                            othr_any_greater = true;
                        }
                        ::core::cmp::Ordering::Greater => {
                            self_any_greater = true;
                        }
                        ::core::cmp::Ordering::Equal => {}
                    }
                    if self_any_greater && othr_any_greater {
                        return ::core::option::Option::None;
                    }
                )*
                ::core::option::Option::Some(
                    match (self_any_greater, othr_any_greater) {
                        (false, false) => ::core::cmp::Ordering::Equal,
                        (false, true) => ::core::cmp::Ordering::Less,
                        (true, false) => ::core::cmp::Ordering::Greater,
                        (true, true) => ::core::unreachable!(),
                    }
                )
            }
        }
        impl #impl_generics_both #root::LatticeOrd<#ident #ty_generics_othr> for #ident #ty_generics_self
        where
            #both_where_predicates
            #( #compare_where_predicates ),*
        {}
    }
}

/// See [`derive_is_bot_macro`].
fn derive_is_bot(
    ProcessItemStruct {
        root,
        item_struct,
        item_struct_renamed: _,
        self_where_predicates,
        both_where_predicates: _,
        field_names,
        combined_generics: _,
    }: &ProcessItemStruct,
) -> TokenStream {
    let isbot_where_predicates = item_struct.fields.iter().map(|Field { ty, .. }| {
        quote! {
            #ty: #root::IsBot
        }
    });

    let ident = &item_struct.ident;
    let (impl_generics_self, ty_generics_self, _) = item_struct.generics.split_for_impl();
    quote! {
        impl #impl_generics_self #root::IsBot for #ident #ty_generics_self
        where
            #self_where_predicates
            #( #isbot_where_predicates ),*
        {
            fn is_bot(&self) -> bool {
                #(
                    if !#root::IsBot::is_bot(&self.#field_names) {
                        return false;
                    }
                )*
                true
            }
        }
    }
}

/// See [`derive_is_top_macro`].
fn derive_is_top(
    ProcessItemStruct {
        root,
        item_struct,
        item_struct_renamed: _,
        self_where_predicates,
        both_where_predicates: _,
        field_names,
        combined_generics: _,
    }: &ProcessItemStruct,
) -> TokenStream {
    let istop_where_predicates = item_struct.fields.iter().map(|Field { ty, .. }| {
        quote! {
            #ty: #root::IsTop
        }
    });

    let ident = &item_struct.ident;
    let (impl_generics_self, ty_generics_self, _) = item_struct.generics.split_for_impl();
    quote! {
        impl #impl_generics_self #root::IsTop for #ident #ty_generics_self
        where
            #self_where_predicates
            #( #istop_where_predicates ),*
        {
            fn is_top(&self) -> bool {
                #(
                    if !#root::IsTop::is_top(&self.#field_names) {
                        return false;
                    }
                )*
                true
            }
        }
    }
}

/// See [`derive_lattice_from_macro`].
fn derive_lattice_from(
    ProcessItemStruct {
        root,
        item_struct,
        item_struct_renamed,
        self_where_predicates: _,
        both_where_predicates,
        field_names,
        combined_generics,
    }: &ProcessItemStruct,
) -> TokenStream {
    let latticefrom_where_predicates = item_struct
        .fields
        .iter()
        .zip(item_struct_renamed.fields.iter())
        .map(|(field_self, field_othr)| {
            let ty_self = &field_self.ty;
            let ty_othr = &field_othr.ty;
            quote! {
                #ty_self: #root::LatticeFrom<#ty_othr>
            }
        });

    let ident = &item_struct.ident;
    let (_, ty_generics_self, _) = item_struct.generics.split_for_impl();
    let (_, ty_generics_othr, _) = item_struct_renamed.generics.split_for_impl();
    let (impl_generics_both, _, _) = combined_generics.split_for_impl();
    quote! {
        impl #impl_generics_both #root::LatticeFrom<#ident #ty_generics_othr> for #ident #ty_generics_self
        where
            #both_where_predicates
            #( #latticefrom_where_predicates ),*
        {
            fn lattice_from(other: #ident #ty_generics_othr) -> Self {
                Self {
                    #(
                        #field_names: #root::LatticeFrom::lattice_from(other.#field_names),
                    )*
                }
            }
        }
    }
}

/// Also see `lattices/tests/macro.rs`
#[cfg(test)]
mod test {
    use syn::parse_quote;

    use super::*;

    /// Snapshots the macro output without actually testing if it compiles.
    /// See `lattices/tests/macro.rs` for compiling tests.
    macro_rules! assert_derive_snapshots {
        ( $( $t:tt )* ) => {
            {
                let item = parse_quote! {
                    $( $t )*
                };
                let process_item_struct = process_item_struct(item);
                let derive_lattice = derive_lattice(&process_item_struct);
                hydro_build_utils::assert_snapshot!(prettyplease::unparse(&parse_quote! { #derive_lattice }));
            }
        };
    }

    #[test]
    fn derive_example() {
        assert_derive_snapshots! {
            struct MyLattice<KeySet, Epoch> {
                keys: SetUnion<KeySet>,
                epoch: Max<Epoch>,
            }
        };
    }

    #[test]
    fn derive_pair() {
        assert_derive_snapshots! {
            pub struct Pair<LatA, LatB> {
                pub a: LatA,
                pub b: LatB,
            }
        };
    }

    #[test]
    fn derive_similar_fields() {
        // Will create duplicate where clauses, but that is OK.
        assert_derive_snapshots! {
            pub struct SimilarFields {
                a: Max<usize>,
                b: Max<usize>,
                c: Max<usize>,
            }
        };
    }
}
