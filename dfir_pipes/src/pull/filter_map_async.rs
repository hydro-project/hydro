use core::future::Future;
use core::pin::Pin;
use core::task::Context;

use pin_project_lite::pin_project;

use crate::Yes;
use crate::pull::{FusedPull, Pull, PullStep};

pin_project! {
    /// Pull combinator that both filters and maps items using an async closure.
    ///
    /// Similar to [`super::FilterMap`], but the closure returns a `Future<Output = Option<Item>>`
    /// instead of `Option<Item>`. When the future yields `Poll::Pending`, this operator
    /// yields `Pending` as well.
    #[must_use = "`Pull`s do nothing unless polled"]
    pub struct FilterMapAsync<Prev, Func, Fut, Meta>
    where
        Fut: Future,
    {
        #[pin]
        prev: Prev,
        func: Func,
        #[pin]
        current: Option<FilterMapAsyncCurrent<Fut, Meta>>,
    }
}

pin_project! {
    struct FilterMapAsyncCurrent<Fut, Meta>
    where
        Fut: Future,
    {
        #[pin]
        future: Fut,
        meta: Meta,
    }
}

impl<Prev, Func, Fut, Meta> FilterMapAsync<Prev, Func, Fut, Meta>
where
    Self: Pull,
    Fut: Future,
{
    pub(crate) const fn new(prev: Prev, func: Func) -> Self {
        Self {
            prev,
            func,
            current: None,
        }
    }
}

impl<Prev, Func, Fut, Item> Pull for FilterMapAsync<Prev, Func, Fut, Prev::Meta>
where
    Prev: Pull,
    Func: FnMut(Prev::Item) -> Fut,
    Fut: Future<Output = Option<Item>>,
{
    type Ctx<'ctx> = Context<'ctx>;

    type Item = Item;
    type Meta = Prev::Meta;
    type CanPend = Yes;
    type CanEnd = Prev::CanEnd;

    fn pull(
        self: Pin<&mut Self>,
        ctx: &mut Self::Ctx<'_>,
    ) -> PullStep<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let mut this = self.project();
        loop {
            if let Some(cur) = this.current.as_mut().as_pin_mut().map(|c| c.project()) {
                match Future::poll(cur.future, ctx) {
                    core::task::Poll::Ready(Some(item)) => {
                        let meta = *cur.meta;
                        this.current.as_mut().set(None);
                        return PullStep::Ready(item, meta);
                    }
                    core::task::Poll::Ready(None) => {
                        this.current.as_mut().set(None);
                        continue;
                    }
                    core::task::Poll::Pending => {
                        return PullStep::Pending(Yes);
                    }
                }
            }
            debug_assert!(this.current.is_none());

            match this.prev.as_mut().pull(crate::Context::from_task(ctx)) {
                PullStep::Ready(item, meta) => {
                    let future = (this.func)(item);
                    this.current
                        .as_mut()
                        .set(Some(FilterMapAsyncCurrent { future, meta }));
                }
                PullStep::Pending(_) => {
                    return PullStep::Pending(Yes);
                }
                PullStep::Ended(can_end) => {
                    return PullStep::Ended(can_end);
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, upper) = self.prev.size_hint();
        (0, upper)
    }
}

impl<Prev, Func, Fut, Item> FusedPull for FilterMapAsync<Prev, Func, Fut, Prev::Meta>
where
    Prev: FusedPull,
    Func: FnMut(Prev::Item) -> Fut,
    Fut: Future<Output = Option<Item>>,
{
}

#[cfg(test)]
mod tests {
    use core::pin::pin;
    use core::task::{Context, Waker};

    extern crate alloc;
    use alloc::vec;

    use crate::Yes;
    use crate::pull::{Pull, PullStep};

    #[test]
    fn filter_map_async_basic() {
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);

        let mut p = pin!(
            crate::pull::iter(vec![1, 2, 3, 4])
                .filter_map_async(|x| async move { if x % 2 == 0 { Some(x * 10) } else { None } })
        );

        assert_eq!(PullStep::Ready(20, ()), p.as_mut().pull(&mut cx));
        assert_eq!(PullStep::Ready(40, ()), p.as_mut().pull(&mut cx));

        let step: PullStep<i32, (), Yes, Yes> = p.as_mut().pull(&mut cx);
        assert!(step.is_ended());
    }

    #[test]
    fn filter_map_async_all_some() {
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);

        let mut p =
            pin!(crate::pull::iter(vec![1, 2, 3]).filter_map_async(|x| async move { Some(x * 2) }));

        assert_eq!(PullStep::Ready(2, ()), p.as_mut().pull(&mut cx));
        assert_eq!(PullStep::Ready(4, ()), p.as_mut().pull(&mut cx));
        assert_eq!(PullStep::Ready(6, ()), p.as_mut().pull(&mut cx));

        let step: PullStep<i32, (), Yes, Yes> = p.as_mut().pull(&mut cx);
        assert!(step.is_ended());
    }

    #[test]
    fn filter_map_async_all_none() {
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);

        let mut p = pin!(
            crate::pull::iter(vec![1, 2, 3]).filter_map_async(|_x| async move { None::<i32> })
        );

        let step: PullStep<i32, (), Yes, Yes> = p.as_mut().pull(&mut cx);
        assert!(step.is_ended());
    }

    #[test]
    fn filter_map_async_pending_propagates() {
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);

        let mut p = pin!(
            crate::pull::iter(vec![1]).filter_map_async(|_| core::future::pending::<Option<i32>>())
        );

        for _ in 0..10 {
            let step: PullStep<i32, (), Yes, Yes> = p.as_mut().pull(&mut cx);
            assert!(step.is_pending());
        }
    }
}
