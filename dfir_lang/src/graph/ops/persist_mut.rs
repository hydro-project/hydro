use quote::quote_spanned;

use super::{
    DelayType, OpInstGenerics, OperatorCategory, OperatorConstraints, OperatorInstance,
    OperatorWriteOutput, Persistence, RANGE_0, RANGE_1, WriteContextArgs,
};
use crate::diagnostic::{Diagnostic, Level};

/// `persist_mut()` is similar to `persist()` except that it also enables deletions.
/// `persist_mut()` expects an input of type [`Persistence<T>`](https://docs.rs/dfir_rs/latest/dfir_rs/util/enum.Persistence.html),
/// and it is this enumeration that enables the user to communicate deletion.
/// Deletions/persists happen in the order they are received in the stream.
/// For example, `[Persist(1), Delete(1), Persist(1)]` will result in a a single `1` value being stored.
///
/// ```dfir
/// use dfir_rs::util::Persistence;
///
/// source_iter([
///         Persistence::Persist(1),
///         Persistence::Persist(2),
///         Persistence::Delete(1),
///     ])
///     -> persist_mut::<'mutable>()
///     -> assert_eq([2]);
/// ```
pub const PERSIST_MUT: OperatorConstraints = OperatorConstraints {
    name: "persist_mut",
    categories: &[OperatorCategory::Persistence],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 0,
    persistence_args: RANGE_1,
    type_args: RANGE_0,
    is_external_input: false,
    // If this is set to true, the state will need to be cleared via `write_tick_end`
    // to prevent reading uncleared data if this subgraph doesn't run.
    // https://github.com/hydro-project/hydro/issues/1298
    // If `'tick` lifetimes are added.
    flo_type: None,
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: |wc @ &WriteContextArgs {
                   root,
                   op_span,
                   work_fn_async,
                   ident,
                   inputs,
                   outputs,
                   is_pull,
                   op_name,
                   op_inst:
                       OperatorInstance {
                           generics:
                               OpInstGenerics {
                                   persistence_args, ..
                               },
                           ..
                       },
                   ..
               },
               diagnostics| {
        if [Persistence::Mutable] != persistence_args[..] {
            diagnostics.push(Diagnostic::spanned(
                op_span,
                Level::Error,
                format!(
                    "{} only supports `'{}`.",
                    op_name,
                    Persistence::Mutable.to_str_lowercase()
                ),
            ));
        }

        let persistdata_ident = wc.make_ident("persistdata");
        let write_prologue = quote_spanned! {op_span=>
            let mut #persistdata_ident = #root::util::sparse_vec::SparseVec::default();
        };

        let write_iterator = if is_pull {
            let input = &inputs[0];
            quote_spanned! {op_span=>
                let #ident = {
                    #[inline(always)]
                    fn check_pull<Prev, T: ::std::hash::Hash + ::std::cmp::Eq>(prev: Prev)
                        -> impl #root::dfir_pipes::pull::Pull<Item = #root::util::Persistence::<T>, Meta = Prev::Meta, CanPend = Prev::CanPend, CanEnd = Prev::CanEnd>
                    where
                        Prev: #root::dfir_pipes::pull::Pull<Item = #root::util::Persistence::<T>>
                    {
                        prev
                    }

                    let iter = {
                        let fut = #root::dfir_pipes::pull::Pull::for_each(check_pull(#input), |item| {
                            match item {
                                #root::util::Persistence::Persist(v) => #persistdata_ident.push(v),
                                #root::util::Persistence::Delete(v) => #persistdata_ident.delete(&v),
                            }
                        });
                        let () = #work_fn_async(fut).await;

                        #persistdata_ident.iter().cloned()
                    };
                    #root::dfir_pipes::pull::iter(iter)
                };
            }
        } else {
            let output = &outputs[0];
            quote_spanned! {op_span=>
                let #ident = #root::dfir_pipes::push::Fold::new(
                    &mut #persistdata_ident,
                    |state: &mut #root::util::sparse_vec::SparseVec<_>, item| {
                        match item {
                            #root::util::Persistence::Persist(v) => state.push(v),
                            #root::util::Persistence::Delete(v) => state.delete(&v),
                        }
                    },
                    #root::dfir_pipes::push::flat_map(
                        |state: #root::util::sparse_vec::SparseVec<_>| state.iter().cloned().collect::<::std::vec::Vec<_>>(),
                        #output,
                    ),
                );
            }
        };

        Ok(OperatorWriteOutput {
            write_prologue,
            write_iterator,
            ..Default::default()
        })
    },
};
