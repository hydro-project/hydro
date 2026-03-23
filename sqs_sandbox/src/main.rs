use aws_sdk_sqs::Client;
use aws_sdk_sqs::types::{DeleteMessageBatchRequestEntry, Message};
use futures::StreamExt;
use futures::stream::Stream;

#[tokio::main]
async fn main() {
    let queue_url = std::env::var("SQS_QUEUE_URL").expect("set SQS_QUEUE_URL env var");

    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let client = Client::new(&config);

    // Send
    client
        .send_message()
        .queue_url(&queue_url)
        .message_body("hello from sqs_sandbox!")
        .send()
        .await
        .expect("failed to send message");
    println!("sent message");

    // Receive
    let resp = client
        .receive_message()
        .queue_url(&queue_url)
        .wait_time_seconds(5)
        .max_number_of_messages(1)
        .send()
        .await
        .expect("failed to receive message");

    for msg in resp.messages() {
        println!("received: {}", msg.body().unwrap_or("(empty)"));

        // Delete after processing
        client
            .delete_message()
            .queue_url(&queue_url)
            .receipt_handle(msg.receipt_handle().unwrap())
            .send()
            .await
            .expect("failed to delete message");
    }
}

pub fn sqs_stream(
    client: &Client,
    queue_url: impl Into<String>,
) -> impl 'static + Stream<Item = Message> {
    let queue_url = queue_url.into();
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
