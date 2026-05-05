//! [`Sort`] push combinator.
use alloc::vec::Vec;
use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::push::{Push, PushStep, ready};

pin_project! {
    /// Push combinator that collects all items, sorts them, then emits them
    /// downstream in sorted order on flush.
    #[must_use = "`Push`es do nothing unless items are pushed into them"]
    #[derive(Clone, Debug)]
    pub struct Sort<Item, Next> {
        #[pin]
        next: Next,
        buf: Vec<Item>,
        sorted: bool,
        flush_idx: usize,
    }
}

impl<Item, Next> Sort<Item, Next> {
    /// Creates a new `Sort` push combinator.
    pub const fn new(next: Next) -> Self {
        Self {
            next,
            buf: Vec::new(),
            sorted: false,
            flush_idx: 0,
        }
    }
}

// TODO(mingwei): support arbitrary metadata.
impl<Item, Next> Push<Item, ()> for Sort<Item, Next>
where
    Item: Ord + Clone,
    Next: Push<Item, ()>,
{
    type Ctx<'ctx> = Next::Ctx<'ctx>;

    type CanPend = Next::CanPend;

    fn poll_ready(self: Pin<&mut Self>, _ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        PushStep::Done
    }

    fn start_send(self: Pin<&mut Self>, item: Item, _meta: ()) {
        let this = self.project();
        this.buf.push(item);
        *this.sorted = false;
    }

    fn poll_flush(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        let mut this = self.project();
        if !*this.sorted {
            this.buf.sort_unstable();
            *this.sorted = true;
            *this.flush_idx = 0;
        }
        while *this.flush_idx < this.buf.len() {
            ready!(this.next.as_mut().poll_ready(ctx));
            let item = this.buf[*this.flush_idx].clone();
            this.next.as_mut().start_send(item, ());
            *this.flush_idx += 1;
        }
        this.buf.clear();
        *this.sorted = false;
        *this.flush_idx = 0;
        this.next.poll_flush(ctx)
    }

    fn size_hint(self: Pin<&mut Self>, hint: (usize, Option<usize>)) {
        let this = self.project();
        this.buf.reserve(hint.0);
        this.next.size_hint(hint);
    }
}
