use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures::stream::Stream;
use pin_project_lite::pin_project;

pin_project! {
    /// Same as [`Iterator::flat_map`] but as a [`Stream`].
    ///
    /// Takes a non-async closure `F(Item) -> Iterable` and flattens the results.
    #[must_use = "streams do nothing unless polled"]
    pub struct FlatMap<St, Func, IntoIter> {
        #[pin]
        stream: St,
        func: Func,
        // Current iterator being consumed
        current_iter: Option<IntoIter>,
    }
}

impl<St, Func, IntoIter> FlatMap<St, Func, IntoIter::IntoIter>
where
    St: Stream,
    Func: FnMut(St::Item) -> IntoIter,
    IntoIter: IntoIterator,
{
    /// Create with flat-mapping function `func` and source `stream`.
    pub fn new(stream: St, func: Func) -> Self {
        Self {
            stream,
            func,
            current_iter: None,
        }
    }
}

impl<St, Func, IntoIter> Stream for FlatMap<St, Func, IntoIter::IntoIter>
where
    St: Stream,
    Func: FnMut(St::Item) -> IntoIter,
    IntoIter: IntoIterator,
{
    type Item = IntoIter::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        loop {
            // First, try to get the next item from the current iterator
            if let Some(iter) = this.current_iter.as_mut() {
                if let Some(item) = iter.next() {
                    return Poll::Ready(Some(item));
                }
                // Current iterator is exhausted, clear it
                *this.current_iter = None;
            }

            // Get the next item from the stream and create a new iterator
            match ready!(this.stream.as_mut().poll_next(cx)) {
                Some(stream_item) => {
                    let new_iter = (this.func)(stream_item).into_iter();
                    *this.current_iter = Some(new_iter);
                    // Loop back to try getting an item from the new iterator
                }
                None => {
                    // Stream is exhausted
                    return Poll::Ready(None);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use futures::stream::{self, StreamExt};

    use super::*;

    #[tokio::test]
    async fn test_flat_map_basic() {
        let stream = stream::iter(vec!["hello".to_string(), "world".to_string()]);
        let flat_mapped = FlatMap::new(stream, |s: String| s.chars().collect::<Vec<_>>());
        let result: Vec<char> = flat_mapped.collect().await;
        assert_eq!(
            result,
            vec!['h', 'e', 'l', 'l', 'o', 'w', 'o', 'r', 'l', 'd']
        );
    }

    #[tokio::test]
    async fn test_flat_map_empty() {
        let stream = stream::iter(Vec::<Vec<i32>>::new());
        let flat_mapped = FlatMap::new(stream, |v| v);
        let result: Vec<i32> = flat_mapped.collect().await;
        assert_eq!(result, Vec::<i32>::new());
    }

    #[tokio::test]
    async fn test_flat_map_nested() {
        let stream = stream::iter(vec![vec![1, 2], vec![3, 4, 5], vec![]]);
        let flat_mapped = FlatMap::new(stream, |v| v);
        let result: Vec<i32> = flat_mapped.collect().await;
        assert_eq!(result, vec![1, 2, 3, 4, 5]);
    }
}
