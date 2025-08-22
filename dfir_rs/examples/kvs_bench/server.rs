use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use bincode::options;
use bytes::{BufMut, Bytes, BytesMut};
use dfir_lang::graph::{WriteConfig, WriteGraphType};
use dfir_rs::compiled::pull::HalfMultisetJoinState;
use dfir_rs::dfir_syntax;
use dfir_rs::scheduled::ticks::TickInstant;
use futures::Stream;
use lattices::map_union::{MapUnionHashMap, MapUnionSingletonMap};
use lattices::set_union::SetUnionSingletonSet;
use lattices::{Max, Point, WithBot};
use rand::{Rng, SeedableRng};
use serde::Serialize;
use serde::de::DeserializeSeed;
use tokio::task;
use tokio_stream::StreamExt;

use crate::Topology;
use crate::buffer_pool::BufferPool;
use crate::protocol::{
    KvsRequest, KvsRequestDeserializer, KvsResponse, MyLastWriteWins, MySetUnion, NodeId,
};

pub fn run_server<RX>(
    server_id: usize,
    topology: Topology<RX>,
    dist: f64,
    throughput: Arc<AtomicUsize>,
    write_graph: Option<WriteGraphType>,
    write_config: Option<WriteConfig>,
) where
    RX: Stream<Item = (usize, Bytes)> + StreamExt + Unpin + Send + 'static,
{
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            const BUFFER_SIZE: usize = 1024;

            let buffer_pool = BufferPool::<BUFFER_SIZE>::create_buffer_pool();

            let (process_to_peers_tx, mut process_to_peers_rx) =
                dfir_rs::util::unsync_channel::<(Bytes, NodeId)>(None);

            let (client_to_process_tx, client_to_process_rx) =
                dfir_rs::util::unsync_channel::<(KvsRequest<BUFFER_SIZE>, NodeId)>(None);
            let (process_to_client_tx, mut _process_to_client_rx) =
                dfir_rs::util::unsync_channel::<(KvsResponse<BUFFER_SIZE>, NodeId)>(None);

            let localset = task::LocalSet::new();

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

                                client_to_process_tx.try_send((req, node_id)).unwrap();
                            }
                        }
                    })
                    .await
                    .unwrap()
            }});

            // Handle outgoing peer-to-peer communication
            let outbound_networking_task = localset.run_until({
                let lookup = topology.lookup.clone();

                // TODO: Eventually this would get moved into a dfir operator that would return a Bytes struct and be efficient and zero copy and etc.
                async move {
                    loop {
                        while let Some((serialized_req, node_id)) = process_to_peers_rx.next().await {
                            let index = lookup.binary_search(&node_id).unwrap();
                            topology.tx[index].send(serialized_req).unwrap();
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

            let mut rng = rand::rngs::SmallRng::from_entropy();
            let dist = rand_distr::Zipf::new(1_000_000, dist).unwrap();

            let mut pre_gen_index = 0;
            let pre_gen_random_numbers: Vec<u64> = (0..(128*1024)).map(|_| rng.sample(dist) as u64).collect();

            let create_unique_id = move |server_id: u128, tick: TickInstant, e: u128| -> u128 {
                assert!(tick < TickInstant(1_000_000_000));
                assert!(e < 1_000_000_000);

                (relatively_recent_timestamp.load(Ordering::Relaxed) as u128)
                    .checked_mul(100).unwrap()
                    .checked_add(server_id).unwrap()
                    .checked_mul(1_000_000_000).unwrap()
                    .checked_add(tick.0 as u128).unwrap()
                    .checked_mul(1_000_000_000).unwrap()
                    .checked_add(e).unwrap()
            };

            let mut throughput_internal = 0usize;

            let mut df = dfir_syntax! {

                simulated_put_requests = spin() -> flat_map(|_| {
                    let buffer_pool = buffer_pool.clone();
                    let pre_gen_random_numbers = &pre_gen_random_numbers;
                    std::iter::repeat_with(move || {
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
                    })
                });

                union_puts_and_gossip_requests = union();

                simulated_put_requests -> union_puts_and_gossip_requests;
                source_stream(client_to_process_rx)
                    // -> inspect(|x| println!("{server_id}:{:5}: from peers: {x:?}", context.current_tick()))
                    -> union_puts_and_gossip_requests;

                client_input = union_puts_and_gossip_requests
                    -> enumerate::<'tick>()
                    -> demux(|(e, (req, addr)): (usize, (KvsRequest<BUFFER_SIZE>, NodeId)), var_args!(gets, store, broadcast)| {
                        match req {
                            KvsRequest::Put {key, value} => {
                                throughput_internal += 1;
                                const GATE: usize = 2 * 1024;
                                if throughput_internal.is_multiple_of(GATE) {
                                    throughput.fetch_add(GATE, Ordering::SeqCst);
                                }
                                let marker = create_unique_id(server_id as u128, context.current_tick(), e as u128);

                                broadcast.give((key, MyLastWriteWins::new(
                                    Max::new(marker),
                                    WithBot::new_from(Point::new(value.clone())),
                                )));

                                store.give((key, MyLastWriteWins::new(
                                    Max::new(marker),
                                    WithBot::new_from(Point::new(value)),
                                )));
                            },
                            KvsRequest::Gossip {map} => {
                                for (key, reg) in map.into_reveal() {
                                    store.give((key, reg));
                                }
                            },
                            KvsRequest::Get {key} => gets.give((key, addr)),
                            KvsRequest::Delete {key} => {
                                let marker = create_unique_id(server_id as u128, context.current_tick(), e as u128);

                                broadcast.give((key, MyLastWriteWins::new(
                                    Max::new(marker),
                                    WithBot::default(),
                                )));

                                store.give((key, MyLastWriteWins::new(
                                    Max::new(marker),
                                    WithBot::default(),
                                )));
                            }
                        }
                    });

                peers = cross_join::<'static, 'tick, HalfMultisetJoinState>();
                source_iter(topology.lookup) -> [0]peers;

                ticker = source_interval(Duration::from_millis(100))
                    -> [signal]batcher;

                // broadcast out locally generated changes to other nodes.
                client_input[broadcast]
                    -> map(|(key, reg)| MapUnionSingletonMap::new_from((key, reg)))
                    -> [input]batcher;

                batcher = _lattice_fold_batch::<MapUnionHashMap<_, _>>()
                    -> map(|lattice| {
                        use bincode::Options;
                        let serialization_options = options();
                        let req = KvsRequest::Gossip { map: lattice };
                        let mut serialized = BytesMut::with_capacity(serialization_options.serialized_size(&req).unwrap() as usize);
                        let mut serializer = bincode::Serializer::new((&mut serialized).writer(), options());
                        Serialize::serialize(&req, &mut serializer).unwrap();

                        serialized.freeze()
                    })
                    -> [1]peers;

                peers
                    // -> inspect(|x| println!("{server_id}:{:5}: sending to peers: {x:?}", context.current_tick()))
                    -> for_each(|(node_id, serialized_req)| process_to_peers_tx.try_send((serialized_req, node_id)).unwrap());

                // join for lookups
                lookup = _lattice_join_fused_join::<'static, 'tick, MyLastWriteWins<BUFFER_SIZE>, MySetUnion>();

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
                    -> map(|singleton_map: lattices::map_union::MapUnionSingletonMap<_, lattices::Pair::<MyLastWriteWins<BUFFER_SIZE>, MySetUnion>>| {
                        let lattices::collections::SingletonMap(k, v) = singleton_map.into_reveal();
                        (k, (v.into_reveal()))
                    })
                    -> map(|(key, (reg, gets))| {
                        gets.0.into_iter().map(move |(dest, _seq_num)| {
                            (KvsResponse::GetResponse { key, reg: reg.clone() }, dest)
                        })
                    })
                    -> flatten()
                    // -> inspect(|x| println!("{gossip_addr}:{:5}: Response to client: {x:?}", context.current_tick()))
                    -> for_each(|x| process_to_client_tx.try_send(x).unwrap());

            };

            if let Some(graph) = write_graph {
                let meta_graph = df
                    .meta_graph()
                    .expect("No graph found, maybe failed to parse.");
                meta_graph.open_graph(graph, write_config).unwrap();
            }

            let df_task = df.run_async();

            futures::join!(inbound_networking_task, outbound_networking_task, df_task, f3);
        });
    });
}
