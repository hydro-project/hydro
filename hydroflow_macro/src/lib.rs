#![cfg_attr(
    feature = "diagnostics",
    feature(proc_macro_diagnostic, proc_macro_span, proc_macro_def_site)
)]
#![allow(clippy::explicit_auto_deref)]

use hydroflow_lang::diagnostic::{Diagnostic, Level};
use hydroflow_lang::graph::{build_hfcode, partition_graph, FlatGraphBuilder};
use hydroflow_lang::parse::HfCode;
use proc_macro2::{Ident, Literal, Span};
use quote::quote;
use syn::{parse_macro_input, parse_quote, Attribute, LitStr};

/// Create a Hydroflow instance using Hydroflow's custom "surface syntax."
///
/// For example usage, take a look at the [`surface_*` tests in the `tests` folder](https://github.com/hydro-project/hydroflow/tree/main/hydroflow/tests)
/// or the [`examples` folder](https://github.com/hydro-project/hydroflow/tree/main/hydroflow/examples)
/// in the [Hydroflow repo](https://github.com/hydro-project/hydroflow).
// TODO(mingwei): rustdoc examples inline.
#[proc_macro]
pub fn hydroflow_syntax(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    hydroflow_syntax_internal(input, Some(Level::Help))
}

/// [`hydroflow_syntax!`] but will not emit any diagnostics (errors, warnings, etc.).
///
/// Used for testing, users will want to use [`hydroflow_syntax!`] instead.
#[proc_macro]
pub fn hydroflow_syntax_noemit(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    hydroflow_syntax_internal(input, None)
}

fn root() -> proc_macro2::TokenStream {
    use std::env::{var as env_var, VarError};

    let hydroflow_crate = proc_macro_crate::crate_name("hydroflow")
        .expect("hydroflow should be present in `Cargo.toml`");
    match hydroflow_crate {
        proc_macro_crate::FoundCrate::Itself => {
            if Err(VarError::NotPresent) == env_var("CARGO_BIN_NAME")
                && Err(VarError::NotPresent) != env_var("CARGO_PRIMARY_PACKAGE")
                && Ok("hydroflow") == env_var("CARGO_CRATE_NAME").as_deref()
            {
                // In the crate itself, including unit tests.
                quote! { crate }
            } else {
                // In an integration test, example, bench, etc.
                quote! { ::hydroflow }
            }
        }
        proc_macro_crate::FoundCrate::Name(name) => {
            let ident: Ident = Ident::new(&name, Span::call_site());
            quote! { ::#ident }
        }
    }
}

fn hydroflow_syntax_internal(
    input: proc_macro::TokenStream,
    min_diagnostic_level: Option<Level>,
) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as HfCode);
    let root = root();
    let (graph_code_opt, diagnostics) = build_hfcode(input, &root);
    let tokens = graph_code_opt
        .map(|(_graph, code)| code)
        .unwrap_or_else(|| quote! { #root::scheduled::graph::Hydroflow::new() });

    let diagnostics = diagnostics
        .iter()
        .filter(|diag: &&Diagnostic| Some(diag.level) <= min_diagnostic_level);

    #[cfg(feature = "diagnostics")]
    {
        diagnostics.for_each(Diagnostic::emit);
        tokens.into()
    }

    #[cfg(not(feature = "diagnostics"))]
    {
        let diagnostics = diagnostics.map(Diagnostic::to_tokens);
        quote! {
            {
                #(
                    #diagnostics
                )*
                #tokens
            }
        }
        .into()
    }
}

/// Parse Hydroflow "surface syntax" without emitting code.
///
/// Used for testing, users will want to use [`hydroflow_syntax!`] instead.
#[proc_macro]
pub fn hydroflow_parser(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as HfCode);

    let flat_graph_builder = FlatGraphBuilder::from_hfcode(input);
    let (flat_graph, _uses, diagnostics) = flat_graph_builder.build();
    diagnostics.iter().for_each(Diagnostic::emit);
    let flat_mermaid = flat_graph.mermaid_string_flat();

    let part_graph = partition_graph(flat_graph).unwrap();
    let part_mermaid = part_graph.to_mermaid();

    let lit0 = Literal::string(&*flat_mermaid);
    let lit1 = Literal::string(&*part_mermaid);

    quote! { println!("{}\n\n{}\n", #lit0, #lit1); }.into()
}

#[doc(hidden)]
#[proc_macro]
pub fn surface_booktest_operators(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    assert!(input.is_empty(), "Input must be empty");
    let each = hydroflow_lang::graph::ops::OPERATORS.iter().map(|op| {
        let op_ident = Ident::new(op.name, Span::call_site());
        let op_filename = format!("../../docs/docgen/{}.md", op.name);
        let lit_filename = LitStr::new(&*op_filename, Span::call_site());
        quote! {
            #[doc = include_str!(#lit_filename)]
            mod #op_ident {}
        }
    });
    let out = quote! {
        #( #each )*
    };
    out.into()
}

fn hydroflow_wrap(item: proc_macro::TokenStream, attribute: Attribute) -> proc_macro::TokenStream {
    use quote::ToTokens;

    let root = root();

    let mut input: syn::ItemFn = match syn::parse(item) {
        Ok(it) => it,
        Err(e) => return e.into_compile_error().into(),
    };

    let statements = input.block.stmts;

    input.block.stmts = parse_quote!(
        #root::tokio::task::LocalSet::new().run_until(async {
            #( #statements )*
        }).await
    );

    input.attrs.push(attribute);

    input.into_token_stream().into()
}

#[proc_macro_attribute]
pub fn hydroflow_test(
    _: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let root = root();

    hydroflow_wrap(
        item,
        parse_quote!(
            #[#root::tokio::test(flavor = "current_thread")]
        ),
    )
}

#[proc_macro_attribute]
pub fn hydroflow_main(
    _: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let root = root();

    hydroflow_wrap(
        item,
        parse_quote!(
            #[#root::tokio::main(flavor = "current_thread")]
        ),
    )
}
