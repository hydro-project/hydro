use std::sync::atomic::AtomicU64;
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Instant;

use futures::{SinkExt, StreamExt};
use hydroflow::bytes::Bytes;
use hydroflow::serde::{Deserialize, Serialize};
use hydroflow::tokio;
use hydroflow::util::cli::{ConnectedBidi, ConnectedSink, ConnectedSource};
use hydroflow::util::{deserialize_from_bytes, serialize_to_bytes};

#[derive(Serialize, Deserialize, Clone, Debug)]
struct IncrementRequest {
    tweet_id: u64,
    likes: i32,
}

#[tokio::main]
async fn main() {
    let mut ports = hydroflow::util::cli::init().await;
    let mut start_node = ports
        .port("increment_start_node")
        .connect::<ConnectedBidi>()
        .await
        .into_sink();

    let mut end_node = ports
        .port("end_node_query")
        .connect::<ConnectedBidi>()
        .await
        .into_source();

    let num_clients: Vec<usize> = serde_json::from_str(&std::env::args().nth(1).unwrap()).unwrap();
    let num_clients = num_clients[0];

    let atomic_counter = Arc::new(AtomicU64::new(0));
    let atomic_borrow = atomic_counter.clone();
    let (latency_sender, latency_receiver) = mpsc::channel::<u128>();
    thread::spawn(move || {
        let mut last_instant = Instant::now();
        loop {
            thread::sleep(std::time::Duration::from_millis(100));
            let now = Instant::now();
            let counter = atomic_borrow.swap(0, std::sync::atomic::Ordering::Relaxed);
            let elapsed = now - last_instant;
            last_instant = now;
            println!("throughput,{},{}", counter, elapsed.as_secs_f64());

            while let Ok(latency) = latency_receiver.try_recv() {
                println!("latency,{}", latency);
            }
        }
    });

    let (inc_sender, mut inc_receiver) = tokio::sync::mpsc::unbounded_channel::<Bytes>();
    tokio::spawn(async move {
        loop {
            let value = inc_receiver.recv().await.unwrap();
            start_node.send(value).await.unwrap();
        }
    });

    let mut queues = vec![];

    for i in 0..num_clients {
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<i32>();
        queues.push(sender);

        let inc_sender = inc_sender.clone();
        let latency_sender = latency_sender.clone();
        let atomic_counter = atomic_counter.clone();
        tokio::spawn(async move {
            loop {
                let id = ((rand::random::<u64>() % 1024) / (num_clients as u64))
                    * (num_clients as u64)
                    + (i as u64);
                let increment = rand::random::<bool>();
                let start = Instant::now();
                inc_sender
                    .send(serialize_to_bytes(IncrementRequest {
                        tweet_id: id,
                        likes: if increment { 1 } else { -1 },
                    }))
                    .unwrap();

                receiver.recv().await.unwrap();

                latency_sender.send(start.elapsed().as_micros()).unwrap();

                atomic_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        });
    }

    tokio::spawn(async move {
        loop {
            let updated =
                deserialize_from_bytes::<(u64, i32)>(end_node.next().await.unwrap().unwrap())
                    .unwrap();
            if queues[(updated.0 % (num_clients as u64)) as usize]
                .send(updated.1)
                .is_err()
            {
                break;
            }
        }
    });

    let mut line = String::new();
    std::io::stdin().read_line(&mut line).unwrap();
    assert!(line.starts_with("stop"));
    std::process::exit(0);
}
