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
