use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures::stream::Stream;
use pin_project_lite::pin_project;

pin_project! {
    /// Same as [`Iterator::flatten`] but as a [`Stream`].
    ///
    /// Flattens a stream of iterables into a stream of their items.
    #[must_use = "streams do nothing unless polled"]
    pub struct Flatten<St, IntoIter> {
        #[pin]
        stream: St,
        // Current iterator being consumed
        current_iter: Option<IntoIter>,
    }
}

impl<St, IntoIter> Flatten<St, IntoIter::IntoIter>
where
    St: Stream<Item = IntoIter>,
    IntoIter: IntoIterator,
{
    /// Create with source `stream`.
    pub fn new(stream: St) -> Self {
        Self {
            stream,
            current_iter: None,
        }
    }
}

impl<St, IntoIter> Stream for Flatten<St, IntoIter::IntoIter>
where
    St: Stream<Item = IntoIter>,
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
                    let new_iter = stream_item.into_iter();
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
    async fn test_flatten_basic() {
        let stream = stream::iter(vec![vec![1, 2], vec![3, 4, 5], vec![]]);
        let flattened = Flatten::new(stream);
        let result: Vec<i32> = flattened.collect().await;
        assert_eq!(result, vec![1, 2, 3, 4, 5]);
    }

    #[tokio::test]
    async fn test_flatten_empty() {
        let stream = stream::iter(Vec::<Vec<i32>>::new());
        let flattened = Flatten::new(stream);
        let result: Vec<i32> = flattened.collect().await;
        assert_eq!(result, Vec::<i32>::new());
    }

    #[tokio::test]
    async fn test_flatten_strings() {
        let stream = stream::iter(vec![
            "hello".chars().collect::<Vec<_>>(),
            "world".chars().collect::<Vec<_>>(),
        ]);
        let flattened = Flatten::new(stream);
        let result: Vec<char> = flattened.collect().await;
        assert_eq!(
            result,
            vec!['h', 'e', 'l', 'l', 'o', 'w', 'o', 'r', 'l', 'd']
        );
    }

    #[tokio::test]
    async fn test_flatten_options() {
        let stream = stream::iter(vec![Some(1), None, Some(2), Some(3)]);
        let flattened = Flatten::new(stream);
        let result: Vec<i32> = flattened.collect().await;
        assert_eq!(result, vec![1, 2, 3]);
    }
}
