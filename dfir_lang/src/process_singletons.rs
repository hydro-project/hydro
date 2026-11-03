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
    process_singletons(tokens, &mut |ref_token, _immediate_borrow| {
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
///
/// For shared refs: splices the resolved expression directly (e.g. `&T` or `&Option<T>`).
/// For mutable refs: splices the resolved expression directly (e.g. `&mut T` or `&mut Option<T>`).
///
/// When the reference is the direct operand of `&` or `&mut` (as in
/// `(&#name).as_reveal_ref()`), the resolved expression is dereferenced first.
/// Those expressions are temporary references (`Option::as_ref` / `as_mut`), so
/// borrowing them directly dies at the end of the statement. `*` reborrows the
/// value inside the buffer instead.
pub fn postprocess_singletons(
    tokens: TokenStream,
    resolved_exprs: impl IntoIterator<Item = TokenStream>,
) -> Punctuated<Expr, Token![,]> {
    let mut resolved_exprs_iter = resolved_exprs.into_iter();
    let processed = process_singletons(tokens, &mut |_ref_token, immediate_borrow| {
        let span = _ref_token.ident.span();
        let expr_tokens = resolved_exprs_iter.next().unwrap();
        let mut inner = TokenStream::new();
        if immediate_borrow {
            let mut star = proc_macro2::Punct::new('*', proc_macro2::Spacing::Alone);
            star.set_span(span);
            inner.extend([TokenTree::Punct(star)]);
        }
        inner.extend(expr_tokens);
        // Wrap in parentheses to preserve precedence.
        let mut group = Group::new(proc_macro2::Delimiter::Parenthesis, inner);
        group.set_span(span);
        TokenTree::Group(group)
    });
    Punctuated::parse_terminated.parse2(processed).unwrap()
}

/// Traverse the token stream, applying the `map_singleton_fn` whenever a singleton is found,
/// returning the transformed token stream.
///
/// The bool is true when this reference is the direct operand of a prefix `&` or `&mut`.
/// A following `.`, `?`, call, or index is not direct: those bind tighter than `&`, so
/// `&#name.method()` is `&(name.method())`, not `(&name).method()`.
///
/// Parses: `#ident`, `#mut ident`, `#{N} ident`, `#{N} mut ident`
fn process_singletons(
    tokens: TokenStream,
    map_singleton_fn: &mut impl FnMut(SingletonRef, bool) -> TokenTree,
) -> TokenStream {
    let mut iter = tokens.into_iter().peekable();
    let mut out = TokenStream::new();
    while iter.peek().is_some() {
        match iter.peek().unwrap() {
            TokenTree::Group(group) => {
                let mut new_group = Group::new(
                    group.delimiter(),
                    process_singletons(group.stream(), map_singleton_fn),
                );
                new_group.set_span(group.span());

                let _ = iter.next().unwrap(); // Advance past the `peek`ed group.
                out.extend([TokenTree::Group(new_group)]);
            }
            TokenTree::Punct(punct) if '#' == punct.as_char() => {
                let tokens = iter.by_ref().collect::<TokenStream>();
                let (opt_singleton, tokens_rest) = SingletonRef::try_parse
                    .parse2(tokens)
                    .expect("bug: should be infallible");
                let immediate_borrow = opt_singleton.is_some()
                    && prefix_borrow_applies(&out)
                    && !followed_by_tighter_suffix(&tokens_rest);
                iter = tokens_rest.into_iter().peekable();
                if let Some(singleton) = opt_singleton {
                    out.extend([map_singleton_fn(singleton, immediate_borrow)]);
                } else if let Some(tt) = iter.next() {
                    out.extend([tt]);
                }
            }
            _ => {
                out.extend([iter.next().unwrap()]);
            }
        }
    }
    out
}

/// True when `emitted` ends with a prefix `&` or `&mut` (not bitwise `&`).
fn prefix_borrow_applies(emitted: &TokenStream) -> bool {
    let tokens: Vec<TokenTree> = emitted.clone().into_iter().collect();
    let mut end = tokens.len();
    if matches!(tokens.last(), Some(TokenTree::Ident(ident)) if ident == "mut") {
        end = end.saturating_sub(1);
    }
    if end == 0 {
        return false;
    }
    let Some(TokenTree::Punct(amp)) = tokens.get(end - 1) else {
        return false;
    };
    if amp.as_char() != '&' {
        return false;
    }
    end == 1 || !is_value_token(&tokens[end - 2])
}

/// Tokens that complete a value expression, so a following `&` is bitwise AND.
fn is_value_token(tt: &TokenTree) -> bool {
    match tt {
        TokenTree::Literal(_) | TokenTree::Group(_) => true,
        TokenTree::Ident(ident) => {
            !(ident == "return"
                || ident == "break"
                || ident == "yield"
                || ident == "if"
                || ident == "else"
                || ident == "while"
                || ident == "loop"
                || ident == "match"
                || ident == "in")
        }
        TokenTree::Punct(_) => false,
    }
}

/// `.`, `?`, calls, and indexing bind tighter than prefix `&` / `&mut`.
fn followed_by_tighter_suffix(tokens_rest: &TokenStream) -> bool {
    match tokens_rest.clone().into_iter().next() {
        Some(TokenTree::Punct(punct)) if matches!(punct.as_char(), '.' | '?') => true,
        Some(TokenTree::Group(group))
            if matches!(
                group.delimiter(),
                proc_macro2::Delimiter::Parenthesis | proc_macro2::Delimiter::Bracket
            ) =>
        {
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use proc_macro2::{Punct, Spacing};
    use quote::{ToTokens, quote};

    use super::*;

    fn rewrite(input: TokenStream) -> String {
        let resolved = std::iter::repeat_with(|| quote!(hoff.as_mut().unwrap()));
        postprocess_singletons(input, resolved)
            .into_iter()
            .map(|expr| expr.to_token_stream().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }

    #[test]
    fn immediate_borrow_derefs_the_temporary_reference() {
        let hash = Punct::new('#', Spacing::Alone);
        let shared = rewrite(quote! { (&#hash namespaces).as_reveal_ref() });
        assert_eq!(
            shared,
            "(& (* hoff . as_mut () . unwrap ())) . as_reveal_ref ()"
        );

        let mutable = rewrite(quote! { (&mut #hash mut namespaces).as_reveal_mut() });
        assert_eq!(
            mutable,
            "(& mut (* hoff . as_mut () . unwrap ())) . as_reveal_mut ()"
        );
    }

    #[test]
    fn method_call_keeps_reference_temporary() {
        let hash = Punct::new('#', Spacing::Alone);
        // `.` binds tighter than `&`, so this must not deref the state place.
        let got = rewrite(quote! { &#hash namespaces.as_reveal_ref() });
        assert_eq!(got, "& (hoff . as_mut () . unwrap ()) . as_reveal_ref ()");
    }

    #[test]
    fn plain_use_is_not_an_extra_deref() {
        let hash = Punct::new('#', Spacing::Alone);
        let got = rewrite(quote! { #hash namespaces.as_reveal_ref() });
        assert_eq!(got, "(hoff . as_mut () . unwrap ()) . as_reveal_ref ()");
    }
}
