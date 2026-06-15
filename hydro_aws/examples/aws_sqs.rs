use std::sync::Arc;

use clap::{ArgAction, Parser};
use hydro_aws::source_sdk_config;
use hydro_aws::sqs::{dest_sqs, source_sqs_standard, sqs_client};
use hydro_deploy::{AwsNetwork, Deployment, Host};
use hydro_lang::deploy::TrybuildHost;
use hydro_lang::live_collections::stream::{ExactlyOnce, TotalOrder};
use hydro_lang::location::Location;
use hydro_lang::nondet::nondet;
use hydro_lang::viz::config::GraphConfig;
use stageleft::q;

type HostCreator = Box<dyn Fn(&mut Deployment) -> Arc<dyn Host>>;

// aws sqs send-message --queue-url 'https://sqs.<REGION>.amazonaws.com/<ACCOUNT_ID>/<QUEUE_NAME>' --message-body 'foobar'
// AWS_PROFILE='<AWS_PROFILE>' cargo run -p hydro_aws --example aws_sqs --all-features -- --queue-url 'https://sqs.<REGION>.amazonaws.com/<ACCOUNT_ID>/<QUEUE_NAME>'
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
        source_sqs_standard(sqs_client, &args.queue_url)
            .assume_ordering::<TotalOrder>(
                nondet!(/** Safe to assume total order because the side effect is only printing messages for observation/debugging. */),
            )
            .assume_retries::<ExactlyOnce>(
                nondet!(/** Safe to assume exactly-once because the side effect is only printing messages for observation/debugging. */),
            )
            .for_each(q!(|msg| {
                println!("MSG: {:?} {:?}", msg.message_id(), msg.body());
            }));
    }
    {
        let sdk_config = source_sdk_config(&process_send);
        let sqs_client = sqs_client(sdk_config);
        let input_messages = process_send.source_iter(q!(["hello", "world"]
            .repeat(10)
            .into_iter()
            .map(str::to_owned)));
        dest_sqs(sqs_client, input_messages, &args.queue_url);
    }

    // Extract the IR BEFORE the builder is consumed by deployment methods
    let built = flow.finalize();

    // Generate graph visualizations based on command line arguments
    if built.generate_graph(&args.graph)?.is_some() {
        return Ok(());
    }

    // Now use the built flow for deployment with optimization
    let _nodes = built
        .with_default_optimize()
        .with_process(
            &process_send,
            TrybuildHost::new(create_host(&mut deployment)).features(vec!["sqs".to_owned()]),
        )
        .with_process(
            &process_recv,
            TrybuildHost::new(create_host(&mut deployment)).features(vec!["sqs".to_owned()]),
        )
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();

    deployment.start().await.unwrap();

    tokio::signal::ctrl_c().await.unwrap();
    Ok(())
}
