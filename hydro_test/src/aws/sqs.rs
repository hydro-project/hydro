use aws_config::SdkConfig;
use aws_sdk_sqs::Client;
use aws_sdk_sqs::types::{DeleteMessageBatchRequestEntry, Message};
use futures_util::stream::StreamExt as _;
use hydro_lang::live_collections::boundedness::Boundedness;
use hydro_lang::live_collections::stream::{AtLeastOnce, ExactlyOnce, NoOrder, Ordering};
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
) -> Singleton<Client, Loc, Bounded> { // TODO(mingwei): Should this be unbounded since it is a ball of chaning state?
    sdk_config.map(q!(|c| Client::new(&c)))
}

/// At-least-once delivery, message ordering isn't preserved.
pub fn source_sqs_standard<'a, Loc: Location<'a>>(
    client: Singleton<Client, Loc, Bounded>,
    queue_url: &'a str,
) -> Stream<Message, Loc, Bounded, NoOrder, AtLeastOnce> {
    client
        .flat_map_stream_unordered(q!(move |client| {
            let recv_msg = client
                .receive_message()
                .queue_url(queue_url)
                .wait_time_seconds(10)
                .max_number_of_messages(10);
            let delete_msg = client.delete_message_batch().queue_url(queue_url);
            futures_util::stream::unfold((), move |()| {
                let recv_msg = recv_msg.clone();
                let delete_msg = delete_msg.clone();
                async move {
                    let result = recv_msg.send().await;
                    let output = match result {
                        Ok(output) => output,
                        Err(e) => {
                            eprintln!("error receiving message: {e}");
                            return None;
                        }
                    };

                    let messages = output.messages.unwrap_or_default();
                    if messages.is_empty() {
                        return None;
                    }

                    // TODO(mingwei): Should we give the user control over this?
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

                    Some((messages, ()))
                }
            })
            .flat_map(|vec| futures_util::stream::iter(vec))
        }))
        .weaken_retries()
}

// /// Exactly-once in-order delivery.
// ///
// /// The user must ensure `queue_url` has is a FIFO queue.
// pub fn source_sqs_fifo<'a, Loc: Location<'a>>(
//     client: Singleton<Client, Loc, Bounded>,
//     queue_url: &'a str,
// ) -> Stream<Message, Loc, Bounded, TotalOrder, ExactlyOnce> {
//     client.flat_map_stream_ordered(q!(|client| self::sqs_stream(client, queue_url)))
// }

/// Writes messages to SQS.
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

fn sqs_send(client: &Client, queue_url: &str, message: String) -> impl use<> + Future<Output = ()> {
    let message = client
        .send_message()
        .queue_url(queue_url)
        .message_body(message);
    async move {
        message.send().await.expect("Failed to send message to SQS");
    }
}

// // TODO(mingwei): return meaningful error type (once DFIR supports it).
// fn sqs_sink(
//     client: &Client,
//     queue_url: &str,
// ) -> impl use<> + futures::sink::Sink<SendMessageBatchRequestEntry, Error = Infallible> {
//     futures::sink::unfold(
//         (Vec::new(), client.send_message_batch().queue_url(queue_url)),
//         move |(mut buf, send_msg), message| async move {
//             debug_assert!(buf.len() < 10);
//             buf.push(message);
//             if 10 == buf.len() {
//                 let output = (&send_msg)
//                     .clone()
//                     .set_entries(Some(std::mem::take(&mut buf)))
//                     .send()
//                     .await
//                     .expect("Failed to send batch to SQS");
//                 if !output.failed().is_empty() {
//                     let err_msg = output
//                         .failed()
//                         .iter()
//                         .fold(String::new(), |acc, failed| format!("{acc}\n{failed:?}"));
//                     panic!("{}", err_msg);
//                 }
//             }
//             Ok((buf, send_msg))
//         },
//     )
// }

// /// Returns a stream of all available SQS messages. Ends when no more messages are immediately available.
// fn sqs_stream(
//     client: Client,
//     queue_url: &str,
// ) -> impl use<> + futures_util::stream::Stream<Item = Message> {
//     let recv_msg = client
//         .receive_message()
//         .queue_url(queue_url)
//         .wait_time_seconds(10)
//         .max_number_of_messages(10);
//     let delete_msg = client.delete_message_batch().queue_url(queue_url);
//     futures_util::stream::unfold((), move |()| {
//         let recv_msg = recv_msg.clone();
//         let delete_msg = delete_msg.clone();
//         async move {
//             let result = recv_msg.send().await;
//             let output = match result {
//                 Ok(output) => output,
//                 Err(e) => {
//                     eprintln!("error receiving message: {e}");
//                     return None;
//                 }
//             };

//             let messages = output.messages.unwrap_or_default();

//             delete_msg
//                 .set_entries(Some(
//                     messages
//                         .iter()
//                         .enumerate()
//                         .map(|(i, msg)| {
//                             DeleteMessageBatchRequestEntry::builder()
//                                 .id(i.to_string())
//                                 .receipt_handle(msg.receipt_handle().unwrap())
//                                 .build()
//                                 .unwrap()
//                         })
//                         .collect(),
//                 ))
//                 .send()
//                 .await
//                 .unwrap();

//             Some((messages, ()))
//         }
//     })
//     .flat_map(|vec| futures_util::stream::iter(vec))
// }
