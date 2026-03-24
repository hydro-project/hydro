//! [`StreamCompat`] adapter wrapping a [`Pull`] into a [`futures_core::stream::Stream`].
use core::pin::Pin;
use core::task::Poll;

use pin_project_lite::pin_project;

use crate::Context;
use crate::pull::Pull;

pin_project! {
    /// Adapter that wraps a [`Pull`] to implement the [`Stream`](futures_core::stream::Stream) trait.
    #[must_use = "`Stream`s do nothing unless polled"]
    pub struct StreamCompat<Pul> {
        #[pin]
        pull: Pul,
    }
}

impl<Pul> StreamCompat<Pul> {
    /// Creates a new [`StreamCompat`] wrapping the given [`Pull`].
    pub(crate) const fn new(pull: Pul) -> Self {
        Self { pull }
    }

    /// Returns the wrapped [`Pull`].
    pub fn into_inner(self) -> Pul {
        self.pull
    }

    /// Returns a pinned mutable reference to the wrapped [`Pull`].
    pub fn as_pin_mut(self: Pin<&mut Self>) -> Pin<&mut Pul> {
        self.project().pull
    }

    /// Returns a pinned reference to the wrapped [`Pull`].
    pub fn as_pin_ref(self: Pin<&Self>) -> Pin<&Pul> {
        self.project_ref().pull
    }
}

impl<Pul> AsMut<Pul> for StreamCompat<Pul> {
    fn as_mut(&mut self) -> &mut Pul {
        &mut self.pull
    }
}

impl<Pul> AsRef<Pul> for StreamCompat<Pul> {
    fn as_ref(&self) -> &Pul {
        &self.pull
    }
}

impl<Pul> futures_core::stream::Stream for StreamCompat<Pul>
where
    Pul: Pull,
    for<'ctx> Pul::Ctx<'ctx>: Context<'ctx>,
{
    type Item = Pul::Item;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.as_pin_mut()
            .pull(Context::from_task(cx))
            .into_poll()
            .map(|opt| opt.map(|(item, _meta)| item))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.pull.size_hint()
    }
}
