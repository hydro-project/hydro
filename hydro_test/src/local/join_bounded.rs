use hydro_lang::live_collections::stream::{ExactlyOnce, NoOrder, TotalOrder};
use hydro_lang::location::Location;
/// Compile-time type tests for `Stream::join` with bounded right side (JoinBounded).
///
/// These functions are never called — they exist solely to verify that the
/// type-level ordering guarantees are correct at compile time.
use hydro_lang::prelude::*;

/// Joining an unbounded stream with a bounded stream preserves left's ordering.
#[expect(dead_code, reason = "compile-time type test")]
fn join_unbounded_with_bounded_preserves_order<'a>(
    left: Stream<(i32, char), Process<'a>, Unbounded, TotalOrder, ExactlyOnce>,
    right: Stream<(i32, char), Process<'a>, Bounded, TotalOrder, ExactlyOnce>,
) -> Stream<(i32, (char, char)), Process<'a>, Unbounded, TotalOrder, ExactlyOnce> {
    left.join(right)
}

/// Joining two unbounded streams produces NoOrder (backward compat).
#[expect(dead_code, reason = "compile-time type test")]
fn join_unbounded_with_unbounded_is_no_order<'a>(
    left: Stream<(i32, char), Process<'a>, Unbounded, TotalOrder, ExactlyOnce>,
    right: Stream<(i32, char), Process<'a>, Unbounded, TotalOrder, ExactlyOnce>,
) -> Stream<(i32, (char, char)), Process<'a>, Unbounded, NoOrder, ExactlyOnce> {
    left.join(right)
}

/// Joining two bounded streams preserves left's ordering.
#[expect(dead_code, reason = "compile-time type test")]
fn join_bounded_with_bounded_preserves_order<'a, L: Location<'a>>(
    left: Stream<(i32, char), L, Bounded, TotalOrder, ExactlyOnce>,
    right: Stream<(i32, char), L, Bounded, TotalOrder, ExactlyOnce>,
) -> Stream<(i32, (char, char)), L, Bounded, TotalOrder, ExactlyOnce> {
    left.join(right)
}

/// Joining an unbounded NoOrder stream with bounded preserves NoOrder.
#[expect(dead_code, reason = "compile-time type test")]
fn join_unbounded_noorder_with_bounded<'a>(
    left: Stream<(i32, char), Process<'a>, Unbounded, NoOrder, ExactlyOnce>,
    right: Stream<(i32, char), Process<'a>, Bounded, NoOrder, ExactlyOnce>,
) -> Stream<(i32, (char, char)), Process<'a>, Unbounded, NoOrder, ExactlyOnce> {
    left.join(right)
}

// === cross_product: accepts different boundedness ===

/// cross_product of unbounded × bounded produces unbounded output.
#[expect(dead_code, reason = "compile-time type test")]
fn cross_product_unbounded_with_bounded<'a>(
    left: Stream<i32, Process<'a>, Unbounded, TotalOrder, ExactlyOnce>,
    right: Stream<char, Process<'a>, Bounded, TotalOrder, ExactlyOnce>,
) -> Stream<(i32, char), Process<'a>, Unbounded, NoOrder, ExactlyOnce> {
    left.cross_product(right)
}

/// cross_product of bounded × bounded produces bounded output.
#[expect(dead_code, reason = "compile-time type test")]
fn cross_product_bounded_with_bounded<'a>(
    left: Stream<i32, Process<'a>, Bounded, TotalOrder, ExactlyOnce>,
    right: Stream<char, Process<'a>, Bounded, TotalOrder, ExactlyOnce>,
) -> Stream<(i32, char), Process<'a>, Bounded, NoOrder, ExactlyOnce> {
    left.cross_product(right)
}

/// cross_product of unbounded × unbounded produces unbounded output.
#[expect(dead_code, reason = "compile-time type test")]
fn cross_product_unbounded_with_unbounded<'a>(
    left: Stream<i32, Process<'a>, Unbounded, TotalOrder, ExactlyOnce>,
    right: Stream<char, Process<'a>, Unbounded, TotalOrder, ExactlyOnce>,
) -> Stream<(i32, char), Process<'a>, Unbounded, NoOrder, ExactlyOnce> {
    left.cross_product(right)
}

// === Runtime correctness tests ===

#[cfg(test)]
mod tests {
    use hydro_lang::prelude::*;
    use stageleft::q;

    /// cross_product of unbounded × bounded produces correct results.
    #[test]
    fn cross_product_mixed_boundedness_correctness() {
        let mut flow = FlowBuilder::new();
        let process = flow.process::<()>();
        let tick = process.tick();

        // Unbounded left
        let left = process.source_iter(q!(vec![1, 2]));
        // Bounded right (batched)
        let right = process
            .source_iter(q!(vec!['a', 'b']))
            .batch(&tick, nondet!(/** test */))
            .all_ticks();

        let out = left.cross_product(right).sim_output();

        flow.sim().exhaustive(async || {
            out.assert_yields_only_unordered(vec![
                (1, 'a'),
                (1, 'b'),
                (2, 'a'),
                (2, 'b'),
            ])
            .await;
        });
    }

    /// join with bounded right produces correct results.
    #[test]
    fn join_mixed_boundedness_correctness() {
        let mut flow = FlowBuilder::new();
        let process = flow.process::<()>();
        let tick = process.tick();

        // Unbounded left
        let left = process.source_iter(q!(vec![(1, 'a'), (2, 'b')]));
        // Bounded right
        let right = process
            .source_iter(q!(vec![(1, 'x'), (2, 'y')]))
            .batch(&tick, nondet!(/** test */))
            .all_ticks();

        let out = left.join(right).sim_output();

        flow.sim().exhaustive(async || {
            out.assert_yields_only_unordered(vec![
                (1, ('a', 'x')),
                (2, ('b', 'y')),
            ])
            .await;
        });
    }
}
