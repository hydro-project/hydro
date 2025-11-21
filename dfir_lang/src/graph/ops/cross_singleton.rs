use quote::{ToTokens, quote_spanned};
use syn::parse_quote;

use super::{
    DelayType, OperatorCategory, OperatorConstraints, OperatorWriteOutput, RANGE_0, RANGE_1,
    WriteContextArgs,
};
use crate::graph::PortIndexValue;

/// > 2 input streams, 1 output stream, no arguments.
///
/// Operates like cross-join, but treats one of the inputs as a "singleton"-like stream, emitting
/// ignoring everything after the first element. This operator blocks on the singleton input, and
/// then joins it with all the elements in the other stream if an element is present. This operator
/// is useful when a singleton input must be used to transform elements of a stream, since unlike
/// cross-product it avoids cloning the stream of inputs. It is also useful for creating conditional
/// branches, since the operator short circuits if the singleton input produces no values.
///
/// There are two inputs to `cross_singleton`, they are `input` and `single`.
/// `input` is the input data flow, and `single` is the singleton input.
///
/// ```dfir
/// join = cross_singleton();
///
/// source_iter([1, 2, 3]) -> [input]join;
/// source_iter([0]) -> [single]join;
///
/// join -> assert_eq([(1, 0), (2, 0), (3, 0)]);
/// ```
pub const CROSS_SINGLETON: OperatorConstraints = OperatorConstraints {
    name: "cross_singleton",
    categories: &[OperatorCategory::MultiIn],
    persistence_args: RANGE_0,
    type_args: RANGE_0,
    hard_range_inn: &(2..=2),
    soft_range_inn: &(2..=2),
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 0,
    is_external_input: false,
    has_singleton_output: false,
    flo_type: None,
    ports_inn: Some(|| super::PortListSpec::Fixed(parse_quote! { input, single })),
    ports_out: None,
    input_delaytype_fn: |idx| match idx {
        PortIndexValue::Path(path) if "single" == path.to_token_stream().to_string() => {
            Some(DelayType::Stratum)
        }
        _else => None,
    },
    write_fn: |wc @ &WriteContextArgs {
                   root,
                   context,
                   df_ident,
                   ident,
                   op_span,
                   inputs,
                   is_pull,
                   ..
               },
               _diagnostics| {
        assert!(is_pull);

        let item_stream = &inputs[0];
        let singleton_stream = &inputs[1];
        let singleton_handle_ident = wc.make_ident("singleton_handle");
        let singleton_state_ident = wc.make_ident("singleton_state");

        let write_prologue = quote_spanned! {op_span=>
            let #singleton_handle_ident = #df_ident.add_state(
                ::std::cell::RefCell::new(::std::option::Option::None)
            );
            // Reset the value if it is a new tick. TODO(mingwei): handle other lifespans?
            #df_ident.set_state_lifespan_hook(#singleton_handle_ident, #root::scheduled::graph::StateLifespan::Tick, |rcell| { rcell.take(); });
        };

        let write_iterator = quote_spanned! {op_span=>
            let mut #singleton_state_ident = unsafe {
                // SAFETY: handle from `#df_ident.add_state(..)`.
                #context.state_ref_unchecked(#singleton_handle_ident)
            }.borrow_mut();

            let #ident = #root::compiled::pull::CrossSingleton::new(#item_stream, #singleton_stream, &mut *#singleton_state_ident);
        };

        Ok(OperatorWriteOutput {
            write_prologue,
            write_iterator,
            ..Default::default()
        })
    },
};
