use std::collections::BTreeSet;

use proc_macro2::Span;
use quote::quote_spanned;
use syn::spanned::Spanned;
use syn::token::Colon;
use syn::{parse_quote_spanned, Expr, Ident, LitInt, Pat, PatType};

use super::{
    OperatorCategory, OperatorConstraints, OperatorInstance, OperatorWriteOutput, PortIndexValue,
    PortListSpec, WriteContextArgs, RANGE_0, RANGE_1,
};
use crate::diagnostic::{Diagnostic, Level};
use crate::pretty_span::PrettySpan;

/// This operator takes the input pipeline and allows the user to determine which singular output
/// pipeline each item should be delivered to.
///
/// > Arguments: A Rust closure, the first argument is a reference to the item and the second
/// > argument corresponds to one of two modes, either named or indexed.
///
/// > Note: The closure has access to the [`context` object](surface_flows.mdx#the-context-object).
///
/// # Named mode
/// With named ports, the closure's second argument must be a Rust 'slice pattern' of names, such as
/// `[port_a, port_b, port_c]`, where each name is an output port. The closure should return the
/// name of the desired output port.
///
/// ```dfir
/// my_partition = source_iter(1..=100) -> partition(|val: &usize, [fzbz, fizz, buzz, rest]|
///     match (val % 3, val % 5) {
///         (0, 0) => fzbz,
///         (0, _) => fizz,
///         (_, 0) => buzz,
///         (_, _) => rest,
///     }
/// );
/// my_partition[fzbz] -> for_each(|v| println!("{}: fizzbuzz", v));
/// my_partition[fizz] -> for_each(|v| println!("{}: fizz", v));
/// my_partition[buzz] -> for_each(|v| println!("{}: buzz", v));
/// my_partition[rest] -> for_each(|v| println!("{}", v));
/// ```
///
/// # Indexed mode
/// With indexed mode, the closure's second argument is a the number of output ports. This is a
/// single usize value, useful for e.g. round robin partitioning. Each output pipeline port must be
/// numbered with an index, starting from zero and with no gaps. The closure returns the index of
/// the desired output port.
///
/// ```dfir
/// my_partition = source_iter(1..=100) -> partition(|val, num_outputs| val % num_outputs);
/// my_partition[0] -> for_each(|v| println!("0: {}", v));
/// my_partition[1] -> for_each(|v| println!("1: {}", v));
/// my_partition[2] -> for_each(|v| println!("2: {}", v));
/// ```
pub const PARTITION: OperatorConstraints = OperatorConstraints {
    name: "partition",
    categories: &[OperatorCategory::MultiOut],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: &(2..),
    soft_range_out: &(2..),
    num_args: 1,
    persistence_args: RANGE_0,
    type_args: RANGE_0,
    is_external_input: false,
    has_singleton_output: false,
    flo_type: None,
    ports_inn: None,
    ports_out: Some(|| PortListSpec::Variadic),
    input_delaytype_fn: |_| None,
    write_fn: |&WriteContextArgs {
                   root,
                   op_span,
                   ident,
                   outputs,
                   is_pull,
                   op_name,
                   op_inst: OperatorInstance { output_ports, .. },
                   arguments,
                   ..
               },
               diagnostics| {
        assert!(!is_pull);

        // Clone because we may modify the closure's arg2 to inject the type.
        let mut func = arguments[0].clone();

        let idx_ints = (0..output_ports.len())
            .map(|i| LitInt::new(&format!("{}_usize", i), op_span))
            .collect::<Vec<_>>();

        let mut output_sort_permutation: Vec<_> = (0..outputs.len()).collect();
        let arg2_val = if let Some(port_idents) =
            determine_indices_or_idents(output_ports, op_span, op_name, diagnostics)?
        {
            // All idents.
            let (closure_idents, arg2_span) =
                extract_closure_idents(&mut func, op_name).map_err(|err| diagnostics.push(err))?;
            check_closure_ports_match(
                &closure_idents,
                &port_idents,
                op_name,
                arg2_span,
                diagnostics,
            )?;
            output_sort_permutation.sort_by_key(|&i| {
                closure_idents
                    .iter()
                    .position(|ident| ident == &port_idents[i])
                    .expect(
                        "Missing port, this should've been caught in the check above, this is a bug.",
                    )
            });
            let arg2_val = quote_spanned! {arg2_span.span()=> [ #( #idx_ints ),* ] };

            arg2_val
        } else {
            // All indices.
            let len_lit = LitInt::new(&format!("{}_usize", output_ports.len()), op_span);
            let arg2_val = quote_spanned! {op_span=> #len_lit };
            arg2_val
        };

        let sorted_outputs = output_sort_permutation.into_iter().map(|i| &outputs[i]);

        let write_iterator = quote_spanned! {op_span=>
            let #ident = #root::sinktools::map(
                |__item| {
                    #[allow(clippy::redundant_closure_call)]
                    let __idx = (#func)(&__item, #arg2_val);
                    (__idx, __item)
                },
                #root::sinktools::demux_var(
                    #root::var_expr!( #( #sorted_outputs ),* ),
                ),
            );
        };

        Ok(OperatorWriteOutput {
            write_iterator,
            ..Default::default()
        })
    },
};

/// Returns `Ok(Some(idents))` if ports are idents, or `Ok(None)` if ports are indices.
/// Returns `Err(())` if there are any errors (pushed to `diagnostics`).
fn determine_indices_or_idents(
    output_ports: &[PortIndexValue],
    op_span: Span,
    op_name: &'static str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Option<Vec<Ident>>, ()> {
    // Port idents supplied via port connections in the surface syntax.
    // Two modes, either all numeric `0, 1, 2, 3, ...` or all `Ident`s.
    // If ports are `Idents` then the closure's 2nd argument, input array must have named
    // values corresponding to the port idents.
    let mut ports_numeric = BTreeSet::new();
    let mut ports_idents = Vec::new();
    // If any ports are elided we return `Err(())` early.
    let mut err_elided = false;
    for output_port in output_ports {
        match output_port {
            PortIndexValue::Elided(port_span) => {
                err_elided = true;
                diagnostics.push(Diagnostic::spanned(
                    port_span.unwrap_or(op_span),
                    Level::Error,
                    format!(
                        "Output ports from `{}` cannot be blank, must be named or indexed.",
                        op_name
                    ),
                ));
            }
            PortIndexValue::Int(port_idx) => {
                ports_numeric.insert(port_idx);

                if port_idx.value < 0 {
                    diagnostics.push(Diagnostic::spanned(
                        port_idx.span,
                        Level::Error,
                        format!("Output ports from `{}` must be non-nonegative indices starting from zero.", op_name),
                    ));
                }
            }
            PortIndexValue::Path(port_path) => {
                let port_ident = syn::parse2::<Ident>(quote_spanned!(op_span=> #port_path))
                    .map_err(|err| diagnostics.push(err.into()))?;
                ports_idents.push(port_ident);
            }
        }
    }
    if err_elided {
        return Err(());
    }

    match (!ports_numeric.is_empty(), !ports_idents.is_empty()) {
        (false, false) => {
            // Had no ports or only elided ports.
            assert!(diagnostics.iter().any(Diagnostic::is_error), "Empty input ports, expected an error diagnostic but none were emitted, this is a bug.");
            Err(())
        }
        (true, true) => {
            // Conflict.
            let msg = &*format!(
                "Output ports from `{}` must either be all integer indices or all identifiers.",
                op_name
            );
            diagnostics.extend(
                output_ports
                    .iter()
                    .map(|output_port| Diagnostic::spanned(output_port.span(), Level::Error, msg)),
            );
            Err(())
        }
        (true, false) => {
            let max_port_idx = ports_numeric.last().unwrap().value;
            if usize::try_from(max_port_idx).unwrap() >= ports_numeric.len() {
                let mut expected = 0;
                for port_numeric in ports_numeric {
                    if expected != port_numeric.value {
                        diagnostics.push(Diagnostic::spanned(
                            port_numeric.span,
                            Level::Error,
                            format!(
                                "Output port indices from `{}` must be consecutive from zero, missing {}.",
                                op_name, expected
                            ),
                        ));
                    }
                    expected = port_numeric.value + 1;
                }
                // Can continue with code gen, port numbers will be treated as if they're
                // consecutive from their ascending order.
            }
            Ok(None)
        }
        (false, true) => Ok(Some(ports_idents)),
    }
}

// Returns a vec of closure idents and the arg2 span.
fn extract_closure_idents(
    func: &mut Expr,
    op_name: &'static str,
) -> Result<(Vec<Ident>, Span), Diagnostic> {
    let Expr::Closure(func) = func else {
        return Err(Diagnostic::spanned(
            func.span(),
            Level::Error,
            "Argument must be a two-argument closure expression",
        ));
    };
    if 2 != func.inputs.len() {
        return Err(Diagnostic::spanned(
            func.inputs.span(),
            Level::Error,
            &*format!(
                "Closure provided to `{}(..)` must have two arguments: \
                the first argument is the item, and for named ports the second argument must contain a Rust 'slice pattern' to determine the port names and order. \
                For example, the second argument could be `[foo, bar, baz]` for ports `foo`, `bar`, and `baz`.",
                op_name
            ),
        ));
    }

    // Port idents specified in the closure's second argument.
    let mut arg2 = &mut func.inputs[1];
    let mut already_has_type = false;
    if let Pat::Type(pat_type) = arg2 {
        arg2 = &mut *pat_type.pat;
        already_has_type = true;
    }

    let arg2_span = arg2.span();
    if let Pat::Ident(pat_ident) = arg2 {
        arg2 = &mut *pat_ident
            .subpat
            .as_mut()
            .ok_or_else(|| Diagnostic::spanned(
                arg2_span,
                Level::Error,
                format!(
                    "Second argument for the `{}` closure must contain a Rust 'slice pattern' to determine the port names and order. \
                    For example: `arr @ [foo, bar, baz]` for ports `foo`, `bar`, and `baz`.",
                    op_name
                )
            ))?
            .1;
    }
    let Pat::Slice(pat_slice) = arg2 else {
        return Err(Diagnostic::spanned(
            arg2_span,
            Level::Error,
            format!(
                "Second argument for the `{}` closure must have a Rust 'slice pattern' to determine the port names and order. \
                For example: `[foo, bar, baz]` for ports `foo`, `bar`, and `baz`.",
                op_name
            )
        ));
    };

    let idents = pat_slice
        .elems
        .iter()
        .map(|pat| {
            let Pat::Ident(pat_ident) = pat else {
                panic!("TODO(mingwei) expected ident pat");
            };
            pat_ident.ident.clone()
        })
        .collect();

    // Last step: set the type `[a, b, c]: [usize; 3]` if it is not already specified.
    if !already_has_type {
        let len = LitInt::new(&pat_slice.elems.len().to_string(), arg2_span);
        *arg2 = Pat::Type(PatType {
            attrs: vec![],
            pat: Box::new(arg2.clone()),
            colon_token: Colon { spans: [arg2_span] },
            ty: parse_quote_spanned! {arg2_span=> [usize; #len] },
        });
    }

    Ok((idents, arg2_span))
}

// Checks that the closure names and output port names match.
fn check_closure_ports_match(
    closure_idents: &[Ident],
    port_idents: &[Ident],
    op_name: &'static str,
    arg2_span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<(), ()> {
    let mut err = false;
    for port_ident in port_idents {
        if !closure_idents.contains(port_ident) {
            // An output port is missing from the closure args.
            err = true;
            diagnostics.push(Diagnostic::spanned(
                arg2_span,
                Level::Error,
                format!(
                    "Argument specifying the output ports in `{0}(..)` does not contain extra port `{1}`: ({2}) (1/2).",
                    op_name, port_ident, PrettySpan(port_ident.span()),
                ),
            ));
            diagnostics.push(Diagnostic::spanned(
                port_ident.span(),
                Level::Error,
                format!(
                    "Port `{1}` not found in the arguments specified in `{0}(..)`'s closure: ({2}) (2/2).",
                    op_name, port_ident, PrettySpan(arg2_span),
                ),
            ));
        }
    }
    for closure_ident in closure_idents {
        if !port_idents.contains(closure_ident) {
            // A closure arg is missing from the output ports.
            err = true;
            diagnostics.push(Diagnostic::spanned(
                closure_ident.span(),
                Level::Error,
                format!(
                    "`{}(..)` closure argument `{}` missing corresponding output port.",
                    op_name, closure_ident,
                ),
            ));
        }
    }
    (!err).then_some(()).ok_or(())
}
