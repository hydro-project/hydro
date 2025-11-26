//! Unsync single-producer single-consumer channel (i.e. a single-threaded queue with async hooks).

use std::cell::RefCell;
use std::collections::VecDeque;
use std::num::NonZeroUsize;
use std::pin::Pin;
use std::rc::{Rc, Weak};
use std::task::{Context, Poll, Waker};

use futures::{Sink, Stream, ready};
use smallvec::SmallVec;
#[doc(inline)]
pub use tokio::sync::mpsc::error::{SendError, TrySendError};

/// Send half of am unsync MPSC.
pub struct Sender<T> {
    weak: Weak<RefCell<Shared<T>>>,
}
impl<T> Sender<T> {
    /// Asynchronously sends value to the receiver.
    pub async fn send(&self, item: T) -> Result<(), SendError<T>> {
        let mut item = Some(item);
        std::future::poll_fn(move |ctx| {
            if let Some(strong) = Weak::upgrade(&self.weak) {
                let mut shared = strong.borrow_mut();
                if shared
                    .capacity
                    .is_some_and(|cap| cap.get() <= shared.buffer.len())
                {
                    // Full.
                    shared.send_wakers.push(ctx.waker().clone());
                    Poll::Pending
                } else {
                    shared.buffer.push_back(item.take().unwrap());
                    shared.wake_receiver();
                    Poll::Ready(Ok(()))
                }
            } else {
                // Closed.
                Poll::Ready(Err(SendError(item.take().unwrap())))
            }
        })
        .await
    }

    /// Tries to send the value to the receiver without blocking.
    ///
    /// Returns an error if the destination is closed or if the buffer is at capacity.
    ///
    /// [`TrySendError::Full`] will never be returned if this is an unbounded channel.
    pub fn try_send(&self, item: T) -> Result<(), TrySendError<T>> {
        if let Some(strong) = Weak::upgrade(&self.weak) {
            let mut shared = strong.borrow_mut();
            if shared
                .capacity
                .is_some_and(|cap| cap.get() <= shared.buffer.len())
            {
                Err(TrySendError::Full(item))
            } else {
                shared.buffer.push_back(item);
                shared.wake_receiver();
                Ok(())
            }
        } else {
            Err(TrySendError::Closed(item))
        }
    }

    /// Close this sender. No more messages can be sent from this sender.
    ///
    /// Note that this only closes the channel from the view-point of this sender. The channel
    /// remains open until all senders have gone away, or until the [`Receiver`] closes the channel.
    pub fn close_this_sender(&mut self) {
        self.weak = Weak::new();
    }

    /// If this sender or the corresponding [`Receiver`] is closed.
    pub fn is_closed(&self) -> bool {
        0 == self.weak.strong_count()
    }
}
impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Self {
            weak: self.weak.clone(),
        }
    }
}
impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        // Really we should only do this if we're the very last sender,
        // But `1 == self.weak.weak_count()` seems unreliable.
        if let Some(strong) = self.weak.upgrade() {
            strong.borrow_mut().wake_receiver();
        }
    }
}

impl<T> Sink<T> for Sender<T> {
    type Error = TrySendError<Option<T>>;

    fn poll_ready(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if let Some(strong) = Weak::upgrade(&self.weak) {
            let mut shared = strong.borrow_mut();
            if shared
                .capacity
                .is_some_and(|cap| cap.get() <= shared.buffer.len())
            {
                // Full.
                shared.send_wakers.push(ctx.waker().clone());
                Poll::Pending
            } else {
                // Has room.
                Poll::Ready(Ok(()))
            }
        } else {
            // Closed
            Poll::Ready(Err(TrySendError::Closed(None)))
        }
    }

    fn start_send(self: Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
        self.try_send(item).map_err(|e| match e {
            TrySendError::Full(item) => TrySendError::Full(Some(item)),
            TrySendError::Closed(item) => TrySendError::Closed(Some(item)),
        })
    }

    fn poll_flush(self: Pin<&mut Self>, _ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(
        mut self: Pin<&mut Self>,
        ctx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        ready!(self.as_mut().poll_flush(ctx))?;
        Pin::into_inner(self).close_this_sender();
        Poll::Ready(Ok(()))
    }
}

/// Receiving half of an unsync MPSC.
pub struct Receiver<T> {
    strong: Rc<RefCell<Shared<T>>>,
}
impl<T> Receiver<T> {
    /// Receive a value asynchronously.
    pub async fn recv(&mut self) -> Option<T> {
        std::future::poll_fn(|ctx| self.poll_recv(ctx)).await
    }

    /// Poll for a value.
    /// NOTE: takes `&mut self` to prevent multiple concurrent receives.
    pub fn poll_recv(&mut self, ctx: &Context<'_>) -> Poll<Option<T>> {
        let mut shared = self.strong.borrow_mut();
        if let Some(value) = shared.buffer.pop_front() {
            shared.wake_sender();
            Poll::Ready(Some(value))
        } else if 0 == Rc::weak_count(&self.strong) {
            Poll::Ready(None) // Empty and dropped.
        } else {
            shared.recv_waker = Some(ctx.waker().clone());
            Poll::Pending
        }
    }

    /// Closes this receiving end, not allowing more values to be sent while still allowing already-sent values to be consumed.
    pub fn close(&mut self) {
        assert_eq!(
            1,
            Rc::strong_count(&self.strong),
            "BUG: receiver has non-exclusive Rc."
        );

        let new_shared = {
            let mut shared = self.strong.borrow_mut();
            shared.wake_all_senders();

            Shared {
                buffer: std::mem::take(&mut shared.buffer),
                ..Default::default()
            }
        };
        self.strong = Rc::new(RefCell::new(new_shared));
        // Drop old `Rc`, invalidating all `Weak`s.
    }
}
impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        self.close()
    }
}
impl<T> Stream for Receiver<T> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.poll_recv(ctx)
    }
}

/// Struct shared between sender and receiver.
struct Shared<T> {
    buffer: VecDeque<T>,
    capacity: Option<NonZeroUsize>,
    send_wakers: SmallVec<[Waker; 1]>,
    recv_waker: Option<Waker>,
}
impl<T> Shared<T> {
    /// Wakes one sender (if there are any wakers), and removes the waker.
    pub fn wake_sender(&mut self) {
        if let Some(waker) = self.send_wakers.pop() {
            waker.wake();
        }
    }
    /// Wakes all senders and removes their wakers.
    pub fn wake_all_senders(&mut self) {
        self.send_wakers.drain(..).for_each(Waker::wake);
    }
    /// Wakes the receiver (if the waker is set) and removes it.
    pub fn wake_receiver(&mut self) {
        if let Some(waker) = self.recv_waker.take() {
            waker.wake();
        }
    }
}
impl<T> Default for Shared<T> {
    fn default() -> Self {
        let (buffer, capacity, send_wakers, recv_waker) = Default::default();
        Self {
            buffer,
            capacity,
            send_wakers,
            recv_waker,
        }
    }
}

/// Create an unsync MPSC channel, either bounded (if `capacity` is `Some`) or unbounded (if `capacity` is `None`).
pub fn channel<T>(capacity: Option<NonZeroUsize>) -> (Sender<T>, Receiver<T>) {
    let (buffer, send_wakers, recv_waker) = Default::default();
    let shared = Rc::new(RefCell::new(Shared {
        buffer,
        capacity,
        send_wakers,
        recv_waker,
    }));
    let sender = Sender {
        weak: Rc::downgrade(&shared),
    };
    let receiver = Receiver { strong: shared };
    (sender, receiver)
}

/// Create a bounded unsync MPSC channel. Panics if capacity is zero.
pub fn bounded<T>(capacity: usize) -> (Sender<T>, Receiver<T>) {
    let capacity = NonZeroUsize::new(capacity);
    assert!(capacity.is_some(), "Capacity cannot be zero.");
    channel(capacity)
}

/// Create an unbounded unsync MPSC channel.
pub fn unbounded<T>() -> (Sender<T>, Receiver<T>) {
    channel(None)
}

#[cfg(test)]
mod test {
    use futures::StreamExt;
    use rand::Rng;
    use tokio::task::LocalSet;
    use web_time::Duration;

    use super::*;

    async fn delay(n: u64) {
        let millis = rand::thread_rng().gen_range(0..n);
        tokio::time::sleep(Duration::from_millis(millis)).await;
    }

    #[crate::test]
    async fn test_send_multiple_outstanding() {
        let (send, recv) = bounded::<u64>(10);

        let a_fut = send.send(123);
        let b_fut = send.send(234);

        futures::future::try_join(a_fut, b_fut).await.unwrap();
        drop(send);

        let mut out: Vec<_> = recv.collect().await;
        out.sort_unstable();
        assert_eq!([123, 234], &*out);
    }

    #[crate::test]
    async fn test_spsc_random() {
        let runs = (0..1_000).map(|_| async {
            let (send, recv) = bounded::<u64>(10);

            let local = LocalSet::new();

            local.spawn_local(async move {
                for x in 0..100 {
                    send.send(x).await.unwrap();
                    delay(4).await;
                }
            });
            local.spawn_local(async move {
                delay(5).await; // Delay once first.

                let mut recv = recv;
                let mut i = 0;
                while let Some(x) = recv.recv().await {
                    assert_eq!(i, x);
                    i += 1;
                    delay(5).await;
                }
                assert_eq!(100, i);
            });
            local.await;
        });
        futures::future::join_all(runs).await;
    }

    #[crate::test]
    async fn test_mpsc_random() {
        let runs = (0..1_000).map(|_| async {
            let (send, recv) = bounded::<u64>(30);
            let send_a = send.clone();
            let send_b = send.clone();
            let send_c = send;

            let local = LocalSet::new();

            local.spawn_local(async move {
                for x in 0..100 {
                    send_a.send(x).await.unwrap();
                    delay(5).await;
                }
            });
            local.spawn_local(async move {
                for x in 100..200 {
                    send_b.send(x).await.unwrap();
                    delay(5).await;
                }
            });
            local.spawn_local(async move {
                for x in 200..300 {
                    send_c.send(x).await.unwrap();
                    delay(5).await;
                }
            });
            local.spawn_local(async move {
                delay(1).await; // Delay once first.

                let mut recv = recv;
                let mut vec = Vec::new();
                while let Some(x) = recv.next().await {
                    vec.push(x);
                    delay(1).await;
                }
                assert_eq!(300, vec.len());
                vec.sort_unstable();
                for (i, &x) in vec.iter().enumerate() {
                    assert_eq!(i as u64, x);
                }
            });
            local.await;
        });
        futures::future::join_all(runs).await;
    }

    #[crate::test]
    async fn test_stream_sink_loop() {
        use futures::{SinkExt, StreamExt};

        const N: usize = 100;

        let (mut send, mut recv) = unbounded::<usize>();
        send.send(0).await.unwrap();
        // Connect it to itself
        let mut recv_ref = recv.by_ref().map(|x| x + 1).map(Ok).take(N);
        send.send_all(&mut recv_ref).await.unwrap();
        assert_eq!(Some(N), recv.recv().await);
    }
}
