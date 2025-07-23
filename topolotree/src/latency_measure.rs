use std::collections::HashMap;
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Instant;

use dfir_rs::bytes::Bytes;
use dfir_rs::util::deploy::{ConnectedDirect, ConnectedSink, ConnectedSource};
use dfir_rs::util::{deserialize_from_bytes, serialize_to_bytes};
use futures::{SinkExt, StreamExt};

mod protocol;
use protocol::*;

#[tokio::main]
async fn main() {
    let ports = dfir_rs::util::deploy::init::<()>().await;
    let mut start_node = ports
        .port("increment_start_node")
        .connect::<ConnectedDirect>()
        .into_sink();

    let mut end_node = ports
        .port("end_node_query")
        .connect::<ConnectedDirect>()
        .into_source();

    let num_clients: u64 = std::env::args().nth(1).unwrap().parse().unwrap();
    let partition_n: u64 = std::env::args().nth(2).unwrap().parse().unwrap();
    let keys_per_partition: u64 = std::env::args().nth(3).unwrap().parse().unwrap();

    let atomic_counter = Arc::new(AtomicU64::new(0));
    let atomic_borrow = atomic_counter.clone();
    let atomic_keep_running = Arc::new(AtomicBool::new(true));
    let atomic_keep_running_clone = atomic_keep_running.clone();
    let (latency_sender, latency_receiver) = mpsc::channel::<u128>();
    let printer_thread = thread::spawn(move || {
        let mut last_instant = Instant::now();
        while atomic_keep_running_clone.load(std::sync::atomic::Ordering::Relaxed) {
            thread::sleep(std::time::Duration::from_millis(100));
            let now = Instant::now();
            let counter = atomic_borrow.swap(0, std::sync::atomic::Ordering::Relaxed);
            let elapsed = now - last_instant;
            last_instant = now;
            println!("throughput,{},{}", counter, elapsed.as_secs_f64());

            while let Ok(latency) = latency_receiver.try_recv() {
                println!("latency,{}", latency);
            }

            std::io::stdout().flush().unwrap()
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
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<i64>();
        queues.push(sender);

        let inc_sender = inc_sender.clone();
        let latency_sender = latency_sender.clone();
        let atomic_counter = atomic_counter.clone();
        let keep_running = atomic_keep_running.clone();
        tokio::spawn(async move {
            #[cfg(debug_assertions)]
            let mut count_tracker = HashMap::new();

            let mut next_base: u64 = 0;

            while keep_running.load(std::sync::atomic::Ordering::Relaxed) {
                let id = (partition_n * keys_per_partition)
                    + ((((next_base % keys_per_partition) / num_clients) * num_clients) + i);
                next_base = next_base.wrapping_add(1);
                let increment = rand::random::<bool>();
                let change = if increment { 1 } else { -1 };
                let start = Instant::now();
                inc_sender
                    .send(serialize_to_bytes(OperationPayload { key: id, change }))
                    .unwrap();

                let received = receiver.recv().await.unwrap();
                #[cfg(debug_assertions)]
                {
                    let count = count_tracker.entry(id).or_insert(0);
                    *count += change;
                    assert_eq!(*count, received);
                }

                if next_base % 100 == 0 {
                    latency_sender.send(start.elapsed().as_micros()).unwrap();
                }

                atomic_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        });
    }

    tokio::spawn(async move {
        loop {
            let updated =
                deserialize_from_bytes::<QueryResponse>(end_node.next().await.unwrap().unwrap())
                    .unwrap();

            if updated.key / keys_per_partition != partition_n {
                continue;
            }

            if queues[((updated.key % keys_per_partition) % num_clients) as usize]
                .send(updated.value)
                .is_err()
            {
                break;
            }
        }
    });

    let mut line = String::new();
    std::io::stdin().read_line(&mut line).unwrap();
    assert!(line.starts_with("stop"));

    atomic_keep_running.store(false, std::sync::atomic::Ordering::Relaxed);
    printer_thread.join().unwrap();

    println!("end");

    std::process::exit(0);
}
