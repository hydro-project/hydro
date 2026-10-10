use quote::ToTokens;
use syn::spanned::Spanned;

struct CopySpanInput {
    sources: Vec<syn::Expr>,
    target: proc_macro2::TokenStream,
}

impl syn::parse::Parse for CopySpanInput {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
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

    let combined_span = sources
        .into_iter()
        .map(|mut inner_source| {
            while let syn::Expr::Group(g) = inner_source {
                inner_source = *g.expr;
            }
            inner_source.span()
        })
        .reduce(|a, b| a.join(b).unwrap_or(a))
        .expect("bug: `sources` was empty");

    let output = target
        .into_iter()
        .fold(proc_macro2::TokenStream::new(), |mut acc, mut token| {
            recursively_set_span(&mut token, combined_span);
            acc.extend(std::iter::once(token));
            acc
        });

    proc_macro::TokenStream::from(output)
}

struct SpanLocationInput {
    sources: Vec<syn::Expr>,
}

impl syn::parse::Parse for SpanLocationInput {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let mut sources = vec![];
        while !input.is_empty() {
            sources.push(input.parse::<syn::Expr>()?);
            if input.parse::<syn::Token![,]>().is_err() {
                break;
            }
        }
        Ok(SpanLocationInput { sources })
    }
}

/// Expands to the source position of the first expression given, as a reference to the tuple
/// `(file, line, column): &'static (Option<&'static str>, u32, u32)`, evaluated at macro
/// expansion time. The tuple is a constant, so the reference is promoted to `'static` and the
/// position costs one pointer wherever it is stored.
///
/// `file` is the path of the source file as the compiler saw it (`None` when the span has no
/// local file, for example under some IDE proc-macro servers), and `line` and `column` are
/// one-based. The position is that of the expression's first token, which for a call such as
/// `batch(x, y)` is the `batch` identifier. With no expressions, or when the expression's span
/// carries no position, the macro expands to `&(None, 0, 0)`.
///
/// This exists because a runtime backtrace cannot recover this position: rustc collapses the
/// debug-info location of code produced by an external macro to that macro's call site, so a
/// backtrace captured inside a `macro_rules!` expansion names the macro invocation, not the
/// user's token inside it. Reading the span while the macro expands sidesteps that.
#[proc_macro]
pub fn span_location(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let SpanLocationInput { sources } = syn::parse_macro_input!(input as SpanLocationInput);

    let position = sources.into_iter().next().and_then(|mut source| {
        while let syn::Expr::Group(g) = source {
            source = *g.expr;
        }
        if !proc_macro::is_available() {
            return None;
        }
        let span = source.span().unwrap();
        // Debug info records absolute paths, so make this one absolute too; the compiler's
        // working directory is where the relative path it gives is rooted.
        let file = span.local_file().map(|p| {
            if p.is_absolute() {
                p
            } else {
                std::env::current_dir().map(|d| d.join(&p)).unwrap_or(p)
            }
        });
        Some((
            file.map(|p| p.display().to_string()),
            span.line() as u32,
            span.column() as u32,
        ))
    });

    let output = match position {
        Some((Some(file), line, column)) => quote::quote! {
            &(::core::option::Option::Some(#file), #line, #column)
        },
        Some((None, line, column)) => quote::quote! {
            &(::core::option::Option::None, #line, #column)
        },
        None => quote::quote! {
            &(::core::option::Option::None, 0u32, 0u32)
        },
    };

    proc_macro::TokenStream::from(output)
}
