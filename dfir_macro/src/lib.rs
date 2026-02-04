#![cfg_attr(
    nightly,
    feature(proc_macro_diagnostic, proc_macro_span, proc_macro_def_site)
)]

use dfir_lang::diagnostic::Level;
use dfir_lang::graph::{
    BuildDfirCodeOutput, FlatGraphBuilder, FlatGraphBuilderOutput, build_dfir_code, partition_graph,
};
use dfir_lang::parse::DfirCode;
use proc_macro2::{Ident, Literal, Span};
use quote::{format_ident, quote, quote_spanned};
use syn::{
    Attribute, Fields, GenericParam, ItemEnum, Variant, WherePredicate, parse_macro_input,
    parse_quote,
};

/// Create a runnable graph instance using DFIR's custom syntax.
///
/// For example usage, take a look at the [`surface_*` tests in the `tests` folder](https://github.com/hydro-project/hydro/tree/main/dfir_rs/tests)
/// or the [`examples` folder](https://github.com/hydro-project/hydro/tree/main/dfir_rs/examples)
/// in the [Hydro repo](https://github.com/hydro-project/hydro).
// TODO(mingwei): rustdoc examples inline.
#[proc_macro]
pub fn dfir_syntax(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    dfir_syntax_internal(input, Some(Level::Help))
}

/// [`dfir_syntax!`] but will not emit any diagnostics (errors, warnings, etc.).
///
/// Used for testing, users will want to use [`dfir_syntax!`] instead.
#[proc_macro]
pub fn dfir_syntax_noemit(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    dfir_syntax_internal(input, None)
}

fn root() -> proc_macro2::TokenStream {
    use std::env::{VarError, var as env_var};

    let root_crate_name = format!(
        "{}_rs",
        env!("CARGO_PKG_NAME").strip_suffix("_macro").unwrap()
    );
    let root_crate_ident = root_crate_name.replace('-', "_");
    let root_crate = proc_macro_crate::crate_name(&root_crate_name)
        .unwrap_or_else(|_| panic!("{root_crate_name} should be present in `Cargo.toml`"));
    match root_crate {
        proc_macro_crate::FoundCrate::Itself => {
            if Err(VarError::NotPresent) == env_var("CARGO_BIN_NAME")
                && Err(VarError::NotPresent) != env_var("CARGO_PRIMARY_PACKAGE")
                && Ok(&*root_crate_ident) == env_var("CARGO_CRATE_NAME").as_deref()
            {
                // In the crate itself, including unit tests.
                quote! { crate }
            } else {
                // In an integration test, example, bench, etc.
                let ident: Ident = Ident::new(&root_crate_ident, Span::call_site());
                quote! { ::#ident }
            }
        }
        proc_macro_crate::FoundCrate::Name(name) => {
            let ident = Ident::new(&name, Span::call_site());
            quote! { ::#ident }
        }
    }
}

fn dfir_syntax_internal(
    input: proc_macro::TokenStream,
    retain_diagnostic_level: Option<Level>,
) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DfirCode);
    let root = root();

    let (code, mut diagnostics) = match build_dfir_code(input, &root) {
        Ok(BuildDfirCodeOutput {
            partitioned_graph: _,
            code,
            diagnostics,
        }) => (code, diagnostics),
        Err(diagnostics) => (quote! { #root::scheduled::graph::Dfir::new() }, diagnostics),
    };

    let diagnostic_tokens = retain_diagnostic_level.and_then(|level| {
        diagnostics.retain_level(level);
        diagnostics.try_emit_all().err()
    });

    quote! {
        {
            #diagnostic_tokens
            #code
        }
    }
    .into()
}

/// Parse DFIR syntax without emitting code.
///
/// Used for testing, users will want to use [`dfir_syntax!`] instead.
#[proc_macro]
pub fn dfir_parser(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DfirCode);

    let flat_graph_builder = FlatGraphBuilder::from_dfir(input);
    let err_diagnostics = 'err: {
        let (mut flat_graph, mut diagnostics) = match flat_graph_builder.build() {
            Ok(FlatGraphBuilderOutput {
                flat_graph,
                uses: _,
                diagnostics,
            }) => (flat_graph, diagnostics),
            Err(diagnostics) => {
                break 'err diagnostics;
            }
        };

        if let Err(diagnostic) = flat_graph.merge_modules() {
            diagnostics.push(diagnostic);
            break 'err diagnostics;
        }

        let flat_mermaid = flat_graph.mermaid_string_flat();

        let part_graph = partition_graph(flat_graph).unwrap();
        let part_mermaid = part_graph.to_mermaid(&Default::default());

        let lit0 = Literal::string(&flat_mermaid);
        let lit1 = Literal::string(&part_mermaid);

        return quote! {
            {
                println!("{}\n\n{}\n", #lit0, #lit1);
            }
        }
        .into();
    };

    return err_diagnostics
        .try_emit_all()
        .err()
        .unwrap_or_default()
        .into();
}

fn wrap_localset(item: proc_macro::TokenStream, attribute: Attribute) -> proc_macro::TokenStream {
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

/// Checks that the given closure is a morphism. For now does nothing.
#[proc_macro]
pub fn morphism(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // TODO(mingwei): some sort of code analysis?
    item
}

/// Checks that the given closure is a monotonic function. For now does nothing.
#[proc_macro]
pub fn monotonic_fn(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // TODO(mingwei): some sort of code analysis?
    item
}

#[proc_macro_attribute]
pub fn dfir_test(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let root = root();
    let args_2: proc_macro2::TokenStream = args.into();

    wrap_localset(
        item,
        parse_quote!(
            #[#root::tokio::test(flavor = "current_thread", #args_2)]
        ),
    )
}

#[proc_macro_attribute]
pub fn dfir_main(
    _: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let root = root();

    wrap_localset(
        item,
        parse_quote!(
            #[#root::tokio::main(flavor = "current_thread")]
        ),
    )
}

#[proc_macro_derive(DemuxEnum)]
pub fn derive_demux_enum(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let root = root();

    let ItemEnum {
        ident: item_ident,
        generics,
        variants,
        ..
    } = parse_macro_input!(item as ItemEnum);

    // Sort variants alphabetically.
    let mut variants = variants.into_iter().collect::<Vec<_>>();
    variants.sort_by(|a, b| a.ident.cmp(&b.ident));

    // Return type for each variant.
    let variant_output_types = variants
        .iter()
        .map(|variant| match &variant.fields {
            Fields::Named(fields) => {
                let field_types = fields.named.iter().map(|field| &field.ty);
                quote! {
                    ( #( #field_types, )* )
                }
            }
            Fields::Unnamed(fields) => {
                let field_types = fields.unnamed.iter().map(|field| &field.ty);
                quote! {
                    ( #( #field_types, )* )
                }
            }
            Fields::Unit => quote!(()),
        })
        .collect::<Vec<_>>();

    let variant_generics_sink = variants
        .iter()
        .map(|variant| format_ident!("__Sink{}", variant.ident))
        .collect::<Vec<_>>();
    let variant_generics_pinned_sink = variant_generics_sink.iter().map(|ident| {
        quote_spanned! {ident.span()=>
            ::std::pin::Pin::<&mut #ident>
        }
    });
    let variant_generics_pinned_sink_all = quote! {
        ( #( #variant_generics_pinned_sink, )* )
    };
    let variant_localvars_sink = variants
        .iter()
        .map(|variant| {
            format_ident!(
                "__sink_{}",
                variant.ident.to_string().to_lowercase(),
                span = variant.ident.span()
            )
        })
        .collect::<Vec<_>>();

    let mut full_generics_sink = generics.clone();
    full_generics_sink.params.extend(
        variant_generics_sink
            .iter()
            .map::<GenericParam, _>(|ident| parse_quote!(#ident)),
    );
    full_generics_sink.make_where_clause().predicates.extend(
        variant_generics_sink
            .iter()
            .zip(variant_output_types.iter())
            .map::<WherePredicate, _>(|(sink_generic, output_type)| {
                parse_quote! {
                    // TODO(mingwei): generic error types?
                    #sink_generic: #root::futures::sink::Sink<#output_type, Error = #root::Never>
                }
            }),
    );

    let variant_pats_sink_start_send =
        variants
            .iter()
            .zip(variant_localvars_sink.iter())
            .map(|(variant, sinkvar)| {
                let Variant { ident, fields, .. } = variant;
                let (fields_pat, push_item) = field_pattern_item(fields);
                quote! {
                    Self::#ident #fields_pat => #sinkvar.as_mut().start_send(#push_item)
                }
            });

    let (impl_generics_item, ty_generics, where_clause_item) = generics.split_for_impl();
    let (impl_generics_sink, _ty_generics_sink, where_clause_sink) =
        full_generics_sink.split_for_impl();

    let single_impl = (1 == variants.len()).then(|| {
        let Variant { ident, fields, .. } = variants.first().unwrap();
        let (fields_pat, push_item) = field_pattern_item(fields);
        let out_type = variant_output_types.first().unwrap();
        quote! {
            impl #impl_generics_item #root::util::demux_enum::SingleVariant
                for #item_ident #ty_generics #where_clause_item
            {
                type Output = #out_type;
                fn single_variant(self) -> Self::Output {
                    match self {
                        Self::#ident #fields_pat => #push_item,
                    }
                }
            }
        }
    });

    quote! {
        impl #impl_generics_sink #root::util::demux_enum::DemuxEnumSink<#variant_generics_pinned_sink_all>
            for #item_ident #ty_generics #where_clause_sink
        {
            type Error = #root::Never;

            fn poll_ready(
                ( #( #variant_localvars_sink, )* ): &mut #variant_generics_pinned_sink_all,
                __cx: &mut ::std::task::Context<'_>,
            ) -> ::std::task::Poll<::std::result::Result<(), Self::Error>> {
                // Ready all sinks simultaneously.
                #(
                    let #variant_localvars_sink = #variant_localvars_sink.as_mut().poll_ready(__cx)?;
                )*
                #(
                    ::std::task::ready!(#variant_localvars_sink);
                )*
                ::std::task::Poll::Ready(::std::result::Result::Ok(()))
            }

            fn start_send(
                self,
                ( #( #variant_localvars_sink, )* ): &mut #variant_generics_pinned_sink_all,
            ) -> ::std::result::Result<(), Self::Error> {
                match self {
                    #( #variant_pats_sink_start_send, )*
                }
            }

            fn poll_flush(
                ( #( #variant_localvars_sink, )* ): &mut #variant_generics_pinned_sink_all,
                __cx: &mut ::std::task::Context<'_>,
            ) -> ::std::task::Poll<::std::result::Result<(), Self::Error>> {
                // Flush all sinks simultaneously.
                #(
                    let #variant_localvars_sink = #variant_localvars_sink.as_mut().poll_flush(__cx)?;
                )*
                #(
                    ::std::task::ready!(#variant_localvars_sink);
                )*
                ::std::task::Poll::Ready(::std::result::Result::Ok(()))
            }

            fn poll_close(
                ( #( #variant_localvars_sink, )* ): &mut #variant_generics_pinned_sink_all,
                __cx: &mut ::std::task::Context<'_>,
            ) -> ::std::task::Poll<::std::result::Result<(), Self::Error>> {
                // Close all sinks simultaneously.
                #(
                    let #variant_localvars_sink = #variant_localvars_sink.as_mut().poll_close(__cx)?;
                )*
                #(
                    ::std::task::ready!(#variant_localvars_sink);
                )*
                ::std::task::Poll::Ready(::std::result::Result::Ok(()))
            }
        }

        impl #impl_generics_item #root::util::demux_enum::DemuxEnumBase
            for #item_ident #ty_generics #where_clause_item {}

        #single_impl
    }
    .into()
}

/// (fields pattern, push item expr)
fn field_pattern_item(fields: &Fields) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
    let idents = fields
        .iter()
        .enumerate()
        .map(|(i, field)| {
            field
                .ident
                .clone()
                .unwrap_or_else(|| format_ident!("_{}", i))
        })
        .collect::<Vec<_>>();
    let (fields_pat, push_item) = match fields {
        Fields::Named(_) => (quote!( { #( #idents, )* } ), quote!( ( #( #idents, )* ) )),
        Fields::Unnamed(_) => (quote!( ( #( #idents ),* ) ), quote!( ( #( #idents, )* ) )),
        Fields::Unit => (quote!(), quote!(())),
    };
    (fields_pat, push_item)
}
