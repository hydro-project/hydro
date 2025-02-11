use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use futures::{Future, Stream, StreamExt};
use tokio::sync::{mpsc, oneshot};

pub async fn async_retry<T, F: Future<Output = Result<T>>>(
    mut thunk: impl FnMut() -> F,
    count: usize,
    delay: Duration,
) -> Result<T> {
    for _ in 1..count {
        let result = thunk().await;
        if result.is_ok() {
            return result;
        } else {
            tokio::time::sleep(delay).await;
        }
    }

    thunk().await
}

type PriorityBroadcacst = (
    Arc<Mutex<Option<oneshot::Sender<String>>>>,
    Arc<Mutex<Vec<mpsc::UnboundedSender<String>>>>,
);

// Divides up a single stream into two channels (one prioritized and the other not)
// The first data packet that comes in will get sent to the first channel, and all others will get sent to the second channel
pub fn prioritized_broadcast<T: Stream<Item = io::Result<String>> + Send + Unpin + 'static>(
    mut lines: T,
    default: impl Fn(String) + Send + 'static,
) -> PriorityBroadcacst {
    let priority_receivers = Arc::new(Mutex::new(None::<oneshot::Sender<String>>));
    let receivers = Arc::new(Mutex::new(Vec::<mpsc::UnboundedSender<String>>::new()));

    let weak_priority_receivers = Arc::downgrade(&priority_receivers);
    let weak_receivers = Arc::downgrade(&receivers);

    tokio::spawn(async move {
        while let Some(Result::Ok(line)) = lines.next().await {
            if let Some(deploy_receivers) = weak_priority_receivers.upgrade() {
                let mut deploy_receivers = deploy_receivers.lock().unwrap();

                let successful_send = if let Some(r) = deploy_receivers.take() {
                    r.send(line.clone()).is_ok()
                } else {
                    false
                };
                drop(deploy_receivers);

                if successful_send {
                    continue;
                }
            }

            if let Some(receivers) = weak_receivers.upgrade() {
                let mut receivers = receivers.lock().unwrap();
                receivers.retain(|receiver| !receiver.is_closed());

                let mut successful_send = false;
                for receiver in receivers.iter() {
                    successful_send |= receiver.send(line.clone()).is_ok();
                }
                if !successful_send {
                    (default)(line);
                }
            } else {
                break;
            }
        }

        if let Some(deploy_receivers) = weak_priority_receivers.upgrade() {
            let mut deploy_receivers = deploy_receivers.lock().unwrap();
            drop(deploy_receivers.take());
        }

        if let Some(receivers) = weak_receivers.upgrade() {
            let mut receivers = receivers.lock().unwrap();
            receivers.clear();
        }
    });

    (priority_receivers, receivers)
}

#[cfg(test)]
mod test {
    use tokio_stream::wrappers::UnboundedReceiverStream;

    use super::*;

    #[tokio::test]
    async fn broadcast_listeners_close_when_source_does() {
        let (tx, rx) = mpsc::unbounded_channel();
        let (_, receivers) = prioritized_broadcast(UnboundedReceiverStream::new(rx), |_| {});

        let (tx2, mut rx2) = mpsc::unbounded_channel();

        receivers.lock().unwrap().push(tx2);

        tx.send(Ok("hello".to_string())).unwrap();
        assert_eq!(rx2.recv().await, Some("hello".to_string()));

        let wait_again = tokio::spawn(async move { rx2.recv().await });

        drop(tx);

        assert_eq!(wait_again.await.unwrap(), None);
    }
}
