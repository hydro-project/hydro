use quote::ToTokens;
use syn::spanned::Spanned;

struct CopySpanInput {
    sources: Vec<syn::Expr>,
    target: proc_macro2::TokenStream,
}

impl syn::parse::Parse for CopySpanInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut sources = vec![];
        loop {
            let next_source: syn::Expr = input.parse()?;

            if input.parse::<syn::Token![,]>().is_ok() {
                sources.push(next_source);
            } else {
                return Ok(CopySpanInput {
                    sources,
                    target: next_source.to_token_stream(),
                });
            }
        }
    }
}

fn recursively_set_span(token: &mut proc_macro2::TokenTree, span: proc_macro2::Span) {
    match token {
        proc_macro2::TokenTree::Group(group)
            if group.delimiter() == proc_macro2::Delimiter::None =>
        {
            // None-delimited groups wrap interpolated metavariable fragments (e.g. `$arg:expr`
            // passed through a `macro_rules!` transcriber). Leave them untouched so that the
            // fragment's original spans are preserved, both for precise error attribution
            // within the fragment and to keep the hygiene of its tokens intact.
        }
        proc_macro2::TokenTree::Group(group) => {
            let new_stream = group
                .stream()
                .into_iter()
                .map(|mut inner_token| {
                    recursively_set_span(&mut inner_token, span);
                    inner_token
                })
                .collect();

            let mut new_group = proc_macro2::Group::new(group.delimiter(), new_stream);
            new_group.set_span(span);
            *group = new_group;
        }
        proc_macro2::TokenTree::Ident(_) => {
            // Move the ident's location to the target span, but keep its original
            // resolution context so that hygiene (e.g. for `$crate` or local variables
            // introduced by a `macro_rules!` expansion) is preserved.
            token.set_span(span.resolved_at(token.span()));
        }
        _ => {
            token.set_span(span);
        }
    }
}

#[proc_macro]
pub fn copy_span(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let CopySpanInput { sources, target } = syn::parse_macro_input!(input as CopySpanInput);

    let mut combined_span = None;
    for mut inner_source in sources {
        while let syn::Expr::Group(g) = inner_source {
            inner_source = *g.expr;
        }

        if combined_span.is_none() {
            combined_span = Some(inner_source.span());
        } else {
            combined_span = Some(
                combined_span
                    .unwrap()
                    .join(inner_source.span())
                    .unwrap_or(combined_span.unwrap()),
            );
        }
    }

    let output = target
        .into_iter()
        .fold(proc_macro2::TokenStream::new(), |mut acc, mut token| {
            recursively_set_span(&mut token, combined_span.unwrap());
            acc.extend(std::iter::once(token));
            acc
        });

    proc_macro::TokenStream::from(output)
}
