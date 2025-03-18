use quote::quote_spanned;

use super::{
    OpInstGenerics, OperatorCategory, OperatorConstraints, OperatorInstance, OperatorWriteOutput,
    Persistence, RANGE_0, RANGE_1, WriteContextArgs,
};
use crate::diagnostic::{Diagnostic, Level};

/// Takes one stream as input and filters out any duplicate occurrences. The output
/// contains all unique values from the input.
///
/// ```dfir
/// source_iter(vec![1, 1, 2, 3, 2, 1, 3])
///     -> unique()
///     -> assert_eq([1, 2, 3]);
/// ```
///
/// `unique` can also be provided with one generic lifetime persistence argument, either
/// `'tick` or `'static`, to specify how data persists. The default is `'tick`.
/// With `'tick`, uniqueness is only considered within the current tick, so across multiple ticks
/// duplicate values may be emitted.
/// With `'static`, values will be remembered across ticks and no duplicates will ever be emitted.
///
/// ```rustbook
/// let (input_send, input_recv) = dfir_rs::util::unbounded_channel::<usize>();
/// let mut flow = dfir_rs::dfir_syntax! {
///     source_stream(input_recv)
///         -> unique::<'tick>()
///         -> for_each(|n| println!("{}", n));
/// };
///
/// input_send.send(3).unwrap();
/// input_send.send(3).unwrap();
/// input_send.send(4).unwrap();
/// input_send.send(3).unwrap();
/// flow.run_available();
/// // 3, 4
///
/// input_send.send(3).unwrap();
/// input_send.send(5).unwrap();
/// flow.run_available();
/// // 3, 5
/// // Note: 3 is emitted again.
/// ```
pub const UNIQUE: OperatorConstraints = OperatorConstraints {
    name: "unique",
    categories: &[OperatorCategory::Persistence],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 0,
    persistence_args: &(0..=1),
    type_args: RANGE_0,
    is_external_input: false,
    has_singleton_output: false,
    flo_type: None,
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: |wc @ &WriteContextArgs {
                   root,
                   op_span,
                   context,
                   df_ident,
                   loop_id,
                   ident,
                   inputs,
                   outputs,
                   is_pull,
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
        let persistence = persistence_args.first().copied().unwrap_or_else(|| {
            if loop_id.is_some() {
                Persistence::None
            } else {
                Persistence::Tick
            }
        });

        let input = &inputs[0];
        let output = &outputs[0];

        let uniquedata_ident = wc.make_ident("uniquedata");

        let (write_prologue, get_set) = match persistence {
            Persistence::None => (
                Default::default(),
                quote_spanned! {op_span=>
                    let mut set = #root::rustc_hash::FxHashSet::default();
                },
            ),
            Persistence::Tick => {
                let write_prologue = quote_spanned! {op_span=>
                    let #uniquedata_ident = #df_ident.add_state(::std::cell::RefCell::new(
                        #root::util::monotonic_map::MonotonicMap::<_, #root::rustc_hash::FxHashSet<_>>::default(),
                    ));
                };
                let get_set = quote_spanned! {op_span=>
                    let mut borrow = unsafe {
                        // SAFETY: handle from `#df_ident.add_state(..)`.
                        #context.state_ref_unchecked(#uniquedata_ident)
                    }.borrow_mut();
                    let set = borrow.get_mut_clear((#context.current_tick(), #context.current_stratum()));
                };
                (write_prologue, get_set)
            }
            Persistence::Static => {
                let write_prologue = quote_spanned! {op_span=>
                    let #uniquedata_ident = #df_ident.add_state(::std::cell::RefCell::new(#root::rustc_hash::FxHashSet::default()));
                };
                let get_set = quote_spanned! {op_span=>
                    let mut set = unsafe {
                        // SAFETY: handle from `#df_ident.add_state(..)`.
                        #context.state_ref_unchecked(#uniquedata_ident)
                    }.borrow_mut();
                };
                (write_prologue, get_set)
            }
            Persistence::Mutable => {
                diagnostics.push(Diagnostic::spanned(
                    op_span,
                    Level::Error,
                    "An implementation of 'mutable does not exist",
                ));
                return Err(());
            }
        };

        let filter_fn = quote_spanned! {op_span=>
            |item| {
                #get_set
                if !set.contains(item) {
                    set.insert(::std::clone::Clone::clone(item));
                    true
                } else {
                    false
                }
            }
        };
        let write_iterator = if is_pull {
            quote_spanned! {op_span=>
                let #ident = #input.filter(#filter_fn);
            }
        } else {
            quote_spanned! {op_span=>
                let #ident = #root::pusherator::filter::Filter::new(#filter_fn, #output);
            }
        };

        Ok(OperatorWriteOutput {
            write_prologue,
            write_iterator,
            ..Default::default()
        })
    },
};
