use proc_macro2::Ident;
use quote::{ToTokens, quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{PathArguments, PathSegment, Token, Type, TypePath};

use super::{
    OpInstGenerics, OperatorCategory, OperatorConstraints, OperatorInstance, OperatorWriteOutput,
    PortIndexValue, PortListSpec, RANGE_0, RANGE_1, WriteContextArgs,
};
use crate::diagnostic::{Diagnostic, Level};
use crate::graph::change_spans;

/// > Generic Argument: A enum type which has `#[derive(DemuxEnum)]`. Must match the items in the input stream.
///
/// Takes an input stream of enum instances and splits them into their variants.
///
/// ```rustdoc
/// #[derive(DemuxEnum)]
/// enum Shape {
///     Square(f64),
///     Rectangle(f64, f64),
///     Circle { r: f64 },
///     Triangle { w: f64, h: f64 }
/// }
///
/// let mut df = dfir_syntax! {
///     my_demux = source_iter([
///         Shape::Square(9.0),
///         Shape::Rectangle(10.0, 8.0),
///         Shape::Circle { r: 5.0 },
///         Shape::Triangle { w: 12.0, h: 13.0 },
///     ]) -> demux_enum::<Shape>();
///
///     my_demux[Square] -> map(|s| s * s) -> out;
///     my_demux[Circle] -> map(|(r,)| std::f64::consts::PI * r * r) -> out;
///     my_demux[Rectangle] -> map(|(w, h)| w * h) -> out;
///     my_demux[Circle] -> map(|(w, h)| 0.5 * w * h) -> out;
///
///     out = union() -> for_each(|area| println!("Area: {}", area));
/// };
/// df.run_available();
/// ```
pub const DEMUX_ENUM: OperatorConstraints = OperatorConstraints {
    name: "demux_enum",
    categories: &[OperatorCategory::MultiOut],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: &(..),
    soft_range_out: &(..),
    num_args: 0,
    persistence_args: RANGE_0,
    type_args: RANGE_1,
    is_external_input: false,
    has_singleton_output: false,
    flo_type: None,
    ports_inn: None,
    ports_out: Some(|| PortListSpec::Variadic),
    input_delaytype_fn: |_| None,
    write_fn: |wc @ &WriteContextArgs {
                   root,
                   op_span,
                   ident,
                   inputs,
                   outputs,
                   is_pull,
                   op_name,
                   op_inst:
                       OperatorInstance {
                           output_ports,
                           generics: OpInstGenerics { type_args, .. },
                           ..
                       },
                   ..
               },
               diagnostics| {
        let enum_type = &type_args[0];

        // Port idents supplied via port connections in the surface syntax.
        let port_idents: Vec<_> = output_ports
                    .iter()
                    .filter_map(|output_port| {
                        let PortIndexValue::Path(port_expr) = output_port else {
                            diagnostics.push(Diagnostic::spanned(
                                output_port.span(),
                                Level::Error,
                                format!(
                                    "Output port from `{}(..)` must be specified and must be a valid identifier.",
                                    op_name,
                                ),
                            ));
                            return None;
                        };
                        let port_ident = syn::parse2::<Ident>(quote! { #port_expr })
                            .map_err(|err| diagnostics.push(err.into()))
                            .ok()?;

                        Some(port_ident)
                    })
                    .collect();

        // The entire purpose of this closure and match statement is to generate readable error messages:
        // "missing match arm: `Variant(_)` not covered."
        // Or "no variant named `Variant` found for enum `Shape`"
        // Note this uses the `enum_type`'s span.
        let enum_type_turbofish = ensure_turbofish(enum_type);
        let port_variant_check_match_arms = port_idents
            .iter()
            .map(|port_ident| {
                let enum_type_turbofish =
                    change_spans(enum_type_turbofish.to_token_stream(), port_ident.span());
                quote_spanned! {port_ident.span()=>
                    #enum_type_turbofish::#port_ident { .. } => ()
                }
            })
            .collect::<Vec<_>>();
        let root_span = change_spans(root.clone(), enum_type.span());
        let write_prologue = quote_spanned! {enum_type.span()=>
            let _ = |__val: #enum_type| {
                fn check_impl_demux_enum<T: ?Sized + #root_span::util::demux_enum::DemuxEnumBase>(_: &T) {}
                check_impl_demux_enum(&__val);
                match __val {
                    #(
                        #port_variant_check_match_arms,
                    )*
                };
            };
        };

        let write_iterator = if 1 == outputs.len() {
            // Use `enum_type`'s span.
            let map_fn = quote_spanned! {enum_type.span()=>
                <#enum_type as #root::util::demux_enum::SingleVariant>::single_variant
            };
            if is_pull {
                let input = &inputs[0];
                quote_spanned! {op_span=>
                    let #ident = #input.map(#map_fn);
                }
            } else {
                let output = &outputs[0];
                quote_spanned! {op_span=>
                    let #ident = #root::sinktools::map(#map_fn, #output);
                }
            }
        } else {
            assert!(!is_pull);

            let mut sort_permute: Vec<_> = (0..port_idents.len()).collect();
            sort_permute.sort_by_key(|&i| &port_idents[i]);

            let sorted_outputs = sort_permute.iter().map(|&i| &outputs[i]);
            let outputs_ident = wc.make_ident("outputs");

            quote_spanned! {op_span=>
                let #outputs_ident = (
                    #(
                        ::std::pin::pin!( #sorted_outputs ),
                    )*
                );
                let #ident = {
                    fn demux_enum_guard<Item, Outputs>(outputs: Outputs) -> impl #root::futures::sink::Sink<Item, Error = #root::Never>
                    where
                        Item: #root::util::demux_enum::DemuxEnumSink<Outputs, Error = #root::Never>,
                    {
                        #root::compiled::push::DemuxEnum::new::<Item>(outputs)
                    }
                    demux_enum_guard::<#enum_type, _>(#outputs_ident)
                };
            }
        };

        Ok(OperatorWriteOutput {
            write_prologue,
            write_iterator,
            ..Default::default()
        })
    },
};

/// Ensure enum type has double colon turbofish syntax.
/// `my_mod::MyType<MyGeneric>` becomes `my_mod::MyType::<MyGeneric>`.
fn ensure_turbofish(ty: &Type) -> Type {
    let mut ty = ty.clone();
    // If type is path.
    if let Type::Path(TypePath { qself: _, path }) = &mut ty {
        // If path ends in angle bracketed generics.
        if let Some(PathSegment {
            ident: _,
            arguments: PathArguments::AngleBracketed(angle_bracketed),
        }) = path.segments.last_mut()
        {
            // Ensure the final turbofish double-colon is set.
            angle_bracketed.colon2_token = Some(<Token![::]>::default());
        }
    };
    ty
}
