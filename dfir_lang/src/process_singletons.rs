//! Utility methods for processing singleton references: `#my_var`, `#mut my_var`, `#{N} my_var`, `#{N} mut my_var`.

use proc_macro2::{Group, TokenStream, TokenTree};
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{Expr, Token};

use crate::parse::SingletonRef;

/// Finds all the singleton references and appends them to `found`. Returns the
/// `TokenStream` but with the `#`, `{N}`, and `mut` removed from the varnames.
///
/// Syntax: `#var`, `#mut var`, `#{N} var`, `#{N} mut var`
///
/// The returned tokens are used for "preflight" parsing, to check that the rest of the syntax is
/// OK. However the returned tokens are not used in the codegen as we need to use [`postprocess_singletons`]
/// later to substitute-in the context referencing code for each singleton
pub fn preprocess_singletons(tokens: TokenStream, found: &mut Vec<SingletonRef>) -> TokenStream {
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
    Punctuated::parse_terminated.parse2(processed).unwrap()
}

/// Traverse the token stream, applying the `map_singleton_fn` whenever a singleton is found,
/// returning the transformed token stream.
///
/// Parses: `#ident`, `#mut ident`, `#{N} ident`, `#{N} mut ident`
fn process_singletons(
    tokens: TokenStream,
    map_singleton_fn: &mut impl FnMut(SingletonRef) -> TokenTree,
) -> TokenStream {
    let mut iter = tokens.into_iter().peekable();
    std::iter::from_fn(|| {
        let out = match iter.peek()? {
            TokenTree::Group(group) => {
                let mut new_group = Group::new(
                    group.delimiter(),
                    process_singletons(group.stream(), map_singleton_fn),
                );
                new_group.set_span(group.span());

                let _ = iter.next().unwrap(); // Advance past the `peek`ed group.
                TokenTree::Group(new_group)
            }
            TokenTree::Punct(punct) if '#' == punct.as_char() => {
                let tokens = iter.by_ref().collect::<TokenStream>();
                let (opt_singleton, tokens_rest) = SingletonRef::try_parse
                    .parse2(tokens)
                    .expect("bug: should be infallible");
                iter = tokens_rest.into_iter().peekable();
                if let Some(singleton) = opt_singleton {
                    (map_singleton_fn)(singleton)
                } else {
                    iter.next().unwrap()
                }
            }
            _ => iter.next().unwrap(),
        };
        Some(out)
    })
    .collect()
}
