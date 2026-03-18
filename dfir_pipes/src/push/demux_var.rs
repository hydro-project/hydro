//! [`DemuxVar`] push combinator for variadic demultiplexing.
//!
//! Receives `(usize, Item)` pairs and dispatches each item to one of several
//! downstream [`Push`]es based on the index. The downstream pushes are stored
//! as a variadic tuple (from the [`variadics`] crate) and accessed via the
//! [`PushVariadic`] trait.
//!
//! # Context handling
//!
//! Each downstream push may have a different [`Push::Ctx`] type (especially
//! when type-erased behind `impl Push`). The [`PushVariadic`] trait uses
//! recursive [`Context::Merged`] to combine all downstream contexts into a
//! single merged context type, then [`Context::unmerge_self`] /
//! [`Context::unmerge_other`] to extract each push's individual context.

use core::pin::Pin;

use pin_project_lite::pin_project;
use sealed::sealed;

use crate::push::{Push, PushStep, ready_both};
use crate::{Context, No, Toggle};

/// A variadic of [`Push`]es for use with [`DemuxVar`].
///
/// This sealed trait is implemented recursively for variadic tuples `(P, Rest)`
/// where `P: Push` and `Rest: PushVariadic`, with the base case `()`.
///
/// The [`Ctx`](PushVariadic::Ctx) GAT is built by recursively merging each
/// push's context type: for `(P0, (P1, (P2, ())))`, the merged context is
/// `<P0::Ctx as Context>::Merged<<P1::Ctx as Context>::Merged<P2::Ctx>>`.
///
/// The [`CanPend`](PushVariadic::CanPend) type is built by recursively OR-ing
/// each push's `CanPend` via [`Toggle::Or`].
#[sealed]
pub trait PushVariadic<Item, Meta>: variadics::Variadic
where
    Meta: Copy,
{
    /// The merged context type for all pushes in this variadic.
    ///
    /// Built recursively via [`Context::Merged`] so that each downstream push's
    /// context can be extracted with [`Context::unmerge_self`] and
    /// [`Context::unmerge_other`].
    type Ctx<'ctx>: Context<'ctx>;

    /// Whether any downstream push can return [`PushStep::Pending`].
    ///
    /// Built recursively via [`Toggle::Or`]: if any downstream push has
    /// `CanPend = Yes`, then the variadic also has `CanPend = Yes`.
    type CanPend: Toggle;

    /// Poll readiness of all downstream pushes.
    ///
    /// Calls [`Push::poll_ready`] on each push, passing the appropriate
    /// unmerged context slice. All pushes are polled even if one returns
    /// pending, so that all wakers are registered.
    fn poll_ready(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend>;

    /// Send an item to the push at index `idx`.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is out of bounds (greater than or equal to the number of pushes).
    fn start_send(self: Pin<&mut Self>, idx: usize, item: Item, meta: Meta);

    /// Flush all downstream pushes.
    ///
    /// Calls [`Push::poll_flush`] on each push, passing the appropriate
    /// unmerged context slice. All pushes are flushed even if one returns
    /// pending, so that all wakers are registered.
    fn poll_flush(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend>;

    /// Inform all downstream pushes that approximately `hint` items are about to be sent.
    fn size_hint(self: Pin<&mut Self>, hint: (usize, Option<usize>));
}

/// Recursive case: a push `P` followed by the rest of the variadic `Rest`.
#[sealed]
impl<P, Item, Meta: Copy, Rest> PushVariadic<Item, Meta> for (P, Rest)
where
    P: Push<Item, Meta>,
    Rest: PushVariadic<Item, Meta>,
{
    type Ctx<'ctx> = <P::Ctx<'ctx> as Context<'ctx>>::Merged<Rest::Ctx<'ctx>>;
    type CanPend = <P::CanPend as Toggle>::Or<Rest::CanPend>;

    fn poll_ready(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        let (push, rest) = pin_project_pair(self);
        ready_both!(
            push.poll_ready(<P::Ctx<'_> as Context<'_>>::unmerge_self(ctx)),
            rest.poll_ready(<P::Ctx<'_> as Context<'_>>::unmerge_other(ctx)),
        );
        PushStep::Done
    }

    fn start_send(self: Pin<&mut Self>, idx: usize, item: Item, meta: Meta) {
        let (push, rest) = pin_project_pair(self);
        if idx == 0 {
            push.start_send(item, meta);
        } else {
            rest.start_send(idx - 1, item, meta);
        }
    }

    fn poll_flush(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        let (push, rest) = pin_project_pair(self);
        ready_both!(
            push.poll_flush(<P::Ctx<'_> as Context<'_>>::unmerge_self(ctx)),
            rest.poll_flush(<P::Ctx<'_> as Context<'_>>::unmerge_other(ctx)),
        );
        PushStep::Done
    }

    fn size_hint(self: Pin<&mut Self>, hint: (usize, Option<usize>)) {
        let (push, rest) = pin_project_pair(self);
        push.size_hint(hint);
        rest.size_hint(hint);
    }
}

/// Base case: the empty variadic. Always ready, panics on send.
#[sealed]
impl<Item, Meta> PushVariadic<Item, Meta> for ()
where
    Meta: Copy,
{
    type Ctx<'ctx> = ();
    type CanPend = No;

    fn poll_ready(self: Pin<&mut Self>, _ctx: &mut Self::Ctx<'_>) -> PushStep<No> {
        PushStep::Done
    }

    fn start_send(self: Pin<&mut Self>, idx: usize, _item: Item, _meta: Meta) {
        panic!("PushVariadic index out of bounds (len + {idx})");
    }

    fn poll_flush(self: Pin<&mut Self>, _ctx: &mut Self::Ctx<'_>) -> PushStep<No> {
        PushStep::Done
    }

    fn size_hint(self: Pin<&mut Self>, _hint: (usize, Option<usize>)) {}
}

/// Pin-projects a pair `(A, B)` into its two pinned components.
///
/// # Safety
///
/// Both `A` and `B` are structurally pinned since the pair itself is pinned.
const fn pin_project_pair<A, B>(pair: Pin<&mut (A, B)>) -> (Pin<&mut A>, Pin<&mut B>) {
    // SAFETY: `pair` is pinned, and both fields of a tuple are structurally pinned.
    unsafe {
        let (a, b) = pair.get_unchecked_mut();
        (Pin::new_unchecked(a), Pin::new_unchecked(b))
    }
}

pin_project! {
    /// Push combinator that dispatches `(usize, Item)` pairs to one of several
    /// downstream pushes based on the index.
    ///
    /// The downstream pushes are stored as a variadic tuple implementing
    /// [`PushVariadic`]. Each item `(idx, value)` is routed to the push at
    /// position `idx` in the variadic.
    ///
    /// # Context
    ///
    /// The [`Push::Ctx`] for `DemuxVar` is the recursively merged context of all
    /// downstream pushes (via [`PushVariadic::Ctx`]), allowing each downstream
    /// push to have an independent context type.
    ///
    /// # Panics
    ///
    /// [`Push::start_send`] panics if the index is out of bounds.
    #[must_use = "`Push`es do nothing unless items are pushed into them"]
    pub struct DemuxVar<Pushes> {
        #[pin]
        pushes: Pushes,
    }
}

impl<Pushes> DemuxVar<Pushes> {
    /// Creates a new [`DemuxVar`] with the given downstream pushes.
    pub(crate) const fn new<Item, Meta>(pushes: Pushes) -> Self
    where
        Meta: Copy,
        Pushes: PushVariadic<Item, Meta>,
    {
        Self { pushes }
    }
}

impl<Pushes, Item, Meta> Push<(usize, Item), Meta> for DemuxVar<Pushes>
where
    Pushes: PushVariadic<Item, Meta>,
    Meta: Copy,
{
    type Ctx<'ctx> = Pushes::Ctx<'ctx>;
    type CanPend = Pushes::CanPend;

    fn poll_ready(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        self.project().pushes.poll_ready(ctx)
    }

    fn start_send(self: Pin<&mut Self>, (idx, item): (usize, Item), meta: Meta) {
        self.project().pushes.start_send(idx, item, meta);
    }

    fn poll_flush(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        self.project().pushes.poll_flush(ctx)
    }

    fn size_hint(self: Pin<&mut Self>, hint: (usize, Option<usize>)) {
        self.project().pushes.size_hint(hint);
    }
}

/// Creates a [`DemuxVar`] push that dispatches each `(usize, Item)` pair to
/// one of the downstream pushes in the given variadic, based on the index.
pub const fn demux_var<Pushes, Item, Meta>(pushes: Pushes) -> DemuxVar<Pushes>
where
    Pushes: PushVariadic<Item, Meta>,
    Meta: Copy,
{
    DemuxVar::new(pushes)
}

#[cfg(test)]
mod tests {
    use core::pin::Pin;

    extern crate alloc;
    use alloc::vec;

    use super::*;
    use crate::push::test_utils::{TestPush, assert_can_pend_no};

    #[test]
    fn test_demux_var_basic_dispatch() {
        let mut tp_a = TestPush::no_pend();
        let mut tp_b = TestPush::no_pend();
        let mut tp_c = TestPush::no_pend();

        {
            let pushes = variadics::var_expr!(&mut tp_a, &mut tp_b, &mut tp_c);
            let mut demux = demux_var(pushes);
            let mut demux = Pin::new(&mut demux);

            demux.as_mut().poll_ready(&mut ());
            demux.as_mut().start_send((0, 10), ());
            demux.as_mut().poll_ready(&mut ());
            demux.as_mut().start_send((1, 20), ());
            demux.as_mut().poll_ready(&mut ());
            demux.as_mut().start_send((2, 30), ());
            demux.as_mut().poll_ready(&mut ());
            demux.as_mut().start_send((0, 40), ());
            demux.as_mut().poll_ready(&mut ());
            demux.as_mut().start_send((2, 50), ());
            demux.as_mut().poll_flush(&mut ());
        }

        assert_eq!(tp_a.items(), vec![10, 40]);
        assert_eq!(tp_b.items(), vec![20]);
        assert_eq!(tp_c.items(), vec![30, 50]);
    }

    #[test]
    fn test_demux_var_canpend_is_no_for_sync_pushes() {
        let mut tp_a: TestPush<i32, _, _> = TestPush::no_pend();
        let mut tp_b: TestPush<i32, _, _> = TestPush::no_pend();
        let pushes = variadics::var_expr!(&mut tp_a, &mut tp_b);
        let demux = demux_var(pushes);
        assert_can_pend_no(&demux);
    }

    #[test]
    fn test_demux_var_readies_all_before_send() {
        let mut tp_a = TestPush::no_pend();
        let mut tp_b = TestPush::no_pend();
        let pushes = variadics::var_expr!(&mut tp_a, &mut tp_b);
        let mut demux = demux_var(pushes);
        let mut demux = Pin::new(&mut demux);
        demux.as_mut().poll_ready(&mut ());
        demux.as_mut().start_send((0, 10), ());
        demux.as_mut().poll_ready(&mut ());
        demux.as_mut().start_send((1, 20), ());
        demux.as_mut().poll_flush(&mut ());
    }
}
