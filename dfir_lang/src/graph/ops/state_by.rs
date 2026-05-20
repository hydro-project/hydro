use quote::{ToTokens, quote_spanned};
use syn::parse_quote;

use super::{
    OpInstGenerics, OperatorCategory, OperatorConstraints, OperatorInstance, OperatorWriteOutput,
    Persistence, PortListSpec, RANGE_1, WriteContextArgs,
};
use crate::diagnostic::{Diagnostic, Level};

/// List state operator, but with a closure to map the input to the state lattice and a factory
/// function to initialize the internal data structure.
///
/// Has two output ports:
/// - `[items]`: emits the input items that actually changed the lattice state (deltas).
/// - `[state]`: emits a clone of the accumulated lattice value after all items are processed.
///
/// The `[items]` output items are of the same type as the inputs to the state_by operator and are
/// not required to be a lattice type. This is useful for receiving pass-through context information
/// on the output side.
///
/// ```dfir
/// use std::collections::HashSet;
///
///
/// use lattices::set_union::{CartesianProductBimorphism, SetUnionHashSet, SetUnionSingletonSet};
///
/// my_state = source_iter(0..3)
///     -> state_by::<SetUnionHashSet<usize>>(SetUnionSingletonSet::new_from, std::default::Default::default);
/// my_state[items] -> null();
/// my_state[state] -> null();
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
/// my_state[items] -> null();
/// my_state[state] -> null();
/// ```
///
/// The `state` operator is equivalent to `state_by` used with an identity mapping operator with
/// `Default::default` providing the factory function.
pub const STATE_BY: OperatorConstraints = OperatorConstraints {
    name: "state_by",
    categories: &[OperatorCategory::Persistence],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: &(2..=2),
    soft_range_out: &(2..=2),
    num_args: 2,
    persistence_args: &(0..=1),
    type_args: &(0..=1),
    is_external_input: false,
    has_singleton_output: true,
    flo_type: None,
    ports_inn: None,
    ports_out: Some(|| PortListSpec::Fixed(parse_quote!(items, state))),
    input_delaytype_fn: |_| None,
    write_fn: |&WriteContextArgs {
                   root,
                   op_span,
                   ident,
                   inputs: _,
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

        let write_prologue = quote_spanned! {op_span=>
            let mut #state_ident: #lattice_type = {
                let data_struct = (#factory_fn)();
                ::std::debug_assert!(#root::lattices::IsBot::is_bot(&data_struct));
                data_struct
            };
        };
        let write_tick_end = match persistence {
            Persistence::Tick => quote_spanned! {op_span=>
                #state_ident = ::std::default::Default::default();
            },
            _ => Default::default(),
        };

        let by_fn = &arguments[0];

        // With 2 fixed output ports (items, state), the operator is always push-side.
        // outputs[0] = items (deltas), outputs[1] = state (accumulated lattice).
        assert!(!is_pull, "state_by with 2 outputs must be push-side");
        let items_output = &outputs[0];
        let state_output = &outputs[1];

        let write_iterator = quote_spanned! {op_span=>
            let #ident = {
                #[allow(non_camel_case_types)]
                struct StatePush<'a, Item, MappingFn, ItemsPsh, StatePsh, Lat> {
                    items_push: ItemsPsh,
                    state_push: StatePsh,
                    mapfn: MappingFn,
                    state_ref: &'a mut Lat,
                    _phantom: ::std::marker::PhantomData<fn(Item)>,
                }
                impl<'a, Item, MappingFn, MappedItem, ItemsPsh, StatePsh, Lat>
                    #root::dfir_pipes::push::Push<Item, ()>
                    for StatePush<'a, Item, MappingFn, ItemsPsh, StatePsh, Lat>
                where
                    Item: 'a + ::std::clone::Clone,
                    MappingFn: 'a + ::std::marker::Unpin + Fn(Item) -> MappedItem,
                    ItemsPsh: 'a + ::std::marker::Unpin + #root::dfir_pipes::push::Push<Item, ()>,
                    StatePsh: 'a + ::std::marker::Unpin + #root::dfir_pipes::push::Push<Lat, ()>,
                    Lat: 'a + 'static + ::std::clone::Clone + #root::lattices::Merge<MappedItem>,
                {
                    type Ctx<'ctx> = ();
                    type CanPend = #root::dfir_pipes::No;

                    fn poll_ready(self: ::core::pin::Pin<&mut Self>, _ctx: &mut ()) -> #root::dfir_pipes::push::PushStep<#root::dfir_pipes::No> {
                        #root::dfir_pipes::push::PushStep::Done
                    }

                    fn start_send(self: ::core::pin::Pin<&mut Self>, item: Item, _meta: ()) {
                        // SAFETY: StatePush is Unpin because all fields are Unpin.
                        let this = unsafe { self.get_unchecked_mut() };
                        let changed = #root::lattices::Merge::merge(this.state_ref, (this.mapfn)(::std::clone::Clone::clone(&item)));
                        if changed {
                            // SAFETY: items_push is Unpin.
                            unsafe { ::core::pin::Pin::new_unchecked(&mut this.items_push) }.start_send(item, ());
                        }
                    }

                    fn poll_finalize(self: ::core::pin::Pin<&mut Self>, _ctx: &mut ()) -> #root::dfir_pipes::push::PushStep<#root::dfir_pipes::No> {
                        // SAFETY: StatePush is Unpin because all fields are Unpin.
                        let this = unsafe { self.get_unchecked_mut() };
                        // Emit the accumulated state to the state output.
                        unsafe { ::core::pin::Pin::new_unchecked(&mut this.state_push) }.start_send(::std::clone::Clone::clone(this.state_ref), ());
                        #root::dfir_pipes::push::PushStep::Done
                    }

                    fn size_hint(self: ::core::pin::Pin<&mut Self>, _hint: (usize, Option<usize>)) {}
                }

                fn check_output<'a, Item, MappingFn, MappedItem, ItemsPsh, StatePsh, Lat>(
                    items_push: ItemsPsh,
                    state_push: StatePsh,
                    mapfn: MappingFn,
                    state_ref: &'a mut Lat,
                ) -> StatePush<'a, Item, MappingFn, ItemsPsh, StatePsh, Lat>
                where
                    Item: 'a + ::std::clone::Clone,
                    MappingFn: 'a + ::std::marker::Unpin + Fn(Item) -> MappedItem,
                    ItemsPsh: 'a + ::std::marker::Unpin + #root::dfir_pipes::push::Push<Item, ()>,
                    StatePsh: 'a + ::std::marker::Unpin + #root::dfir_pipes::push::Push<Lat, ()>,
                    Lat: 'a + 'static + ::std::clone::Clone + #root::lattices::Merge<MappedItem>,
                {
                    StatePush {
                        items_push,
                        state_push,
                        mapfn,
                        state_ref,
                        _phantom: ::std::marker::PhantomData,
                    }
                }
                check_output::<_, _, _, _, _, #lattice_type>(#items_output, #state_output, #by_fn, &mut #state_ident)
            };
        };

        Ok(OperatorWriteOutput {
            write_prologue,
            write_iterator,
            write_tick_end,
            ..Default::default()
        })
    },
};
