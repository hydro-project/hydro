//! Utility methods for processing singleton references: `#my_var`.

use itertools::Itertools;
use proc_macro2::{Group, Ident, TokenStream, TokenTree};
use quote::quote_spanned;
use syn::punctuated::Punctuated;
use syn::{Expr, Token};

use crate::parse::parse_terminated;

/// Finds all the singleton references `#my_var` and appends them to `found_idents`. Returns the
/// `TokenStream` but with the hashes removed from the varnames.
///
/// The returned tokens are used for "preflight" parsing, to check that the rest of the syntax is
/// OK. However the returned tokens are not used in the codegen as we need to use [`postprocess_singletons`]
/// later to substitute-in the context referencing code for each singleton
pub fn preprocess_singletons(tokens: TokenStream, found_idents: &mut Vec<Ident>) -> TokenStream {
    process_singletons(tokens, &mut |singleton_ident| {
        found_idents.push(singleton_ident.clone());
        TokenTree::Ident(singleton_ident)
    })
}

/// Replaces singleton references `#my_var` with the code needed to actually get the value inside.
///
/// * `tokens` - The tokens to update singleton references within.
/// * `resolved_idents` - The context `StateHandle` varnames that correspond 1:1 and in the same
///   order as the singleton references within `tokens` (found in-order via [`preprocess_singletons`]).
///
/// Generates borrowing code ([`std::cell::RefCell::borrow_mut`]). Use
/// [`postprocess_singletons_handles`] for just the `StateHandle`s.
pub fn postprocess_singletons(
    tokens: TokenStream,
    resolved_idents: impl IntoIterator<Item = Ident>,
    context: &Ident,
) -> Punctuated<Expr, Token![,]> {
    let mut resolved_idents_iter = resolved_idents.into_iter();
    let processed = process_singletons(tokens, &mut |singleton_ident| {
        let span = singleton_ident.span();
        let context = Ident::new(&context.to_string(), span.resolved_at(context.span()));
        let mut resolved_ident = resolved_idents_iter.next().unwrap();
        resolved_ident.set_span(span);
        let mut group = Group::new(
            proc_macro2::Delimiter::Parenthesis,
            quote_spanned! {span=>
                *(unsafe {
                    // SAFETY: `handle` is from this instance.
                    #context.state_ref_unchecked(#resolved_ident)
                }.borrow_mut())
            },
        );
        group.set_span(singleton_ident.span());
        TokenTree::Group(group)
    });
    parse_terminated(processed).unwrap()
}

/// Same as [`postprocess_singletons`] but generates just the `StateHandle` ident rather than full
/// `RefCell` borrowing code.
pub fn postprocess_singletons_handles(
    tokens: TokenStream,
    resolved_idents: impl IntoIterator<Item = Ident>,
) -> Punctuated<Expr, Token![,]> {
    let mut resolved_idents_iter = resolved_idents.into_iter();
    let processed = process_singletons(tokens, &mut |singleton_ident| {
        let mut resolved_ident = resolved_idents_iter.next().unwrap();
        resolved_ident.set_span(singleton_ident.span().resolved_at(resolved_ident.span()));
        TokenTree::Ident(resolved_ident)
    });
    parse_terminated(processed).unwrap()
}

/// Traverse the token stream, applying the `map_singleton_fn` whenever a singleton is found,
/// returning the transformed token stream.
fn process_singletons(
    tokens: TokenStream,
    map_singleton_fn: &mut impl FnMut(Ident) -> TokenTree,
) -> TokenStream {
    tokens
        .into_iter()
        .peekable()
        .batching(|iter| {
            let out = match iter.next()? {
                TokenTree::Group(group) => {
                    let mut new_group = Group::new(
                        group.delimiter(),
                        process_singletons(group.stream(), map_singleton_fn),
                    );
                    new_group.set_span(group.span());
                    TokenTree::Group(new_group)
                }
                TokenTree::Ident(ident) => TokenTree::Ident(ident),
                TokenTree::Punct(punct) => {
                    if '#' == punct.as_char() && matches!(iter.peek(), Some(TokenTree::Ident(_))) {
                        // Found a singleton.
                        let Some(TokenTree::Ident(mut singleton_ident)) = iter.next() else {
                            unreachable!()
                        };
                        {
                            // Include the `#` in the span.
                            let span = singleton_ident
                                .span()
                                .join(punct.span())
                                .unwrap_or(singleton_ident.span());
                            singleton_ident.set_span(span.resolved_at(singleton_ident.span()));
                        }
                        (map_singleton_fn)(singleton_ident)
                    } else {
                        TokenTree::Punct(punct)
                    }
                }
                TokenTree::Literal(lit) => TokenTree::Literal(lit),
            };
            Some(out)
        })
        .collect()
}
