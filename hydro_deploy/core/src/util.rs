use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::{Future, Stream, StreamExt};
use tokio::sync::oneshot;

use crate::ssh::PrefixFilteredChannel;

pub async fn async_retry<T, E, F: Future<Output = Result<T, E>>>(
    mut thunk: impl FnMut() -> F,
    count: usize,
    delay: Duration,
) -> Result<T, E> {
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
    Arc<Mutex<Vec<PrefixFilteredChannel>>>,
);

pub fn prioritized_broadcast<T: Stream<Item = std::io::Result<String>> + Send + Unpin + 'static>(
    mut lines: T,
    default: impl Fn(String) + Send + 'static,
) -> PriorityBroadcacst {
    let priority_receivers = Arc::new(Mutex::new(None::<oneshot::Sender<String>>));
    // Option<String> is the prefix to separate special stdout messages from regular ones
    let receivers = Arc::new(Mutex::new(Vec::<PrefixFilteredChannel>::new()));

    let weak_priority_receivers = Arc::downgrade(&priority_receivers);
    let weak_receivers = Arc::downgrade(&receivers);

    tokio::spawn(async move {
        while let Some(Ok(line)) = lines.next().await {
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
                receivers.retain(|receiver| !receiver.1.is_closed());

                let mut successful_send = false;
                // Send to specific receivers if the filter prefix matches
                for (prefix_filter, receiver) in receivers.iter() {
                    if prefix_filter
                        .as_ref()
                        .map(|prefix| line.starts_with(prefix))
                        .unwrap_or(true)
                    {
                        successful_send |= receiver.send(line.clone()).is_ok();
                    }
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
    use tokio::sync::mpsc;
    use tokio_stream::wrappers::UnboundedReceiverStream;

    use super::*;

    #[tokio::test]
    async fn broadcast_listeners_close_when_source_does() {
        let (tx, rx) = mpsc::unbounded_channel();
        let (_, receivers) = prioritized_broadcast(UnboundedReceiverStream::new(rx), |_| {});

        let (tx2, mut rx2) = mpsc::unbounded_channel();

        receivers.lock().unwrap().push((None, tx2));

        tx.send(Ok("hello".to_string())).unwrap();
        assert_eq!(rx2.recv().await, Some("hello".to_string()));

        let wait_again = tokio::spawn(async move { rx2.recv().await });

        drop(tx);

        assert_eq!(wait_again.await.unwrap(), None);
    }
}
