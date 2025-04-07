use quote::{ToTokens, quote_spanned};

use super::{
    OpInstGenerics, OperatorCategory, OperatorConstraints, OperatorInstance, OperatorWriteOutput,
    Persistence, RANGE_1, WriteContextArgs,
};
use crate::diagnostic::{Diagnostic, Level};

/// List state operator, but with a closure to map the input to the state lattice and a factory
/// function to initialize the internal data structure.
///
/// The emitted outputs (both the referencable singleton and the optional pass-through stream) are
/// of the same type as the inputs to the state_by operator and are not required to be a lattice
/// type. This is useful receiving pass-through context information on the output side.
///
/// ```dfir
/// use std::collections::HashSet;
///
///
/// use lattices::set_union::{CartesianProductBimorphism, SetUnionHashSet, SetUnionSingletonSet};
///
/// my_state = source_iter(0..3)
///     -> state_by::<SetUnionHashSet<usize>>(SetUnionSingletonSet::new_from, std::default::Default::default);
/// ```
/// The 2nd argument into `state_by` is a factory function that can be used to supply a custom
/// initial value for the backing state. The initial value is still expected to be bottom (and will
/// be checked). This is useful for doing things like pre-allocating buffers, etc. In the above
/// example, it is just using `Default::default()`
///
/// An example of preallocating the capacity in a hashmap:
///
/// ```dfir
/// use std::collections::HashSet;
/// use lattices::set_union::{SetUnion, CartesianProductBimorphism, SetUnionHashSet, SetUnionSingletonSet};
///
/// my_state = source_iter(0..3)
///     -> state_by::<SetUnionHashSet<usize>>(SetUnionSingletonSet::new_from, {|| SetUnion::new(HashSet::<usize>::with_capacity(1_000)) });
/// ```
///
/// The `state` operator is equivalent to `state_by` used with an identity mapping operator with
/// `Default::default` providing the factory function.
pub const STATE_BY: OperatorConstraints = OperatorConstraints {
    name: "state_by",
    categories: &[OperatorCategory::Persistence],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: &(0..=1),
    soft_range_out: &(0..=1),
    num_args: 2,
    persistence_args: &(0..=1),
    type_args: &(0..=1),
    is_external_input: false,
    has_singleton_output: true,
    flo_type: None,
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: |wc @ &WriteContextArgs {
                   root,
                   context,
                   df_ident,
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
        let factory_fn = &arguments[1];

        let write_prologue = quote_spanned! { op_span=>
                    let #state_ident = {
                        let data_struct : #lattice_type = (#factory_fn)();
                        ::std::debug_assert!(::lattices::IsBot::is_bot(&data_struct));
                        #df_ident.add_state(::std::cell::RefCell::new(data_struct))
                    };
        };
        let write_prologue_after = wc
            .persistence_as_state_lifespan(persistence)
            .map(|lifespan| quote_spanned! {op_span=>
                #df_ident.set_state_lifespan_hook(#state_ident, #lifespan, |rcell| { rcell.take(); });
            }).unwrap_or_default();

        let by_fn = &arguments[0];

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
                                let state = unsafe {
                                    // SAFETY: handle from `#df_ident.add_state(..)`.
                                    context.state_ref_unchecked(state_handle)
                                };
                                let mut state = state.borrow_mut();
                                #root::lattices::Merge::merge(&mut *state, (mapfn)(::std::clone::Clone::clone(item)))
                            })
                    }
                    check_input::<_, _, _, _, #lattice_type>(#input, #by_fn, #state_ident, #context)
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
                            let state = unsafe {
                                // SAFETY: handle from `#df_ident.add_state(..)`.
                                context.state_ref_unchecked(state_handle)
                            };
                            let mut state = state.borrow_mut();
                                #root::lattices::Merge::merge(&mut *state, (mapfn)(::std::clone::Clone::clone(item)))
                        }, push)
                    }
                    check_output::<_, _, _, _, #lattice_type>(#output, #by_fn, #state_ident, #context)
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
                            let state = unsafe {
                                // SAFETY: handle from `#df_ident.add_state(..)`.
                                context.state_ref_unchecked(state_handle)
                            };
                            let mut state = state.borrow_mut();
                            #root::lattices::Merge::merge(&mut *state, (mapfn)(item));
                        })
                    }
                    check_output::<_, _, _, #lattice_type>(#state_ident, #by_fn, #context)
                };
            }
        };
        Ok(OperatorWriteOutput {
            write_prologue,
            write_prologue_after,
            write_iterator,
            ..Default::default()
        })
    },
};
