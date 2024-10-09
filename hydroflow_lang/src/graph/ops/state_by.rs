use quote::{quote_spanned, ToTokens};

use super::{
    OpInstGenerics, OperatorCategory, OperatorConstraints, OperatorInstance, OperatorWriteOutput,
    Persistence, WriteContextArgs, RANGE_1,
};
use crate::diagnostic::{Diagnostic, Level};

/// List state operator, but with a closure to map the input to the state lattice.
///
/// The emitted outputs (both the referencable singleton and the optional pass-through stream) are
/// of the same type as the inputs to the state_by operator and are not required to be a lattice
/// type. This is useful receiving pass-through context information on the output side.
///
/// ```hydroflow
/// use std::collections::HashSet;
///
/// use lattices::set_union::{CartesianProductBimorphism, SetUnionHashSet, SetUnionSingletonSet};
///
/// my_state = source_iter(0..3)
///     -> state_by::<SetUnionHashSet<usize>>(SetUnionSingletonSet::new_from);
/// ```
pub const STATE_BY: OperatorConstraints = OperatorConstraints {
    name: "state_by",
    categories: &[OperatorCategory::Persistence],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: &(0..=1),
    soft_range_out: &(0..=1),
    num_args: 1,
    persistence_args: &(0..=1),
    type_args: &(0..=1),
    is_external_input: false,
    has_singleton_output: true,
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: |&WriteContextArgs {
                   root,
                   context,
                   hydroflow,
                   op_span,
                   ident,
                   inputs,
                   outputs,
                   is_pull,
                   singleton_output_ident,
                   op_name,
                   op_inst:
                       OperatorInstance {
                           generics:
                               OpInstGenerics {
                                   type_args,
                                   persistence_args,
                                   ..
                               },
                           ..
                       },
                   arguments,
                   ..
               },
               diagnostics| {
        let lattice_type = type_args
            .first()
            .map(ToTokens::to_token_stream)
            .unwrap_or(quote_spanned!(op_span=> _));

        let persistence = match persistence_args[..] {
            [] => Persistence::Tick,
            [Persistence::Mutable] => {
                diagnostics.push(Diagnostic::spanned(
                    op_span,
                    Level::Error,
                    format!("{} does not support `'mut`.", op_name),
                ));
                Persistence::Tick
            }
            [a] => a,
            _ => unreachable!(),
        };

        let state_ident = singleton_output_ident;
        let mut write_prologue = quote_spanned! {op_span=>
            let #state_ident = #hydroflow.add_state(::std::cell::RefCell::new(
                <#lattice_type as ::std::default::Default>::default()
            ));
        };
        if Persistence::Tick == persistence {
            write_prologue.extend(quote_spanned! {op_span=>
                #hydroflow.set_state_tick_hook(#state_ident, |rcell| { rcell.take(); }); // Resets state to `Default::default()`.
            });
        }

        let func = &arguments[0];

        // TODO(mingwei): deduplicate codegen
        let write_iterator = if is_pull {
            let input = &inputs[0];
            quote_spanned! {op_span=>
                let #ident = {
                    fn check_input<'a, Item, MappingFn, MappedItem, Iter, Lat>(
                        iter: Iter,
                        mapfn: MappingFn,
                        state_handle: #root::scheduled::state::StateHandle<::std::cell::RefCell<Lat>>,
                        context: &'a #root::scheduled::context::Context,
                    ) -> impl 'a + ::std::iter::Iterator<Item = Item>
                    where
                        Item: ::std::clone::Clone,
                        MappingFn: 'a + Fn(Item) -> MappedItem,
                        Iter: 'a + ::std::iter::Iterator<Item = Item>,
                        Lat: 'static + #root::lattices::Merge<MappedItem>,
                    {
                        iter.filter(move |item| {
                                let state = context.state_ref(state_handle);
                                let mut state = state.borrow_mut();
                                #root::lattices::Merge::merge(&mut *state, (mapfn)(::std::clone::Clone::clone(item)))
                            })
                    }
                    check_input::<_, _, _, _, #lattice_type>(#input, #func, #state_ident, #context)
                };
            }
        } else if let Some(output) = outputs.first() {
            quote_spanned! {op_span=>
                let #ident = {
                    fn check_output<'a, Item, MappingFn, MappedItem, Push, Lat>(
                        push: Push,
                        mapfn: MappingFn,
                        state_handle: #root::scheduled::state::StateHandle<::std::cell::RefCell<Lat>>,
                        context: &'a #root::scheduled::context::Context,
                    ) -> impl 'a + #root::pusherator::Pusherator<Item = Item>
                    where
                        Item: 'a + ::std::clone::Clone,
                        MappingFn: 'a + Fn(Item) -> MappedItem,
                        Push: 'a + #root::pusherator::Pusherator<Item = Item>,
                        Lat: 'static + #root::lattices::Merge<MappedItem>,
                    {
                        #root::pusherator::filter::Filter::new(move |item| {
                            let state = context.state_ref(state_handle);
                            let mut state = state.borrow_mut();
                                #root::lattices::Merge::merge(&mut *state, (mapfn)(::std::clone::Clone::clone(item)))
                        }, push)
                    }
                    check_output::<_, _, _, _, #lattice_type>(#output, #func, #state_ident, #context)
                };
            }
        } else {
            quote_spanned! {op_span=>
                let #ident = {
                    fn check_output<'a, Item, MappingFn, MappedItem, Lat>(
                        state_handle: #root::scheduled::state::StateHandle<::std::cell::RefCell<Lat>>,
                        mapfn: MappingFn,
                        context: &'a #root::scheduled::context::Context,
                    ) -> impl 'a + #root::pusherator::Pusherator<Item = Item>
                    where
                        Item: 'a,
                        MappedItem: 'a,
                        MappingFn: 'a + Fn(Item) -> MappedItem,
                        Lat: 'static + #root::lattices::Merge<MappedItem>,
                    {
                        #root::pusherator::for_each::ForEach::new(move |item| {
                            let state = context.state_ref(state_handle);
                            let mut state = state.borrow_mut();
                            #root::lattices::Merge::merge(&mut *state, (mapfn)(item));
                        })
                    }
                    check_output::<_, _, _, #lattice_type>(#state_ident, #func, #context)
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
