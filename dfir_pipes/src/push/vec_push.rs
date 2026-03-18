//! [`VecPush`] terminal push operator that collects items into a `Vec`.
use alloc::vec::Vec;
use core::borrow::BorrowMut;
use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::No;
use crate::push::{Push, PushStep};

pin_project! {
    /// Terminal push operator that collects items into a `Vec`.
    ///
    /// Uses [`Push::size_hint`] to pre-allocate capacity via [`Vec::reserve`].
    #[must_use = "`Push`es do nothing unless items are pushed into them"]
    pub struct VecPush<Buf> {
        buf: Buf,
    }
}

impl<Buf> VecPush<Buf> {
    /// Creates a new [`VecPush`] writing into the given buffer.
    pub(crate) const fn new<Item>(buf: Buf) -> Self
    where
        Buf: BorrowMut<Vec<Item>>,
    {
        Self { buf }
    }
}

impl<Buf, Item, Meta> Push<Item, Meta> for VecPush<Buf>
where
    Buf: BorrowMut<Vec<Item>>,
    Meta: Copy,
{
    type Ctx<'ctx> = ();
    type CanPend = No;

    fn poll_ready(self: Pin<&mut Self>, _ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        PushStep::Done
    }

    fn start_send(self: Pin<&mut Self>, item: Item, _meta: Meta) {
        self.project().buf.borrow_mut().push(item);
    }

    fn poll_flush(self: Pin<&mut Self>, _ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        PushStep::Done
    }

    fn size_hint(self: Pin<&mut Self>, hint: (usize, Option<usize>)) {
        self.project().buf.borrow_mut().reserve(hint.0);
    }
}
