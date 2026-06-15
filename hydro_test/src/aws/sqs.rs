use aws_config::SdkConfig;
use aws_sdk_sqs::Client;
use aws_sdk_sqs::error::SdkError;
use aws_sdk_sqs::operation::receive_message::ReceiveMessageError;
use aws_sdk_sqs::types::{DeleteMessageBatchRequestEntry, Message};
use futures_util::stream::StreamExt as _;
use hydro_lang::live_collections::boundedness::Boundedness;
use hydro_lang::live_collections::stream::{
    AtLeastOnce, ExactlyOnce, NoOrder, Ordering,
};
use hydro_lang::location::Location;
use hydro_lang::prelude::*;

#[ctor::ctor]
fn init_rewrites() {
    stageleft::add_private_reexport(
        vec!["aws_sdk_sqs", "types", "_message"],
        vec!["aws_sdk_sqs", "types"],
    );
}

/// Creates an SQS client from the SDK config.
pub fn sqs_client<'a, Loc: Location<'a>>(
    sdk_config: Singleton<SdkConfig, Loc, Bounded>,
) -> Singleton<Client, Loc, Bounded> {
    // TODO(mingwei): Should this be unbounded since it is a ball of changing state?
    sdk_config.map(q!(|c| Client::new(&c)))
}

/// At-least-once unordered delivery from a _standard_ SQS queue.
pub fn source_sqs_standard<'a, Loc: Location<'a>>(
    client: Singleton<Client, Loc, Bounded>,
    queue_url: &'a str,
) -> Stream<Message, Loc, Bounded, NoOrder, AtLeastOnce> {
    client
        .into_stream()
        .flat_map_stream_blocking(q!(move |client| {
            futures_util::stream::unfold((), move |()| {
                let fut = self::sqs_recv(&client, queue_url);
                async move {
                    let vec = fut.await.expect("Failed to receive from SQS")?;
                    Some((vec, ()))
                }
            })
            .flat_map(futures_util::stream::iter)
        }))
        .weaken_retries()
        .weaken_ordering()
}

/// Delivery from a FIFO SQS queue.
///
/// SQS FIFO only guarantees ordering within a single message group, and
/// duplicates can still occur around visibility-timeout boundaries. Therefore
/// this source conservatively returns `NoOrder` / `AtLeastOnce`. Callers who
/// know their FIFO configuration enforces a single message group and
/// deduplication can `assume_ordering` / `assume_retries` with a [`NonDet`]
/// justification.
///
/// # Non-Determinism
/// The user must ensure `queue_url` is a FIFO queue. If it is not, the output order will be non-deterministic.
pub fn source_sqs_fifo<'a, Loc: Location<'a>>(
    client: Singleton<Client, Loc, Bounded>,
    queue_url: &'a str,
) -> Stream<Message, Loc, Bounded, NoOrder, AtLeastOnce> {
    client
        .into_stream()
        .flat_map_stream_blocking(q!(move |client| {
            futures_util::stream::unfold((), move |()| {
                let fut = self::sqs_recv(&client, queue_url);
                async move {
                    let vec = fut.await.expect("Failed to receive from SQS")?;
                    Some((vec, ()))
                }
            })
            .flat_map(futures_util::stream::iter)
        }))
        .weaken_retries()
        .weaken_ordering()
}

/// Writes messages to a _standard_ SQS queue.
///
/// Does not set `message_group_id`, so sending to a FIFO queue will fail at
/// runtime with `MissingParameter`.
pub fn dest_sqs<'a, Loc: Location<'a>, Bound: Boundedness, Order: Ordering>(
    client: Singleton<Client, Loc, Bounded>,
    input: Stream<String, Loc, Bound, Order, ExactlyOnce>,
    queue_url: &'a str,
) {
    input
        .cross_singleton(client)
        .map(q!(|(message, client)| self::sqs_send(
            &client, queue_url, message
        )))
        .resolve_futures_blocking();
}

// Sends one message to the `queue_url` SQS queue.
fn sqs_send(client: &Client, queue_url: &str, message: String) -> impl use<> + Future<Output = ()> {
    let message = client
        .send_message()
        .queue_url(queue_url)
        .message_body(message);
    async move {
        message.send().await.expect("Failed to send message to SQS");
    }
}

/// Receives and deletes up to 10 messages from the `queue_url` SQS queue.
fn sqs_recv(
    client: &Client,
    queue_url: &str,
) -> impl use<> + Future<Output = Result<Option<Vec<Message>>, SdkError<ReceiveMessageError>>> {
    let recv_msg = client
        .receive_message()
        .queue_url(queue_url)
        .wait_time_seconds(10)
        .max_number_of_messages(10);
    let delete_msg = client.delete_message_batch().queue_url(queue_url);
    async move {
        let output = recv_msg.send().await?;

        let messages = output.messages.unwrap_or_default();
        if messages.is_empty() {
            return Ok(None);
        }

        // TODO(mingwei): Should we give the user control over deletion?
        delete_msg
            .set_entries(Some(
                messages
                    .iter()
                    .enumerate()
                    .map(|(i, msg)| {
                        DeleteMessageBatchRequestEntry::builder()
                            .id(i.to_string())
                            .receipt_handle(msg.receipt_handle().unwrap())
                            .build()
                            .unwrap()
                    })
                    .collect(),
            ))
            .send()
            .await
            .unwrap();

        Ok(Some(messages))
    }
}
