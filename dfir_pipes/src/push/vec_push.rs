//! [`VecPush`] terminal push operator that collects items into a `Vec`.
use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::No;
use crate::push::{Push, PushStep};

/// Trait for Vec-like buffers that support `push` and `reserve`.
pub trait PushBuf<Item> {
    /// Push an item into the buffer.
    fn push(&mut self, item: Item);
    /// Reserve capacity for at least `additional` more items.
    fn reserve(&mut self, additional: usize);
}

impl<Item, T: PushBuf<Item>> PushBuf<Item> for &mut T {
    fn push(&mut self, item: Item) {
        (**self).push(item);
    }
    fn reserve(&mut self, additional: usize) {
        (**self).reserve(additional);
    }
}

#[cfg(feature = "alloc")]
impl<Item> PushBuf<Item> for alloc::vec::Vec<Item> {
    fn push(&mut self, item: Item) {
        self.push(item);
    }
    fn reserve(&mut self, additional: usize) {
        self.reserve(additional);
    }
}

#[cfg(feature = "bumpalo")]
impl<'bump, Item> PushBuf<Item> for bumpalo::collections::Vec<'bump, Item> {
    fn push(&mut self, item: Item) {
        self.push(item);
    }
    fn reserve(&mut self, additional: usize) {
        self.reserve(additional);
    }
}

pin_project! {
    /// Terminal push operator that collects items into a buffer implementing [`PushBuf`].
    ///
    /// Uses [`Push::size_hint`] to pre-allocate capacity via [`PushBuf::reserve`].
    #[must_use = "`Push`es do nothing unless items are pushed into them"]
    pub struct VecPush<Buf> {
        buf: Buf,
    }
}

impl<Buf> VecPush<Buf> {
    /// Creates a new [`VecPush`] writing into the given buffer.
    pub const fn new(buf: Buf) -> Self {
        Self { buf }
    }
}

impl<Buf, Item, Meta> Push<Item, Meta> for VecPush<Buf>
where
    Buf: PushBuf<Item>,
    Meta: Copy,
{
    type Ctx<'ctx> = ();
    type CanPend = No;

    fn poll_ready(self: Pin<&mut Self>, _ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        PushStep::Done
    }

    fn start_send(self: Pin<&mut Self>, item: Item, _meta: Meta) {
        self.project().buf.push(item);
    }

    fn poll_finalize(self: Pin<&mut Self>, _ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        PushStep::Done
    }

    fn size_hint(self: Pin<&mut Self>, hint: (usize, Option<usize>)) {
        self.project().buf.reserve(hint.0);
    }
}
