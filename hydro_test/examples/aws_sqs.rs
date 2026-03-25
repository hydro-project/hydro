use std::sync::Arc;

use clap::{ArgAction, Parser};
use hydro_deploy::{AwsNetwork, Deployment, Host};
use hydro_lang::deploy::TrybuildHost;
use hydro_lang::live_collections::stream::{ExactlyOnce, TotalOrder};
use hydro_lang::location::Location;
use hydro_lang::nondet::nondet;
use hydro_lang::viz::config::GraphConfig;
use hydro_test::aws::source_sdk_config;
use hydro_test::aws::sqs::{dest_sqs, source_sqs_standard, sqs_client};
use stageleft::q;

type HostCreator = Box<dyn Fn(&mut Deployment) -> Arc<dyn Host>>;

// cargo run -p hydro_test --example aws_sqs --all-features -- --queue-url=https://sqs.us-west-1.amazonaws.com/317211446276/hydro_test_queue
#[derive(Parser, Debug)]
struct Args {
    #[clap(flatten)]
    graph: GraphConfig,

    #[arg(long, action = ArgAction::SetTrue)]
    aws: bool,

    /// SQS queue url.
    #[arg(long)]
    queue_url: String,
}

enum ProcessSend {}
enum ProcessRecv {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let mut deployment = Deployment::new();

    let create_host: HostCreator = if args.aws {
        let region = "us-east-1";
        let network = AwsNetwork::new(region, None);

        Box::new(move |deployment| -> Arc<dyn Host> {
            deployment
                .AwsEc2Host()
                .region(region)
                .instance_type("t3.micro")
                .ami("ami-0e95a5e2743ec9ec9") // Amazon Linux 2
                .network(network.clone())
                .add()
        })
    } else {
        let localhost = deployment.Localhost();
        Box::new(move |_| -> Arc<dyn Host> { localhost.clone() })
    };

    let mut flow = hydro_lang::compile::builder::FlowBuilder::new();
    let process_send = flow.process::<ProcessSend>();
    let process_recv = flow.process::<ProcessRecv>();
    {
        let sdk_config = source_sdk_config(&process_recv);
        let sqs_client = sqs_client(sdk_config);
        source_sqs_standard(sqs_client, &*args.queue_url)
            .assume_ordering::<TotalOrder>(
                nondet!(/** Print all messages at least once in arbitrary order */),
            )
            .assume_retries::<ExactlyOnce>(
                nondet!(/** Print all messages at least once in arbitrary order */),
            )
            .for_each(q!(|msg| {
                println!("MSG: {:?} {:?}", msg.message_id(), msg.body());
            }));
    }
    {
        let sdk_config = source_sdk_config(&process_send);
        let sqs_client = sqs_client(sdk_config);
        let input_messages = process_send
            .source_iter(q!(["hello", "world"].repeat(10).into_iter()))
            .map(q!(str::to_owned));
        dest_sqs(sqs_client, input_messages, &*args.queue_url);
    }

    // Extract the IR BEFORE the builder is consumed by deployment methods
    let built = flow.finalize();

    // Generate graph visualizations based on command line arguments
    built.generate_graph_with_config(&args.graph, None)?;

    // If we're just generating a graph file, exit early
    if args.graph.should_exit_after_graph_generation() {
        return Ok(());
    }

    // Now use the built flow for deployment with optimization
    let _nodes = built
        .with_default_optimize()
        .with_process(
            &process_send,
            TrybuildHost::new(create_host(&mut deployment))
                .features(vec!["aws_sqs".to_owned(), "aws".to_owned()]),
        )
        .with_process(
            &process_recv,
            TrybuildHost::new(create_host(&mut deployment))
                .features(vec!["aws_sqs".to_owned(), "aws".to_owned()]),
        )
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();

    deployment.start().await.unwrap();

    tokio::signal::ctrl_c().await.unwrap();
    Ok(())
}

// #[tokio::main]
// async fn main() {
//     use aws_sdk_sqs::Client;
//     use aws_sdk_sqs::types::{DeleteMessageBatchRequestEntry, Message};
//     use futures::StreamExt;
//     use futures::stream::Stream;
//
//     let queue_url = std::env::var("SQS_QUEUE_URL").expect("set SQS_QUEUE_URL env var");

//     let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
//     let client = Client::new(&config);

//     // Send
//     client
//         .send_message()
//         .queue_url(&queue_url)
//         .message_body("hello from sqs_sandbox!")
//         .send()
//         .await
//         .expect("failed to send message");
//     println!("sent message");

//     // Receive
//     let resp = client
//         .receive_message()
//         .queue_url(&queue_url)
//         .wait_time_seconds(5)
//         .max_number_of_messages(1)
//         .send()
//         .await
//         .expect("failed to receive message");

//     for msg in resp.messages() {
//         println!("received: {}", msg.body().unwrap_or("(empty)"));

//         // Delete after processing
//         client
//             .delete_message()
//             .queue_url(&queue_url)
//             .receipt_handle(msg.receipt_handle().unwrap())
//             .send()
//             .await
//             .expect("failed to delete message");
//     }
// }

// pub fn sqs_stream(
//     client: &Client,
//     queue_url: impl Into<String>,
// ) -> impl 'static + Stream<Item = Message> {
//     let queue_url = queue_url.into();
//     let recv_msg = client
//         .receive_message()
//         .queue_url(&*queue_url)
//         .wait_time_seconds(10)
//         .max_number_of_messages(10);
//     let delete_msg = client.delete_message_batch().queue_url(queue_url);
//     futures::stream::unfold((), move |()| {
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
//     .flat_map(|vec| futures::stream::iter(vec))
// }
