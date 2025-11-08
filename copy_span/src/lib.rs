use syn::spanned::Spanned;

struct CopySpanInput {
    source: syn::Expr,
    target: proc_macro2::TokenStream,
}

impl syn::parse::Parse for CopySpanInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let source: syn::Expr = input.parse()?;
        input.parse::<syn::Token![,]>()?;
        let target: proc_macro2::TokenStream = input.parse()?;
        Ok(CopySpanInput { source, target })
    }
}

fn recursively_set_span(token: &mut proc_macro2::TokenTree, span: proc_macro2::Span) {
    match token {
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
        _ => {
            token.set_span(span);
        }
    }
}

#[proc_macro]
pub fn copy_span(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let CopySpanInput { source, target } = syn::parse_macro_input!(input as CopySpanInput);

    let mut inner_source = source;
    while let syn::Expr::Group(g) = inner_source {
        inner_source = *g.expr;
    }

    let output = target
        .into_iter()
        .fold(proc_macro2::TokenStream::new(), |mut acc, mut token| {
            recursively_set_span(&mut token, inner_source.span());
            acc.extend(std::iter::once(token));
            acc
        });

    proc_macro::TokenStream::from(output)
}
