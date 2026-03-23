use aws_config::SdkConfig;
use aws_sdk_sqs::Client;
use aws_sdk_sqs::types::{DeleteMessageBatchRequestEntry, Message};
use futures::StreamExt as _;
use hydro_lang::live_collections::boundedness::Boundedness;
use hydro_lang::live_collections::stream::{AtLeastOnce, ExactlyOnce, NoOrder, Ordering};
use hydro_lang::location::Location;
use hydro_lang::prelude::*;

pub fn sqs_client<'a, Loc: Location<'a>>(
    sdk_config: Singleton<SdkConfig, Loc, Bounded>,
) -> Singleton<Client, Loc, Bounded> {
    sdk_config.map(q!(|c| Client::new(&c)))
}

/// At-least-once delivery, message ordering isn't preserved.
pub fn source_sqs_queue_standard<'a, Loc: Location<'a>>(
    client: Singleton<Client, Loc, Bounded>,
    queue_url: String,
) -> Stream<String, Loc, Unbounded, NoOrder, AtLeastOnce> {
    client.flat_map_unordered(q!(|client| { sqs_stream(client, queue_url) }))
}

pub fn dest_sqs<'a, Loc: Location<'a>, Bound: Boundedness, Order: Ordering>(
    input: Stream<String, Loc, Bound, Order, ExactlyOnce>,
) {
    todo!();
}

fn sqs_stream(
    client: &Client,
    queue_url: String,
) -> impl 'static + futures::stream::Stream<Item = Message> {
    let recv_msg = client
        .receive_message()
        .queue_url(&*queue_url)
        .wait_time_seconds(10)
        .max_number_of_messages(10);
    let delete_msg = client.delete_message_batch().queue_url(queue_url);
    futures::stream::unfold((), move |()| {
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
    .flat_map(|vec| futures::stream::iter(vec))
}
