//! Utility methods for processing singleton references: `#my_var`, `#mut my_var`, `#{N} my_var`, `#{N} mut my_var`.

use itertools::Itertools;
use proc_macro2::{Group, Ident, TokenStream, TokenTree};
use syn::punctuated::Punctuated;
use syn::{Expr, Token};

use crate::parse::parse_terminated;

/// A parsed singleton reference token with mutability and optional access group.
#[derive(Clone, Debug)]
pub struct SingletonRefToken {
    /// The variable name being referenced.
    pub ident: Ident,
    /// Whether this is a mutable reference (`#mut var` or `#{N} mut var`).
    pub is_mut: bool,
    /// Optional access group for ordering (`#{N}` prefix).
    pub access_group: Option<u32>,
}

/// Finds all the singleton references and appends them to `found`. Returns the
/// `TokenStream` but with the `#`, `{N}`, and `mut` removed from the varnames.
///
/// Syntax: `#var`, `#mut var`, `#{N} var`, `#{N} mut var`
///
/// The returned tokens are used for "preflight" parsing, to check that the rest of the syntax is
/// OK. However the returned tokens are not used in the codegen as we need to use [`postprocess_singletons`]
/// later to substitute-in the context referencing code for each singleton
pub fn preprocess_singletons(
    tokens: TokenStream,
    found: &mut Vec<SingletonRefToken>,
) -> TokenStream {
    process_singletons(tokens, &mut |ref_token| {
        let ident = ref_token.ident.clone();
        found.push(ref_token);
        TokenTree::Ident(ident)
    })
}

/// Replaces singleton references with the code needed to actually get the value inside.
///
/// * `tokens` - The tokens to update singleton references within.
/// * `resolved_exprs` - Token streams that correspond 1:1 and in the same
///   order as the singleton references within `tokens` (found in-order via [`preprocess_singletons`]).
/// * `singleton_ref_tokens` - The parsed singleton ref tokens (for mutability info).
///
/// For shared refs: generates `(*expr)` — an immutable place expression.
/// For mutable refs: generates `(*expr)` — a mutable place expression (expr itself is `&mut`).
pub fn postprocess_singletons(
    tokens: TokenStream,
    resolved_exprs: impl IntoIterator<Item = TokenStream>,
) -> Punctuated<Expr, Token![,]> {
    let mut resolved_exprs_iter = resolved_exprs.into_iter();
    let processed = process_singletons(tokens, &mut |_ref_token| {
        let span = _ref_token.ident.span();
        let expr_tokens = resolved_exprs_iter.next().unwrap();
        // Emit `(*expr)` so consumers get a place expression.
        // For shared refs, expr is `&T` so `(*expr)` is immutable.
        // For mutable refs, expr is `&mut T` so `(*expr)` is mutable.
        let deref_tokens: TokenStream = std::iter::once(TokenTree::Punct(proc_macro2::Punct::new(
            '*',
            proc_macro2::Spacing::Alone,
        )))
        .chain(expr_tokens)
        .collect();
        let mut group = Group::new(proc_macro2::Delimiter::Parenthesis, deref_tokens);
        group.set_span(span);
        TokenTree::Group(group)
    });
    parse_terminated(processed).unwrap()
}

/// Traverse the token stream, applying the `map_singleton_fn` whenever a singleton is found,
/// returning the transformed token stream.
///
/// Parses: `#ident`, `#mut ident`, `#{N} ident`, `#{N} mut ident`
fn process_singletons(
    tokens: TokenStream,
    map_singleton_fn: &mut impl FnMut(SingletonRefToken) -> TokenTree,
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
                    if '#' == punct.as_char() {
                        // Parse optional access group: `{N}`
                        let access_group = if matches!(iter.peek(), Some(TokenTree::Group(g)) if g.delimiter() == proc_macro2::Delimiter::Brace)
                        {
                            let Some(TokenTree::Group(group)) = iter.next() else {
                                unreachable!()
                            };
                            let group_tokens: Vec<TokenTree> =
                                group.stream().into_iter().collect();
                            if let [TokenTree::Literal(lit)] = group_tokens.as_slice() {
                                // Parse the integer literal
                                let lit_str = lit.to_string();
                                Some(lit_str.parse::<u32>().unwrap_or_else(|_| {
                                    panic!("Expected integer in singleton access group, got `{}`", lit_str)
                                }))
                            } else {
                                panic!("Expected single integer in singleton access group `{{N}}`")
                            }
                        } else {
                            None
                        };

                        // Parse optional `mut`
                        let is_mut = if matches!(iter.peek(), Some(TokenTree::Ident(id)) if id == "mut")
                        {
                            iter.next(); // consume `mut`
                            true
                        } else {
                            false
                        };

                        // Parse the ident
                        if matches!(iter.peek(), Some(TokenTree::Ident(_))) {
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
                            (map_singleton_fn)(SingletonRefToken {
                                ident: singleton_ident,
                                is_mut,
                                access_group,
                            })
                        } else {
                            // No ident after `#` (or `#{N}` / `#mut`) — emit the punct as-is.
                            // This shouldn't normally happen in valid DFIR syntax.
                            TokenTree::Punct(punct)
                        }
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
