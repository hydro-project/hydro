use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures::stream::Stream;
use pin_project_lite::pin_project;

pin_project! {
    /// Stream combinator that crosses each item from `item_stream` with a singleton value from `singleton_stream`.
    pub struct CrossSingleton<'a, ItemSt, SingletonSt>
    where
        ItemSt: Stream,
        SingletonSt: Stream,
    SingletonSt::Item: Clone,
    {
        #[pin]
        item_stream: ItemSt,
        #[pin]
        singleton_stream: SingletonSt,

        singleton_state: &'a mut Option<SingletonSt::Item>,
    }
}

impl<'a, ItemSt, SingletonSt> CrossSingleton<'a, ItemSt, SingletonSt>
where
    ItemSt: Stream,
    SingletonSt: Stream,
    SingletonSt::Item: Clone,
{
    /// Creates a new `CrossSingleton` stream combinator.
    pub fn new(
        item_stream: ItemSt,
        singleton_stream: SingletonSt,
        singleton_state: &'a mut Option<SingletonSt::Item>,
    ) -> Self {
        Self {
            item_stream,
            singleton_stream,
            singleton_state,
        }
    }
}

impl<'a, ItemSt, SingletonSt> Stream for CrossSingleton<'a, ItemSt, SingletonSt>
where
    ItemSt: Stream,
    SingletonSt: Stream,
    SingletonSt::Item: Clone,
{
    type Item = (ItemSt::Item, SingletonSt::Item);

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        // Set the singleton state only if it is not already set.
        let singleton = match this.singleton_state {
            Some(singleton) => singleton,
            None => {
                let Some(singleton) = ready!(this.singleton_stream.poll_next(cx)) else {
                    // If `singleton_stream` returns EOS (`None`), we return EOS, no fused needed.
                    // This short-circuits the `ItemSt` side, dropping them.
                    return Poll::Ready(None);
                };
                this.singleton_state.insert(singleton)
            }
        };

        // Stream any items.
        let item = ready!(this.item_stream.poll_next(cx));
        // If `item_stream` returns EOS (`None`), we return EOS, no fused needed.
        let pair = item.map(|item| (item, singleton.clone()));
        Poll::Ready(pair)
    }
}
