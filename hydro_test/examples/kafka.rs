use std::sync::Arc;

use clap::{ArgAction, Parser};
use hydro_deploy::aws::NetworkResources;
use hydro_deploy::{AwsNetwork, Deployment, Host, HostTargetType, LinuxCompileType};
use hydro_lang::deploy::TrybuildHost;
use hydro_lang::live_collections::stream::{ExactlyOnce, TotalOrder};
use hydro_lang::location::Location;
use hydro_lang::nondet::nondet;
use hydro_lang::viz::config::GraphConfig;
use hydro_test::kafka::{dest_kafka, kafka_consumer, kafka_producer};
use stageleft::q;

type HostCreator = Box<dyn Fn(&mut Deployment) -> Arc<dyn Host>>;

const TOPIC_PREFIX: &str = "financial_transactions";

// cargo run -p hydro_test --example kafka --features kafka -- --brokers 'localhost:9092'
#[derive(Parser, Debug)]
struct Args {
    #[clap(flatten)]
    graph: GraphConfig,

    /// Use AWS, make sure credentials are set up
    #[arg(long, action = ArgAction::SetTrue)]
    aws: bool,

    /// Kafka bootstrap servers
    #[arg(long, default_value = "localhost:9092")]
    brokers: String,

    /// Kafka security protocol (plaintext or SSL for MSK)
    #[arg(long, default_value = "plaintext")]
    security_protocol: String,

    /// Run mode: "produce" (produce only, prints topic name), "consume" (consume only, requires --topic), or "both" (default)
    #[arg(long, default_value = "both")]
    mode: String,

    /// Topic name for consume-only mode (use the topic printed by a produce run)
    #[arg(long)]
    topic: Option<String>,

    /// Number of messages to produce
    #[arg(long, default_value = "10000")]
    num_messages: usize,

    /// Number of Kafka partitions for the topic
    #[arg(long, default_value = "10")]
    num_partitions: i32,

    /// Number of consumer instances
    #[arg(long, default_value = "3")]
    num_consumers: usize,

    // --- AWS options ---
    /// AWS region
    #[arg(long, default_value = "us-west-2")]
    aws_region: String,

    /// AWS EC2 instance type
    #[arg(long, default_value = "m7i.large")]
    aws_instance_type: String,

    /// AWS AMI ID (Amazon Linux 2)
    #[arg(long, default_value = "ami-055a9df0c8c9f681c")]
    aws_ami: String,

    /// AWS VPC ID (required for --aws)
    #[arg(long)]
    aws_vpc: Option<String>,

    /// AWS subnet ID (required for --aws)
    #[arg(long)]
    aws_subnet: Option<String>,

    /// AWS security group ID (required for --aws)
    #[arg(long)]
    aws_security_group: Option<String>,
}

enum Leader {}
enum Consumer {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let mut deployment = Deployment::new();

    let create_host: HostCreator = if args.aws {
        let vpc = args
            .aws_vpc
            .as_ref()
            .expect("--aws-vpc required with --aws");
        let subnet = args
            .aws_subnet
            .as_ref()
            .expect("--aws-subnet required with --aws");
        let sg = args
            .aws_security_group
            .as_ref()
            .expect("--aws-security-group required with --aws");
        let region = args.aws_region.clone();
        let instance_type = args.aws_instance_type.clone();
        let ami = args.aws_ami.clone();
        let network = AwsNetwork::new(
            &region,
            Some(NetworkResources::new(
                vpc.clone(),
                subnet.clone(),
                sg.clone(),
            )),
        );

        Box::new(move |deployment| -> Arc<dyn Host> {
            deployment
                .AwsEc2Host()
                .region(&region)
                .instance_type(&instance_type)
                .ami(&ami)
                .network(network.clone())
                .target_type(HostTargetType::Linux(LinuxCompileType::Glibc))
                .add()
        })
    } else {
        let localhost = deployment.Localhost();
        Box::new(move |_| -> Arc<dyn Host> { localhost.clone() })
    };

    let num_messages = args.num_messages;
    let num_partitions = args.num_partitions;
    let num_consumers = args.num_consumers;

    let produce = args.mode == "produce" || args.mode == "both";
    let consume = args.mode == "consume" || args.mode == "both";

    // For consume-only, require --topic; otherwise generate a unique one.
    let topic = if let Some(t) = &args.topic {
        t.clone()
    } else {
        format!("{}_{}", TOPIC_PREFIX, std::process::id())
    };

    let mut flow = hydro_lang::compile::builder::FlowBuilder::new();
    let leader = flow.process::<Leader>();
    let consumers = flow.cluster::<Consumer>();

    // Leader: produce transactions spread across partitions.
    if produce {
        let producer = kafka_producer(
            &leader,
            &args.brokers,
            &args.security_protocol,
            &topic,
            num_partitions,
        );
        let transactions = leader.source_iter(q!({
            (0..num_messages).map(|i| {
                let account = format!("account_{}", i % 100);
                let amount = format!("{}", (i % 201) as i64 - 100); // range [-100, 100]
                (account, amount)
            })
        }));
        let sent = dest_kafka(producer, transactions, &topic);
        sent.for_each(q!({
            let count = std::cell::Cell::new(0usize);
            move |producer| {
                let c = count.get() + 1;
                count.set(c);
                if c >= num_messages {
                    rdkafka::producer::Producer::flush(
                        &*producer,
                        std::time::Duration::from_secs(30),
                    )
                    .expect("Failed to flush producer");
                    println!("PRODUCE_DONE {}", c);
                }
            }
        }));
    }

    // Consumers: read from topic and maintain per-account balances.
    if consume {
        kafka_consumer(
            &consumers,
            &args.brokers,
            "kafka_example_consumers",
            &topic,
            &args.security_protocol,
        )
        .assume_ordering::<TotalOrder>(nondet!(/** Safe: balances are commutative (addition). */))
        .assume_retries::<ExactlyOnce>(nondet!(/** Safe: balances are commutative (addition). */))
        .filter_map(q!(|msg| {
            let key =
                rdkafka::Message::key(&msg).map(|k| String::from_utf8_lossy(k).to_string())?;
            let value =
                rdkafka::Message::payload(&msg).map(|v| String::from_utf8_lossy(v).to_string())?;
            let amount: i64 = value.parse().ok()?;
            Some((key, amount))
        }))
        .for_each(q!({
            let balances = std::cell::RefCell::new(std::collections::HashMap::<String, i64>::new());
            move |(account, amount)| {
                let mut map = balances.borrow_mut();
                let balance = map.entry(account.clone()).or_insert(0);
                *balance += amount;
                println!("{}: {}", account, balance);
            }
        }));
    }

    // Extract the IR BEFORE the builder is consumed by deployment methods
    let built = flow.finalize();

    // Generate graph visualizations based on command line arguments
    if built.generate_graph(&args.graph)?.is_some() {
        return Ok(());
    }

    // Deploy
    let mut hosts_builder = built.with_default_optimize();
    hosts_builder = hosts_builder.with_process(
        &leader,
        TrybuildHost::new(create_host(&mut deployment)).features(vec!["kafka".to_owned()]),
    );
    hosts_builder = hosts_builder.with_cluster(
        &consumers,
        (0..num_consumers).map(|_| {
            TrybuildHost::new(create_host(&mut deployment)).features(vec!["kafka".to_owned()])
        }),
    );
    let nodes = hosts_builder.deploy(&mut deployment);

    deployment.deploy().await.unwrap();
    deployment.start().await.unwrap();

    println!(
        "Running Kafka example (mode={}, topic={topic}, {num_messages} messages)...",
        args.mode
    );

    let start = std::time::Instant::now();
    let total = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let (done_tx, mut done_rx) = tokio::sync::mpsc::channel::<()>(1);
    let (produce_done_tx, produce_done_rx) = tokio::sync::oneshot::channel::<()>();
    {
        use hydro_lang::deploy::DeployCrateWrapper;

        if produce {
            let leader_node = nodes.get_process(&leader);
            let mut leader_out = leader_node.stdout();
            let produce_done_tx = std::sync::Mutex::new(Some(produce_done_tx));
            tokio::spawn(async move {
                while let Some(line) = leader_out.recv().await {
                    if line.starts_with("PRODUCE_DONE") {
                        if let Some(tx) = produce_done_tx.lock().unwrap().take() {
                            let _ = tx.send(());
                        }
                    } else {
                        println!("[Leader] {line}");
                    }
                }
            });
        }

        if consume {
            for (i, member) in nodes
                .get_cluster(&consumers)
                .members()
                .into_iter()
                .enumerate()
            {
                let mut member_out = member.stdout();
                let total = total.clone();
                let done_tx = done_tx.clone();
                tokio::spawn(async move {
                    while let Some(_line) = member_out.recv().await {
                        let t = total.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                        if t.is_multiple_of(1_000) {
                            println!("[Consumer {i}] ... {t} total messages consumed so far");
                        }
                        if t >= num_messages {
                            let _ = done_tx.send(()).await;
                            return;
                        }
                    }
                });
            }
        }
    }
    drop(done_tx);

    if produce {
        let _ = produce_done_rx.await;
        let produce_elapsed = start.elapsed();
        println!(
            "Produce: {num_messages} messages in {:.2?} ({:.0} msgs/sec)",
            produce_elapsed,
            num_messages as f64 / produce_elapsed.as_secs_f64()
        );
        if !consume {
            println!("Topic: {topic}");
            println!("Run consume with: --mode consume --topic {topic}");
            return Ok(());
        }
    }

    if consume {
        done_rx.recv().await;
        let elapsed = start.elapsed();
        println!(
            "Consume: {num_messages} messages in {:.2?} ({:.0} msgs/sec)",
            elapsed,
            num_messages as f64 / elapsed.as_secs_f64()
        );
    }

    Ok(())
}
