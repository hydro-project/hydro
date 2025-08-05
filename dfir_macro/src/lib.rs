#![cfg_attr(
    nightly,
    feature(proc_macro_diagnostic, proc_macro_span, proc_macro_def_site)
)]

use dfir_lang::diagnostic::{Diagnostic, Level};
use dfir_lang::graph::{FlatGraphBuilder, build_hfcode, partition_graph};
use dfir_lang::parse::DfirCode;
use proc_macro2::{Ident, Literal, Span};
use quote::{format_ident, quote};
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
    min_diagnostic_level: Option<Level>,
) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DfirCode);
    let root = root();
    let (graph_code_opt, diagnostics) = build_hfcode(input, &root);
    let tokens = graph_code_opt
        .map(|(_graph, code)| code)
        .unwrap_or_else(|| quote! { #root::scheduled::graph::Dfir::new() });

    let diagnostics = diagnostics
        .iter()
        .filter(|diag: &&Diagnostic| Some(diag.level) <= min_diagnostic_level);

    let diagnostic_tokens = Diagnostic::try_emit_all(diagnostics)
        .err()
        .unwrap_or_default();
    quote! {
        {
            #diagnostic_tokens
            #tokens
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
    let (mut flat_graph, _uses, mut diagnostics) = flat_graph_builder.build();
    if !diagnostics.iter().any(Diagnostic::is_error) {
        if let Err(diagnostic) = flat_graph.merge_modules() {
            diagnostics.push(diagnostic);
        } else {
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
        }
    }

    Diagnostic::try_emit_all(diagnostics.iter())
        .err()
        .unwrap_or_default()
        .into()
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
pub fn derive_answer_fn(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
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

    let variant_pusherator_generics = variants
        .iter()
        .map(|variant| format_ident!("__Pusherator{}", variant.ident))
        .collect::<Vec<_>>();
    let variant_pusherator_localvars = variants
        .iter()
        .map(|variant| {
            format_ident!(
                "__pusherator_{}",
                variant.ident.to_string().to_lowercase(),
                span = variant.ident.span()
            )
        })
        .collect::<Vec<_>>();
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

    let mut full_generics = generics.clone();
    full_generics.params.extend(
        variant_pusherator_generics
            .iter()
            .map::<GenericParam, _>(|ident| parse_quote!(#ident)),
    );
    full_generics.make_where_clause().predicates.extend(
        variant_pusherator_generics
            .iter()
            .zip(variant_output_types.iter())
            .map::<WherePredicate, _>(|(pusherator_generic, output_type)| {
                parse_quote! {
                    #pusherator_generic: #root::pusherator::Pusherator<Item = #output_type>
                }
            }),
    );

    let (impl_generics_item, ty_generics, where_clause_item) = generics.split_for_impl();
    let (impl_generics, _ty_generics, where_clause) = full_generics.split_for_impl();

    let variant_pats = variants
        .iter()
        .zip(variant_pusherator_localvars.iter())
        .map(|(variant, pushvar)| {
            let Variant { ident, fields, .. } = variant;
            let (fields_pat, push_item) = field_pattern_item(fields);
            quote! {
                Self::#ident #fields_pat => #pushvar.give(#push_item)
            }
        });

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
        impl #impl_generics #root::util::demux_enum::DemuxEnum<( #( #variant_pusherator_generics, )* )>
            for #item_ident #ty_generics #where_clause
        {
            fn demux_enum(
                self,
                ( #( #variant_pusherator_localvars, )* ):
                    &mut ( #( #variant_pusherator_generics, )* )
            ) {
                match self {
                    #( #variant_pats, )*
                }
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
