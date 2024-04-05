use quote::{quote_spanned, ToTokens};
use syn::parse_quote;

use super::{
    GraphEdgeType, OpInstGenerics, OperatorCategory, OperatorConstraints, OperatorInstance,
    OperatorWriteOutput, PortIndexValue, PortListSpec, WriteContextArgs,
    LATTICE_FOLD_REDUCE_FLOW_PROP_FN, RANGE_0, RANGE_1,
};

/// A lattice-based state operator, used for accumulating lattice state
///
/// Emits both a referenceable accumulated value `state`, and a pass-through stream `items`. In the
/// future the pass-through stream may be deduplicated.
///
/// ```hydroflow
/// use std::collections::HashSet;
///
/// use lattices::set_union::{CartesianProductBimorphism, SetUnionHashSet, SetUnionSingletonSet};
///
/// lhs = source_iter_delta(0..3)
///     -> map(SetUnionSingletonSet::new_from)
///     -> state::<SetUnionHashSet<usize>>();
/// rhs = source_iter_delta(3..5)
///     -> map(SetUnionSingletonSet::new_from)
///     -> state::<SetUnionHashSet<usize>>();
///
/// lhs[items] -> [items_0]my_join;
/// rhs[items] -> [items_1]my_join;
/// lhs[state] -> [state_0]my_join;
/// rhs[state] -> [state_1]my_join;
///
/// my_join = lattice_bimorphism(CartesianProductBimorphism::<HashSet<_>>::default())
///     -> lattice_reduce()
///     -> for_each(|x| println!("{:?}", x));
/// ```
pub const STATE: OperatorConstraints = OperatorConstraints {
    name: "state",
    categories: &[OperatorCategory::Persistence],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: &(2..=2),
    soft_range_out: &(2..=2),
    num_args: 0,
    persistence_args: RANGE_0, // TODO(mingwei)?
    type_args: &(0..=1),
    is_external_input: false,
    ports_inn: None,
    ports_out: Some(|| PortListSpec::Fixed(parse_quote! { items, state })),
    input_delaytype_fn: |_| None,
    input_edgetype_fn: |_| Some(GraphEdgeType::Value),
    output_edgetype_fn: |idx| match idx {
        PortIndexValue::Path(path) if "state" == path.to_token_stream().to_string() => {
            GraphEdgeType::Reference
        }
        _else => GraphEdgeType::Value,
    },
    flow_prop_fn: Some(LATTICE_FOLD_REDUCE_FLOW_PROP_FN),
    write_fn: |&WriteContextArgs {
                   root,
                   context,
                   hydroflow,
                   op_span,
                   ident,
                   inputs,
                   outputs,
                   is_pull,
                   op_inst:
                       OperatorInstance {
                           generics: OpInstGenerics { type_args, .. },
                           ..
                       },
                   ..
               },
               _diagnostics| {
        let lattice_type = type_args
            .first()
            .map(ToTokens::to_token_stream)
            .unwrap_or(quote_spanned!(op_span=> _));

        let state_ident = &outputs[1];

        let write_prologue = quote_spanned! {op_span=>
            let #state_ident = #hydroflow.add_state(::std::cell::RefCell::new(
                <#lattice_type as ::std::default::Default>::default()
            ));
        };

        let write_iterator = if is_pull {
            let input = &inputs[0];
            quote_spanned! {op_span=>
                let #ident = {
                    fn check_input<'a, Item, Iter, Lat>(
                        iter: Iter,
                        state_handle: #root::scheduled::state::StateHandle<::std::cell::RefCell<Lat>>,
                        context: &'a #root::scheduled::context::Context,
                    ) -> impl 'a + ::std::iter::Iterator<Item = Item>
                    where
                        Item: ::std::clone::Clone,
                        Iter: 'a + ::std::iter::Iterator<Item = Item>,
                        Lat: 'static + #root::lattices::Merge<Item>,
                    {
                        iter.inspect(move |item| {
                            let state = context.state_ref(state_handle);
                            let mut state = state.borrow_mut();
                            #root::lattices::Merge::merge(&mut *state, ::std::clone::Clone::clone(item));
                        })
                    }
                    check_input::<_, _, #lattice_type>(#input, #state_ident, #context)
                };
            }
        } else {
            let output = &outputs[0];
            quote_spanned! {op_span=>
                let #ident = {
                    fn check_output<'a, Item, Push, Lat>(
                        push: Push,
                        state_handle: #root::scheduled::state::StateHandle<::std::cell::RefCell<Lat>>,
                        context: &'a #root::scheduled::context::Context,
                    ) -> impl 'a + #root::pusherator::Pusherator<Item = Item>
                    where
                        Item: ::std::clone::Clone,
                        Push: #root::pusherator::Pusherator<Item = Item>,
                        Lat: 'static + #root::lattices::Merge<Item>,
                    {
                        #root::pusherator::inspect::Inspect::new(move |item| {
                            let state = context.state_ref(state_handle);
                            let mut state = state.borrow_mut();
                            #root::lattices::Merge::merge(&mut *state, ::std::clone::Clone::clone(item));
                        }, push)
                    }
                    check_output::<_, _, #lattice_type>(#output, #state_ident, #context)
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
