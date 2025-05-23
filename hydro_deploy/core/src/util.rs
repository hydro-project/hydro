use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::{Future, Stream, StreamExt};
use tokio::sync::{mpsc, oneshot};

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

pub type PrefixFilteredChannel = (Option<String>, mpsc::UnboundedSender<String>);

// pub type PriorityBroadcast = (
//     Arc<Mutex<Option<oneshot::Sender<String>>>>,
//     Arc<Mutex<Vec<PrefixFilteredChannel>>>,
// );

#[derive(Clone)]
pub struct PriorityBroadcast(Arc<Mutex<PriorityBroadcastInternal>>);

struct PriorityBroadcastInternal {
    priority_sender: Option<oneshot::Sender<String>>,
    senders: Vec<PrefixFilteredChannel>,
}

impl PriorityBroadcast {
    pub fn receive_priority(&self) -> oneshot::Receiver<String> {
        let mut this = self.0.lock().unwrap();
        if this.priority_sender.is_some() {
            panic!("Only one deploy priority receiver is allowed at a time");
        }

        let (sender, receiver) = oneshot::channel::<String>();
        this.priority_sender = Some(sender);
        receiver
    }

    pub fn receive(&self, prefix: Option<String>) -> mpsc::UnboundedReceiver<String> {
        let mut this = self.0.lock().unwrap();
        let (sender, receiver) = mpsc::unbounded_channel::<String>();
        this.senders.push((prefix, sender));
        receiver
    }
}

pub fn prioritized_broadcast<T: Stream<Item = std::io::Result<String>> + Send + Unpin + 'static>(
    mut lines: T,
    fallback_receiver: impl Fn(String) + Send + 'static,
) -> PriorityBroadcast {
    let internal = Arc::new(Mutex::new(PriorityBroadcastInternal {
        priority_sender: None,
        senders: Vec::new(),
    }));

    let weak_internal = Arc::downgrade(&internal);

    tokio::spawn(async move {
        while let Some(Ok(line)) = lines.next().await {
            let Some(internal) = weak_internal.upgrade() else {
                break;
            };
            let mut internal = internal.lock().unwrap();

            // Priority receiver
            if let Some(priority_sender) = internal.priority_sender.take() {
                if priority_sender.send(line.clone()).is_ok() {
                    continue; // Skip regular receivers if successfully sent to the priority receiver.
                }
            }

            // Regular receivers
            internal.senders.retain(|receiver| !receiver.1.is_closed());

            let mut successful_send = false;
            for (prefix_filter, sender) in internal.senders.iter() {
                // Send to specific receivers if the filter prefix matches
                if prefix_filter
                    .as_ref()
                    .is_none_or(|prefix| line.starts_with(prefix))
                {
                    successful_send |= sender.send(line.clone()).is_ok();
                }
            }

            // If no receivers successfully received the line, use the fallback receiver.
            if !successful_send {
                (fallback_receiver)(line);
            }
        }

        if let Some(internal) = weak_internal.upgrade() {
            let mut internal = internal.lock().unwrap();
            drop(std::mem::take(&mut internal.priority_sender));
            drop(std::mem::take(&mut internal.senders));
        };
    });

    PriorityBroadcast(internal)
}

#[cfg(test)]
mod test {
    use tokio::sync::mpsc;
    use tokio_stream::wrappers::UnboundedReceiverStream;

    use super::*;

    #[tokio::test]
    async fn broadcast_listeners_close_when_source_does() {
        let (tx, rx) = mpsc::unbounded_channel();
        let priority_broadcast = prioritized_broadcast(UnboundedReceiverStream::new(rx), |_| {});

        let mut rx2 = priority_broadcast.receive(None);

        tx.send(Ok("hello".to_string())).unwrap();
        assert_eq!(rx2.recv().await, Some("hello".to_string()));

        let wait_again = tokio::spawn(async move { rx2.recv().await });

        drop(tx);

        assert_eq!(wait_again.await.unwrap(), None);
    }
}
