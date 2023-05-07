use hydroflow::hydroflow_syntax;
use std::time::Duration;
use tokio::task;
use tokio_stream::StreamExt;

use crate::buffer_pool::BufferPool;
use crate::protocol::KvsRequest;
use crate::protocol::KvsRequestDeserializer;
use crate::protocol::KvsResponse;
use crate::protocol::MyLastWriteWins;
use crate::protocol::MySetUnion;
use bincode::options;
use rand::Rng;
use rand::SeedableRng;
use serde::de::DeserializeSeed;
use serde::Serialize;
use std::io::Cursor;
use std::rc::Rc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use crate::protocol::NodeId;
use crate::Topology;
use futures::Stream;
use lattices::bottom::Bottom;
use lattices::fake::Fake;
use lattices::ord::Max;
use lattices::set_union::SetUnionSingletonSet;
use lattices::Merge;

use hydroflow::compiled::pull::HalfMultisetJoinState;

pub fn run_server<RX>(
    server_id: usize,
    topology: Topology<RX>,
    dist: f64,
    throughput: Arc<AtomicUsize>,
    print_mermaid: bool,
) where
    RX: Stream<Item = (usize, Vec<u8>)> + StreamExt + Unpin + Send + 'static,
{
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {

            let buffer_pool = BufferPool::create_buffer_pool();

            let (transducer_to_peers_tx, mut transducer_to_peers_rx) =
                hydroflow::util::unsync_channel::<(KvsRequest, NodeId)>(None);

            let (client_to_transducer_tx, client_to_transducer_rx) =
                hydroflow::util::unsync_channel::<(KvsRequest, NodeId)>(None);
            let (transducer_to_client_tx, mut _transducer_to_client_rx) =
                hydroflow::util::unsync_channel::<(KvsResponse, NodeId)>(None);

            let localset = tokio::task::LocalSet::new();

            let inbound_networking_task = localset.run_until({
                let buffer_pool = buffer_pool.clone();

                async {
                    task::spawn_local({

                        async move {
                            let mut joined_streams = futures::stream::select_all(topology.rx);

                            while let Some((node_id, req)) = joined_streams.next().await {
                                let mut deserializer = bincode::Deserializer::from_slice(&req, options());
                                let req = KvsRequestDeserializer {
                                    collector: Rc::clone(&buffer_pool),
                                }.deserialize(&mut deserializer).unwrap();

                                client_to_transducer_tx.try_send((req, node_id)).unwrap();
                            }
                        }
                    })
                    .await
                    .unwrap()
            }});

            // Handle outgoing peer-to-peer communication
            let outbound_networking_task = localset.run_until({
                let lookup = topology.lookup.clone();

                // TODO: Eventually this would get moved into a hydroflow operator that would return a Bytes struct and be efficient and zero copy and etc.
                async move {
                    loop {
                        while let Some((req, node_id)) = transducer_to_peers_rx.next().await {
                            let index = lookup.binary_search(&node_id).unwrap();

                            let mut serialized = Vec::new();
                            let mut serializer = bincode::Serializer::new(Cursor::new(&mut serialized), options());
                            Serialize::serialize(&req, &mut serializer).unwrap();

                            topology.tx[index].send(serialized).unwrap();
                        }
                    }
                }
            });

            let relatively_recent_timestamp: &'static _ = &*Box::leak(Box::new(std::sync::atomic::AtomicU64::new(
                std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64)));

            let f3 = localset.run_until({
                async move {
                    let mut interval = tokio::time::interval(
                        Duration::from_millis(100),
                    );

                    loop {
                        interval.tick().await;
                        relatively_recent_timestamp.store(std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis() as u64,
                            Ordering::Relaxed);
                    }
                }
            });

            let batch_interval_ticker = tokio_stream::wrappers::IntervalStream::new(tokio::time::interval(
                Duration::from_millis(100),
            ));

            let mut rng = rand::rngs::SmallRng::from_entropy();
            let dist = rand_distr::Zipf::new(1_000_000, dist).unwrap();

            let mut pre_gen_index = 0;
            let pre_gen_random_numbers: Vec<u64> = (0..(128*1024)).map(|_| rng.sample(dist) as u64).collect();

            let create_unique_id = move |server_id: u128, e: u128| -> u128 {
                (relatively_recent_timestamp.load(Ordering::Relaxed) as u128)
                    .saturating_mul(100)
                    .saturating_add(server_id)
                    .saturating_mul(1_000_000_000)
                    .saturating_add(e % 1_000_000_000)
            };

            let mut throughput_internal = 0usize;

            let mut df = hydroflow_syntax! {

                simulated_put_requests = repeat_fn(2000, move || {
                    let value = BufferPool::get_from_buffer_pool(&buffer_pool);

                    // Did the original C++ benchmark do anything with the data..?
                    // Can uncomment this to modify the buffers then.
                    //
                    // let mut borrow = buff.borrow_mut().unwrap();

                    // let mut r = rng.sample(dist_uniform) as u64;
                    // for i in 0..8 {
                    //     borrow[i] = (r % 256) as u8;
                    //     r /= 256;
                    // }

                    let key = pre_gen_random_numbers[pre_gen_index % pre_gen_random_numbers.len()];
                    pre_gen_index += 1;

                    (KvsRequest::Put {
                        key,
                        value,
                    }, 99999999)
                });

                merge_puts_and_gossip_requests = merge();

                simulated_put_requests -> merge_puts_and_gossip_requests;
                source_stream(client_to_transducer_rx)
                    // -> inspect(|x| println!("{server_id}:{:5}: from peers: {x:?}", context.current_tick()))
                    -> merge_puts_and_gossip_requests;

                client_input = merge_puts_and_gossip_requests
                    -> enumerate::<'static>()
                    -> demux(|(e, (req, addr)): (usize, (KvsRequest, NodeId)), var_args!(gets, store, broadcast)| {
                        match req {
                            KvsRequest::Put {key, value} => {
                                throughput_internal += 1;
                                const GATE: usize = 32 * 1024;
                                if std::intrinsics::unlikely(throughput_internal % GATE == 0) {
                                    throughput.fetch_add(GATE, Ordering::SeqCst);
                                }
                                let marker = create_unique_id(server_id as u128, e as u128);

                                broadcast.give((key, MyLastWriteWins::new(
                                    Max::new(marker),
                                    Bottom::new(Fake::new(value.clone())),
                                )));

                                store.give((key, MyLastWriteWins::new(
                                    Max::new(marker),
                                    Bottom::new(Fake::new(value)),
                                )));
                            },
                            KvsRequest::Gossip {key, reg} => {
                                store.give((key, reg));
                            },
                            KvsRequest::Get {key} => gets.give((key, addr)),
                            KvsRequest::Delete {key} => {
                                let marker = create_unique_id(server_id as u128, e as u128);

                                broadcast.give((key, MyLastWriteWins::new(
                                    Max::new(marker),
                                    Bottom::default(),
                                )));

                                store.give((key, MyLastWriteWins::new(
                                    Max::new(marker),
                                    Bottom::default(),
                                )));
                            }
                        }
                    });

                peers = cross_join::<'static, 'tick, HalfMultisetJoinState>();
                source_iter(topology.lookup) -> [0]peers;

                // broadcast out locally generated changes to other nodes.
                client_input[broadcast]
                    -> batch(1000, batch_interval_ticker) // TODO: What we really want here is merging them while batching.
                    -> keyed_reduce::<'tick>(Merge::merge) // TODO: Change to reduce syntax.
                    -> [1]peers;

                peers
                    // -> inspect(|x| println!("{server_id}:{:5}: sending to peers: {x:?}", context.current_tick()))
                    -> map(|(dst, (key, reg))| (dst, KvsRequest::Gossip { key, reg }) )
                    -> for_each(|(node_id, req)| transducer_to_peers_tx.try_send((req, node_id)).unwrap());

                // join for lookups
                lookup = lattice_join::<'static, 'tick, MyLastWriteWins, MySetUnion>();

                client_input[store]
                    // -> inspect(|x| println!("{server_id}:{:5}: stores-into-lookup: {x:?}", context.current_tick()))
                    -> [0]lookup;

                // Feed gets into the join to make them do the actual matching.
                client_input[gets]
                    -> enumerate() // Ensure that two requests from the same client on the same tick do not get merged into a single request.
                    -> map(|(id, (key, addr))| {
                        (key, SetUnionSingletonSet::new_from((addr, id)))
                    })
                    // -> inspect(|x| println!("{gossip_addr}:{:5}: gets-into-lookup: {x:?}", context.current_tick()))
                    -> [1]lookup;

                // Send get results back to user
                lookup
                    -> map(|(key, (reg, gets))| {
                        gets.0.into_iter().map(move |(dest, _seq_num)| {
                            (KvsResponse::GetResponse { key, reg: reg.clone() }, dest)
                        })
                    })
                    -> flatten()
                    // -> inspect(|x| println!("{gossip_addr}:{:5}: Response to client: {x:?}", context.current_tick()))
                    -> for_each(|x| transducer_to_client_tx.try_send(x).unwrap());

            };

            if print_mermaid {
                let serde_graph = df
                    .meta_graph()
                    .expect("No graph found, maybe failed to parse.");

                println!("{}", serde_graph.to_mermaid());
            }

            let hydroflow_task = df.run_async();

            futures::join!(inbound_networking_task, outbound_networking_task, hydroflow_task, f3);
        });
    });
}
