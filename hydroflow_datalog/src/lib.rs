use hydroflow_datalog_core::{gen_hydroflow_graph, hydroflow_graph_to_program};
use proc_macro2::Span;
use quote::{quote, ToTokens};

#[proc_macro]
pub fn datalog(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let item = proc_macro2::TokenStream::from(item);
    let literal: proc_macro2::Literal = syn::parse_quote! {
        #item
    };

    let hydroflow_crate = proc_macro_crate::crate_name("hydroflow")
        .expect("hydroflow should be present in `Cargo.toml`");
    let root = match hydroflow_crate {
        proc_macro_crate::FoundCrate::Itself => quote! { hydroflow },
        proc_macro_crate::FoundCrate::Name(name) => {
            let ident = syn::Ident::new(&name, Span::call_site());
            quote! { #ident }
        }
    };

    match gen_hydroflow_graph(literal) {
        Ok(graph) => {
            let program = hydroflow_graph_to_program(graph, root);
            program.to_token_stream().into()
        }
        Err(diagnostics) => {
            for diagnostic in diagnostics {
                diagnostic.emit();
            }
            proc_macro::TokenStream::from(quote!(hydroflow::scheduled::graph::Hydroflow::new()))
        }
    }
}
