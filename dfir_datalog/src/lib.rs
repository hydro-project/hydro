use dfir_datalog_core::diagnostic::Diagnostic;
use dfir_datalog_core::{dfir_graph_to_program, gen_dfir_graph};
use proc_macro2::Span;
use quote::{quote, ToTokens};

/// Generate a graph instance from [Datalog](https://en.wikipedia.org/wiki/Datalog) code.
///
/// This uses a variant of Datalog that is similar to [Dedalus](https://www2.eecs.berkeley.edu/Pubs/TechRpts/2009/EECS-2009-173.pdf).
///
/// For examples, see [the datalog tests in the repo](https://github.com/hydro-project/hydro/blob/main/dfir_rs/tests/datalog_frontend.rs).
// TODO(mingwei): rustdoc examples inline.
#[proc_macro]
pub fn datalog(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let item = proc_macro2::TokenStream::from(item);
    let literal: proc_macro2::Literal = syn::parse_quote! {
        #item
    };

    const DFIR_CRATE: &str = "dfir_rs";

    let dfir_crate = proc_macro_crate::crate_name(DFIR_CRATE)
        .unwrap_or_else(|_| panic!("`{}` should be present in `Cargo.toml`", DFIR_CRATE));
    let root = match dfir_crate {
        proc_macro_crate::FoundCrate::Itself => {
            let ident = syn::Ident::new(DFIR_CRATE, Span::call_site());
            quote! { #ident }
        }
        proc_macro_crate::FoundCrate::Name(name) => {
            let ident = syn::Ident::new(&name, Span::call_site());
            quote! { #ident }
        }
    };

    match gen_dfir_graph(literal) {
        Ok(graph) => {
            let program = dfir_graph_to_program(graph, root);
            program.to_token_stream().into()
        }
        Err(diagnostics) => {
            let diagnostic_tokens = Diagnostic::try_emit_all(diagnostics.iter())
                .err()
                .unwrap_or_default();
            proc_macro::TokenStream::from(quote! {
                {
                    #diagnostic_tokens
                    dfir_rs::scheduled::graph::Dfir::new()
                }
            })
        }
    }
}
