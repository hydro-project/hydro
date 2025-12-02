//! [`LazyDemuxSink`] and related items.
use core::fmt::Debug;
use core::hash::Hash;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::collections::HashMap;

use crate::{Sink, ready_both};

/// Sink which receives keys paired with items `(Key, Item)`, and lazily creates sinks on first use.
pub struct LazyDemuxSink<Key, Si, Func> {
    sinks: HashMap<Key, Si>,
    func: Func,
}

impl<Key, Si, Func> LazyDemuxSink<Key, Si, Func> {
    /// Create with the given initialization function.
    pub fn new<Item>(func: Func) -> Self
    where
        Self: Sink<(Key, Item)>,
    {
        Self {
            sinks: HashMap::new(),
            func,
        }
    }
}

impl<Key, Si, Item, Func> Sink<(Key, Item)> for LazyDemuxSink<Key, Si, Func>
where
    Key: Eq + Hash + Debug + Unpin,
    Si: Sink<Item> + Unpin,
    Func: FnMut(&Key) -> Si + Unpin,
{
    type Error = Si::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.get_mut()
            .sinks
            .values_mut()
            .try_fold(Poll::Ready(()), |poll, sink| {
                ready_both!(poll, Pin::new(sink).poll_ready(cx)?);
                Poll::Ready(Ok(()))
            })
    }

    fn start_send(self: Pin<&mut Self>, item: (Key, Item)) -> Result<(), Self::Error> {
        let this = self.get_mut();
        let sink = this
            .sinks
            .entry(item.0)
            .or_insert_with_key(|k| (this.func)(k));
        Pin::new(sink).start_send(item.1)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.get_mut()
            .sinks
            .values_mut()
            .try_fold(Poll::Ready(()), |poll, sink| {
                ready_both!(poll, Pin::new(sink).poll_flush(cx)?);
                Poll::Ready(Ok(()))
            })
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.get_mut()
            .sinks
            .values_mut()
            .try_fold(Poll::Ready(()), |poll, sink| {
                ready_both!(poll, Pin::new(sink).poll_close(cx)?);
                Poll::Ready(Ok(()))
            })
    }
}

/// Creates a `LazyDemuxSink` that lazily creates sinks on first use for each key.
///
/// This requires sinks `Si` to be `Unpin`. If your sinks are not `Unpin`, first wrap them in `Box::pin` to make them `Unpin`.
pub fn demux_map_lazy<Key, Si, Item, Func>(func: Func) -> LazyDemuxSink<Key, Si, Func>
where
    Key: Eq + Hash + Debug + Unpin,
    Si: Sink<Item> + Unpin,
    Func: FnMut(&Key) -> Si + Unpin,
{
    LazyDemuxSink::new(func)
}

#[cfg(test)]
mod test {
    use core::cell::RefCell;
    use core::pin::pin;
    use std::collections::HashMap;
    use std::rc::Rc;

    use futures_util::SinkExt;

    use super::*;
    use crate::for_each::ForEach;

    #[tokio::test]
    async fn test_lazy_demux_sink() {
        let outputs: Rc<RefCell<HashMap<String, Vec<u8>>>> = Rc::new(RefCell::new(HashMap::new()));
        let outputs_clone = outputs.clone();

        let mut sink = demux_map_lazy(move |key: &String| {
            let key = key.clone();
            let outputs = outputs_clone.clone();
            ForEach::new(move |item: &[u8]| {
                outputs
                    .borrow_mut()
                    .entry(key.clone())
                    .or_default()
                    .extend_from_slice(item);
            })
        });

        sink.send(("a".to_owned(), b"test1".as_slice()))
            .await
            .unwrap();
        sink.send(("b".to_owned(), b"test2".as_slice()))
            .await
            .unwrap();
        sink.send(("a".to_owned(), b"test3".as_slice()))
            .await
            .unwrap();
        sink.flush().await.unwrap();
        sink.close().await.unwrap();

        let outputs = outputs.borrow();
        assert_eq!(outputs.get("a").unwrap().as_slice(), b"test1test3");
        assert_eq!(outputs.get("b").unwrap().as_slice(), b"test2");
    }

    #[test]
    fn test_lazy_demux_sink_good() {
        use core::task::Context;

        let outputs: Rc<RefCell<HashMap<String, Vec<u8>>>> = Rc::new(RefCell::new(HashMap::new()));
        let outputs_clone = outputs.clone();

        let mut sink = pin!(demux_map_lazy(move |key: &String| {
            let outputs = outputs_clone.clone();
            let key = key.clone();
            ForEach::new(move |item: &[u8]| {
                outputs
                    .borrow_mut()
                    .entry(key.clone())
                    .or_default()
                    .extend_from_slice(item);
            })
        }));

        let cx = &mut Context::from_waker(futures_task::noop_waker_ref());

        assert_eq!(Poll::Ready(Ok(())), sink.as_mut().poll_ready(cx));
        assert_eq!(
            Ok(()),
            sink.as_mut()
                .start_send(("a".to_owned(), b"test1".as_slice()))
        );
        assert_eq!(
            Ok(()),
            sink.as_mut()
                .start_send(("b".to_owned(), b"test2".as_slice()))
        );
        assert_eq!(
            Ok(()),
            sink.as_mut()
                .start_send(("a".to_owned(), b"test3".as_slice()))
        );
        assert_eq!(Poll::Ready(Ok(())), sink.as_mut().poll_flush(cx));
        assert_eq!(Poll::Ready(Ok(())), sink.as_mut().poll_close(cx));

        let outputs = outputs.borrow();
        assert_eq!(outputs.get("a").unwrap().as_slice(), b"test1test3");
        assert_eq!(outputs.get("b").unwrap().as_slice(), b"test2");
    }
}
