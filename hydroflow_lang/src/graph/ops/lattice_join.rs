use super::{
    FlowProperties, FlowPropertyVal, OperatorConstraints, OperatorWriteOutput, Persistence,
    WriteContextArgs, RANGE_1,
};

use crate::graph::{OpInstGenerics, OperatorInstance};
use quote::{quote_spanned, ToTokens};
use syn::parse_quote;

/// > 2 input streams of type <(K, V1)> and <(K, V2)>, 1 output stream of type <(K, (V1, V2))>
///
/// Performs a group-by with lattice-merge aggregate function on LHS and RHS inputs and then forms the
/// equijoin of the tuples in the input streams by their first (key) attribute. Note that the result nests the 2nd input field (values) into a tuple in the 2nd output field.
///
/// You must specify the LHS and RHS lattice types, they cannot be inferred.
///
/// ```hydroflow
/// // should print `(key, (2, 1))`
/// my_join = lattice_join::<hydroflow::lattices::Max<usize>, hydroflow::lattices::Max<usize>>();
/// source_iter(vec![("key", hydroflow::lattices::Max::new(0)), ("key", hydroflow::lattices::Max::new(2))]) -> [0]my_join;
/// source_iter(vec![("key", hydroflow::lattices::Max::new(1))]) -> [1]my_join;
/// my_join -> for_each(|(k, (v1, v2))| println!("({}, ({:?}, {:?}))", k, v1, v2));
/// ```
///
/// `lattice_join` can also be provided with one or two generic lifetime persistence arguments, either
/// `'tick` or `'static`, to specify how join data persists. With `'tick`, pairs will only be
/// joined with corresponding pairs within the same tick. With `'static`, pairs will be remembered
/// across ticks and will be joined with pairs arriving in later ticks. When not explicitly
/// specified persistence defaults to `static.
///
/// When two persistence arguments are supplied the first maps to port `0` and the second maps to
/// port `1`.
/// When a single persistence argument is supplied, it is applied to both input ports.
/// When no persistence arguments are applied it defaults to `'static` for both.
///
/// The syntax is as follows:
/// ```hydroflow,ignore
/// lattice_join::<MaxRepr<usize>, MaxRepr<usize>>(); // Or
/// lattice_join::<'static, MaxRepr<usize>, MaxRepr<usize>>();
///
/// lattice_join::<'tick, MaxRepr<usize>, MaxRepr<usize>>();
///
/// lattice_join::<'static, 'tick, MaxRepr<usize>, MaxRepr<usize>>();
///
/// lattice_join::<'tick, 'static, MaxRepr<usize>, MaxRepr<usize>>();
/// // etc.
/// ```
///
/// ### Examples
///
/// ```rustbook
/// use hydroflow::lattices::Max;
///
/// let (input_send, input_recv) = hydroflow::util::unbounded_channel::<(usize, Max<usize>)>();
/// let (out_tx, mut out_rx) = hydroflow::util::unbounded_channel::<(usize, (Max<usize>, Max<usize>))>();
///
/// let mut df = hydroflow::hydroflow_syntax! {
///     my_join = lattice_join::<'tick, Max<usize>, Max<usize>>();
///     source_iter([(7, Max::new(2)), (7, Max::new(1))]) -> [0]my_join;
///     source_stream(input_recv) -> [1]my_join;
///     my_join -> for_each(|v| out_tx.send(v).unwrap());
/// };
/// input_send.send((7, Max::new(5))).unwrap();
/// df.run_tick();
/// let out: Vec<_> = hydroflow::util::collect_ready(&mut out_rx);
/// assert_eq!(out, vec![(7, (Max::new(2), Max::new(5)))]);
/// ```
#[hydroflow_internalmacro::operator_docgen]
pub const LATTICE_JOIN: OperatorConstraints = OperatorConstraints {
    name: "lattice_join",
    hard_range_inn: &(2..=2),
    soft_range_inn: &(2..=2),
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 0,
    persistence_args: &(0..=2),
    type_args: &(2..=2),
    is_external_input: false,
    ports_inn: Some(|| super::PortListSpec::Fixed(parse_quote! { 0, 1 })),
    ports_out: None,
    properties: FlowProperties {
        deterministic: FlowPropertyVal::Preserve,
        monotonic: FlowPropertyVal::Preserve,
        inconsistency_tainted: false,
    },
    input_delaytype_fn: |_| None,
    write_fn: |wc @ &WriteContextArgs {
                   root,
                   context,
                   op_span,
                   ident,
                   inputs,
                   op_inst:
                       OperatorInstance {
                           generics:
                               OpInstGenerics {
                                   persistence_args,
                                   type_args,
                                   ..
                               },
                           ..
                       },
                   ..
               },
               _| {
        let persistences = match persistence_args[..] {
            [] => [Persistence::Static, Persistence::Static],
            [a] => [a, a],
            [a, b] => [a, b],
            _ => unreachable!(),
        };

        let lhs_type = type_args
            .get(0)
            .map(ToTokens::to_token_stream)
            .unwrap_or(quote_spanned!(op_span=> _));

        let rhs_type = type_args
            .get(1)
            .map(ToTokens::to_token_stream)
            .unwrap_or(quote_spanned!(op_span=> _));

        // TODO(mingwei): This is messy
        let items = persistences.zip(["lhs", "rhs"]).map(|(persistence, side)| {
            let joindata_ident = wc.make_ident(format!("joindata_{}", side));
            let borrow_ident = wc.make_ident(format!("joindata_{}_borrow", side));
            let (init, borrow) = match persistence {
                Persistence::Tick => (
                    quote_spanned! {op_span=>
                        #root::lang::monotonic_map::MonotonicMap::new_init(
                            #root::compiled::pull::HalfJoinStateLattice::default()
                        )
                    },
                    quote_spanned! {op_span=>
                        &mut *#borrow_ident.get_mut_clear(#context.current_tick())
                    },
                ),
                Persistence::Static => (
                    quote_spanned! {op_span=>
                        #root::compiled::pull::HalfJoinStateLattice::default()
                    },
                    quote_spanned! {op_span=>
                        &mut #borrow_ident
                    },
                ),
            };
            (joindata_ident, borrow_ident, init, borrow)
        });
        let [(lhs_joindata_ident, lhs_borrow_ident, lhs_init, lhs_borrow), (rhs_joindata_ident, rhs_borrow_ident, rhs_init, rhs_borrow)] =
            items;

        let join_keys_ident = wc.make_ident("joinkeys");
        let join_keys_borrow_ident = wc.make_ident("joinkeys_borrow");

        let write_prologue = quote_spanned! {op_span=>
            let #lhs_joindata_ident = df.add_state(::std::cell::RefCell::new(
                #lhs_init
            ));
            let #rhs_joindata_ident = df.add_state(::std::cell::RefCell::new(
                #rhs_init
            ));
            let #join_keys_ident = df.add_state(::std::cell::RefCell::new(
                #root::rustc_hash::FxHashSet::default()
            ));
        };

        let lhs = &inputs[0];
        let rhs = &inputs[1];
        let write_iterator = quote_spanned! {op_span=>
            let mut #lhs_borrow_ident = #context.state_ref(#lhs_joindata_ident).borrow_mut();
            let mut #rhs_borrow_ident = #context.state_ref(#rhs_joindata_ident).borrow_mut();
            let mut #join_keys_borrow_ident = #context.state_ref(#join_keys_ident).borrow_mut();

            let #ident = {
                /// Limit error propagation by bounding locally, erasing output iterator type.
                #[inline(always)]
                fn check_inputs<'a, Key, I1, Lhs, LhsDelta, I2, Rhs, RhsDelta>(
                    lhs: I1,
                    rhs: I2,
                    updated_keys: &'a mut #root::rustc_hash::FxHashSet<Key>,
                    lhs_state: &'a mut #root::compiled::pull::HalfJoinStateLattice<Key, Lhs>,
                    rhs_state: &'a mut #root::compiled::pull::HalfJoinStateLattice<Key, Rhs>,
                ) -> impl 'a + Iterator<Item = (Key, (Lhs, Rhs))>
                where
                    Key: Eq + std::hash::Hash + Clone,
                    Lhs: #root::lattices::Merge<LhsDelta> + Clone + #root::lattices::ConvertFrom<LhsDelta>,
                    Rhs: #root::lattices::Merge<RhsDelta> + Clone + #root::lattices::ConvertFrom<RhsDelta>,
                    I1: Iterator<Item = (Key, LhsDelta)>,
                    I2: Iterator<Item = (Key, RhsDelta)>,
                {
                    #root::compiled::pull::SymmetricHashJoinLattice::new_from_mut(lhs, rhs, updated_keys, lhs_state, rhs_state)
                }
                check_inputs::<_, _, #lhs_type, _, _, #rhs_type, _>(#lhs, #rhs, &mut *#join_keys_borrow_ident, #lhs_borrow, #rhs_borrow)
            };
        };

        Ok(OperatorWriteOutput {
            write_prologue,
            write_iterator,
            ..Default::default()
        })
    },
};
