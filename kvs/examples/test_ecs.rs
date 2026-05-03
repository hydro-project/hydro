use kvs::testing::run_kvs_test;

const CLUSTER_SIZE: usize = 6;

#[tokio::main]
async fn main() {
    let addr = std::env::args().nth(1).unwrap_or_else(|| {
        "HydroK-NlbBC-kfzBVqu3r0cE-8d96205959a909d2.elb.us-east-1.amazonaws.com:80".to_string()
    });

    println!("Testing KVS at {addr}");

    // Give the cluster a moment to stabilize
    println!("Waiting 5s for cluster initialization...");
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    run_kvs_test(&addr, CLUSTER_SIZE).await;
    println!("All tests passed!");
}
