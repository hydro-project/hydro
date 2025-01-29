use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;

use dfir_rs::dfir_syntax;
use dfir_rs::futures::{Sink, Stream};
use dfir_rs::itertools::{chain, Itertools};
use dfir_rs::lattices::map_union::{MapUnionHashMap, MapUnionSingletonMap};
use dfir_rs::lattices::Lattice;
use dfir_rs::scheduled::graph::Dfir;
use lattices::map_union::MapUnion;
use either::Either;
use lattices::cc_traits::{covariant_item_ref, Collection, CollectionRef, Get, Iter, Len};
use lattices::map_union::KeyedBimorphism;
use lattices::set_union::{SetUnion, SetUnionHashSet};
use lattices::{IsTop, Pair, Max, PairBimorphism};
use lazy_static::lazy_static;
use prometheus::{register_int_counter, IntCounter};
use rand::distributions::Alphanumeric;
use rand::seq::IteratorRandom;
use rand::{thread_rng, Rng};
use rand_distr::{Distribution, Zipf};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use smallvec::{smallvec, Array, SmallVec};
use tracing::{info, trace};

use crate::lattices::BoundedSetLattice;
use crate::membership::{MemberData, MemberId};
use crate::model::{all_rows, upsert_row, Clock, Namespaces, RowValue, SingleWrite};
use crate::util::{ClientRequestWithAddress, GossipRequestWithAddress};
use crate::GossipMessage::{Ack, Nack};
use crate::{ClientRequest, ClientResponse, GossipMessage, Key, Namespace};

/// A trait that represents an abstract network address. In production, this will typically be
/// SocketAddr.
pub trait Address: Hash + Debug + Clone + Eq + Serialize {}
impl<A> Address for A where A: Hash + Debug + Clone + Eq + Serialize {}

#[derive(Debug, Eq, PartialEq, Hash, Clone, Serialize, Deserialize)]
pub struct SeedNode<A>
where
    A: Address,
{
    pub id: MemberId,
    pub address: A,
}

#[derive(Debug, Clone, Lattice)]
pub struct InfectingWrite {
    write: SetUnion<SmallVecSet<[SingleWrite<Clock>; 4]>>,
    members: BoundedSetLattice<MemberId, 2>,
}

#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SmallVecSet<T: Array>
where
    T::Item: Clone + Debug + Eq + Serialize + for <'a> Deserialize<'a>,
{
    inner: SmallVec<T>,
}

impl<T: Array> SmallVecSet<T>
where
    T::Item: Clone + Debug + Eq + Serialize + for <'a> Deserialize<'a>,
{
    pub fn new() -> Self {
        SmallVecSet {
            inner: SmallVec::new(),
        }
    }

    pub fn from(inner: SmallVec<T>) -> Self {
        SmallVecSet { inner }
    }
}

impl<T: Array> Default for SmallVecSet<T>
where
    T::Item: Clone + Debug + Eq + Serialize + for <'a> Deserialize<'a>,
{
    fn default() -> Self {
        SmallVecSet {
            inner: SmallVec::new(),
        }
    }
}

impl<T: Array> Extend<T::Item> for SmallVecSet<T>
where
    T::Item: Clone + Debug + Eq + Serialize + for <'a> Deserialize<'a>,
{
    fn extend<I: IntoIterator<Item = T::Item>>(&mut self, iter: I) {
        self.inner.extend(iter)
    }
}

impl<T: Array> Len for SmallVecSet<T>
where
    T::Item: Clone + Debug + Eq + Serialize + for <'a> Deserialize<'a>,
{
    fn len(&self) -> usize {
        self.inner.len()
    }

    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl<T: Array> CollectionRef for SmallVecSet<T>
where
    T::Item: Clone + Debug + Eq + Serialize + for <'a> Deserialize<'a>,
{
    type ItemRef<'a>
        = &'a Self::Item
    where
        Self: 'a;

    covariant_item_ref!();
}

impl<T: Array> Collection for SmallVecSet<T>
where
    T::Item: Clone + Debug + Eq + Serialize + for <'a> Deserialize<'a>,
{
    type Item = T::Item;
}
impl<T: Array> Iter for SmallVecSet<T>
where
    T::Item: Clone + Debug + Eq + Serialize + for <'a> Deserialize<'a>,
{
    type Iter<'a>
        = std::slice::Iter<'a, T::Item>
    where
        Self: 'a;

    fn iter(&self) -> Self::Iter<'_> {
        self.inner.iter()
    }
}

impl<T: Array> IntoIterator for SmallVecSet<T>
where
    T::Item: Clone + Debug + Eq + Serialize + for <'a> Deserialize<'a>,
{
    type Item = T::Item;
    type IntoIter = smallvec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<'a, Q, T: Array> Get<&'a Q> for SmallVecSet<T>
where
    T::Item: Clone + Debug + Eq + Serialize + for <'b> Deserialize<'b>,
    Q: Eq + ?Sized  + PartialEq<<T as Array>::Item>,
{
    fn get(&self, key: &'a Q) -> Option<Self::ItemRef<'_>> {
        self.inner.iter().find(|&k| key == k)
    }
}

impl<T: Array> FromIterator<T::Item> for SmallVecSet<T>
where
    T::Item: Clone + Debug + Eq + Serialize + for <'a> Deserialize<'a>,
{
    #[inline]
    fn from_iter<I: IntoIterator<Item = T::Item>>(iterable: I) -> SmallVecSet<T> {
        let mut v = SmallVecSet::new();
        v.extend(iterable);
        v
    }
}

pub type MessageId = String;

lazy_static! {
    pub static ref SETS_COUNTER: IntCounter =
        register_int_counter!("sets", "Counts the number of SET requests processed.").unwrap();
}

/// Creates a L0 key-value store server using Hydroflow.
///
/// # Arguments
/// -- `client_inputs`: The input stream of client requests for the client protocol.
/// -- `client_outputs`: The output sink of client responses for the client protocol.
/// -- `member_info`: The membership information of the server.
/// -- `seed_nodes`: A list of seed nodes that can be used to bootstrap the gossip cluster.
#[expect(clippy::too_many_arguments)]
pub fn server<
    ClientInput,
    ClientOutput,
    ClientOutputError,
    GossipInput,
    GossipOutput,
    GossipOutputError,
    GossipTrigger,
    SeedNodeStream,
    Addr,
>(
    client_inputs: ClientInput,
    client_outputs: ClientOutput,
    gossip_inputs: GossipInput,
    gossip_outputs: GossipOutput,
    gossip_trigger: GossipTrigger,
    member_info: MemberData<Addr>,
    seed_nodes: Vec<SeedNode<Addr>>,
    seed_node_stream: SeedNodeStream,
) -> Dfir<'static>
where
    ClientInput: Stream<Item = (ClientRequest, Addr)> + Unpin + 'static,
    ClientOutput: Sink<(ClientResponse, Addr), Error = ClientOutputError> + Unpin + 'static,
    GossipInput: Stream<Item = (GossipMessage, Addr)> + Unpin + 'static,
    GossipOutput: Sink<(GossipMessage, Addr), Error = GossipOutputError> + Unpin + 'static,
    GossipTrigger: Stream<Item = ()> + Unpin + 'static,
    SeedNodeStream: Stream<Item = Vec<SeedNode<Addr>>> + Unpin + 'static,
    Addr: Address + DeserializeOwned + 'static,
    ClientOutputError: Debug + 'static,
    GossipOutputError: Debug + 'static,
{
    let my_member_id = member_info.id.clone();
    // TODO: This is ugly, but the only way this works at the moment.
    let member_id_2 = my_member_id.clone();
    let member_id_3 = my_member_id.clone();
    let member_id_4 = my_member_id.clone();
    let member_id_5 = my_member_id.clone();
    let member_id_6 = my_member_id.clone();

    let zipf = Zipf::new(1_000_000, 4.0).unwrap();
    let mut rng = thread_rng();

    let keys = (0..1_000_000)
        .map(|i| Key {
            namespace: Namespace::User,
            table: "table".to_string(),
            row_key: i.to_string(),
        })
        .collect::<Vec<_>>();

    let pre_generated_random_idx: Vec<u64> = (0..128 * 1024)
        .map(|_| zipf.sample(&mut rng) as u64)
        .collect();
    let mut pre_gen_index = 0;

    let pre_gen_values: Vec<_> = (0..128 * 1024)
        .map(|_| {
            // BufferPool::get_from_buffer_pool(&buffer_pool)
            // Generate random 1024 byte String
            let mut rng = thread_rng();
            (0..1024)
                .map(|_| rng.sample(Alphanumeric))
                .map(char::from)
                .collect::<String>()
        })
        .collect();

    dfir_syntax! {

        on_start = initialize() -> tee();
        on_start -> for_each(|_| info!("{:?}: Transducer {} started.", context.current_tick(), member_id_6));

        seed_nodes = source_stream(seed_node_stream)
            -> fold::<'static>(|| Box::new(seed_nodes), |last_seed_nodes, new_seed_nodes: Vec<SeedNode<Addr>>| {
                **last_seed_nodes = new_seed_nodes;
                info!("Updated seed nodes: {:?}", **last_seed_nodes);
            });

        // Setup member metadata for this process.
        on_start -> map(|_| upsert_row(Clock::new(0), Key::new(Namespace::System, "members".to_string(), my_member_id.clone()), serde_json::to_string(&member_info).unwrap()))
            -> writes;

        client_out =
            inspect(|(resp, addr)| trace!("{:?}: Sending response: {:?} to {:?}.", context.current_tick(), resp, addr))
            -> dest_sink(client_outputs);

        client_in = source_stream(client_inputs)
            -> map(|(msg, addr)| ClientRequestWithAddress::from_request_and_address(msg, addr))
            -> demux_enum::<ClientRequestWithAddress<Addr>>();

        client_in[Get]
            -> inspect(|req| trace!("{:?}: Received Get request: {:?}.", context.current_tick(), req))
            -> map(|(key, addr) : (Key, Addr)| {
                MapUnionHashMap::new_from([
                        (
                            key,
                            SetUnionHashSet::new_from([addr /* to respond with the result later*/])
                        ),
                ])
            })
            -> reads;

        client_in[Set]
            -> inspect(|request| trace!("{:?}: Received Set request: {:?}.", context.current_tick(), request))
            -> map(|(key, value, _addr) : (_, _, Addr)| upsert_row(Clock::new(context.current_tick().0), key, value))
            -> inspect(|_| {
                SETS_COUNTER.inc(); // Bump SET metrics
            })
            -> writes;

        simulated_puts = repeat_fn(20000, move || {
            let next_index = pre_gen_index % pre_generated_random_idx.len();
            pre_gen_index += 1;
            upsert_row(Clock::new(pre_gen_index as u64), keys[next_index].clone(), pre_gen_values[next_index].clone())
        })
            -> inspect (|_| {
                SETS_COUNTER.inc();
            })
            -> writes;

        client_in[Delete]
            -> null();
            // -> inspect(|req| trace!("{:?}: Received Delete request: {:?}.", context.current_tick(), req))
            // -> map(|(key, _addr) : (Key, Addr)| delete_row(Clock::new(context.current_tick().0), key.namespace, key.table, key.row_key))
            // -> writes;

        gossip_in = source_stream(gossip_inputs)
            -> map(|(msg, addr)| GossipRequestWithAddress::from_request_and_address(msg, addr))
            -> demux_enum::<GossipRequestWithAddress<Addr>>();

        incoming_gossip_messages = gossip_in[Gossip]
            -> inspect(|request| trace!("{:?}: Received gossip request: {:?}.", context.current_tick(), request))
            -> tee();

        gossip_in[Ack]
            -> inspect(|request| trace!("{:?}: Received gossip ack: {:?}.", context.current_tick(), request))
            -> null();

        gossip_in[Nack]
            -> inspect(|request| trace!("{:?}: Received gossip nack: {:?}.", context.current_tick(), request))
            -> map( |(message_id, member_id, _addr)| {
                MapUnionSingletonMap::new_from((message_id, InfectingWrite { write: SetUnion::new(SmallVecSet::new()), members: BoundedSetLattice::new_from([member_id]) }))
            })
            -> infecting_writes;

        gossip_out = union() -> dest_sink(gossip_outputs);

        incoming_gossip_messages
            -> flat_map(|(_msg_id, _member_id, writes, _addr) : (_, _, SmallVec<[SingleWrite<Clock>; 4]>, _)| writes.into_iter() )
            -> writes;

        gossip_processing_pipeline = incoming_gossip_messages
            -> map(|(msg_id, _member_id, writes, sender_address) : (String, MemberId, SmallVec<[SingleWrite<Clock>; 4]>, Addr)| {
                let namespaces = &#namespaces;
                let all_data = namespaces.as_reveal_ref();

                // Check if any of the data is new
                /* TODO: This logic is duplicated in MapUnion::Merge and ideally should be accessed
                   from the pass-through streaming output from `state`. See
                   https://www.notion.so/hydro-project/Proposal-for-State-API-10a2a586262f8080b981d1a2948a69ac
                   for more. */
                let gossip_has_new_data = writes.iter()
                    .any(|single_write| {

                        let single_write_inner = single_write.as_reveal_ref();
                        let new_ts = single_write_inner.1.as_reveal_ref().0;
                        let existing_ts = &all_data.get(&single_write_inner.0).map(|it| it.key);

                        if let Some(existing_ts) = existing_ts {
                            trace!("Comparing timestamps: {:?} vs {:?}", new_ts, existing_ts);
                            new_ts > existing_ts
                        } else {
                            true
                        }
                    });

                if gossip_has_new_data {
                    (Ack { message_id: msg_id, member_id: member_id_2.clone()}, sender_address, Some(writes))
                } else {
                    (Nack { message_id: msg_id, member_id: member_id_3.clone()}, sender_address, None)
                }
             })
            -> tee();

        gossip_processing_pipeline
            -> map(|(response, address, _writes)| (response, address))
            -> inspect( |(msg, addr)| trace!("{:?}: Sending gossip response: {:?} to {:?}.", context.current_tick(), msg, addr))
            -> gossip_out;

        gossip_processing_pipeline
            -> filter_map(|(_, _, writes)| writes)
            -> flat_map(|writes : SmallVec<[SingleWrite<Clock>; 4]>| writes.into_iter())
            -> writes;

        writes = union();

        writes -> namespaces;

        namespaces = state::<'static, Namespaces::<Clock>>();
        new_writes = namespaces -> tee(); // TODO: Use the output from here to generate NACKs / ACKs

        reads = state::<'tick, MapUnionHashMap<Key, SetUnionHashSet<Addr>>>();

        new_writes -> [0]process_system_table_reads;
        reads -> [1]process_system_table_reads;

        process_system_table_reads = lattice_bimorphism(KeyedBimorphism::<HashMap<_, _>, _>::new(PairBimorphism), #namespaces, #reads)
            -> lattice_reduce::<'tick>() // TODO: This can be removed if we fix https://github.com/hydro-project/hydroflow/issues/1401. Otherwise the result can be returned twice if get & gossip arrive in the same tick.
            -> flat_map(|result: MapUnionHashMap<Key, Pair<RowValue<Clock>, SetUnion<HashSet<Addr>>>>| {

                let mut response: Vec<(ClientResponse, Addr)> = vec![];

                    let result = result.as_reveal_ref();

                    for(key, results) in result.iter() {


                        let (value, addresses) = results.as_reveal_ref();
                        let all_values = value.as_reveal_ref().1.as_reveal_ref();
                        let socket_addr = addresses.as_reveal_ref().iter().find_or_first(|_| true).unwrap();
                        response.push((ClientResponse::Get {key: key.clone(), value: all_values.iter().map(ToOwned::to_owned).collect()}, socket_addr.clone()));
                    }
                response
            }) -> client_out;

        new_writes -> for_each(|x| trace!("NEW WRITE: {:?}", x));

        // Step 1: Put the new writes in a map, with the write as the key and a SetBoundedLattice as the value.
        infecting_writes = union() -> state_by::<'static, MapUnionHashMap<MessageId, InfectingWrite>>(std::convert::identity, {|| MapUnion::new(HashMap::with_capacity(1_000_000_000)) } );

        new_writes -> map(|write| {
            // Ideally, the write itself is the key, but writes are a hashmap and hashmaps don't
            // have a hash implementation. So we just generate a GUID identifier for the write
            // for now.
            let id = uuid::Uuid::new_v4().to_string();
            MapUnionSingletonMap::new_from((id, InfectingWrite { write: SetUnion::new(SmallVecSet::from(smallvec![write])), members: BoundedSetLattice::new() }))
        }) -> infecting_writes;

        gossip_trigger = source_stream(gossip_trigger);

        gossip_messages = gossip_trigger
        -> flat_map( |_|
            {
                let infecting_writes = #infecting_writes.as_reveal_ref().clone();
                trace!("{:?}: Currently gossipping {} infecting writes.", context.current_tick(), infecting_writes.iter().filter(|(_, write)| !write.members.is_top()).count());
                infecting_writes
            }
        )
        -> filter(|(_id, infecting_write)| !infecting_write.members.is_top())
        -> map(|(id, infecting_write)| {
            trace!("{:?}: Choosing a peer to gossip to. {:?}:{:?}", context.current_tick(), id, infecting_write);

            let ns = &#namespaces;
            let namespaces_inner_tree = ns.as_reveal_ref();

            let lefts = namespaces_inner_tree
                .range(all_rows(Namespace::System, "members".to_string()))
                .filter(|(key, _value)| key.row_key != member_id_5)
                .map(|(key, value)| Either::Left((key, value)));

            let seed_nodes = &#seed_nodes;

            let rights = seed_nodes.iter()
                .filter(|seed_node| seed_node.id != member_id_5)
                .map(Either::Right);


            let combined = chain!(lefts, rights);

            let chosen_peer = combined.choose(&mut thread_rng());

            match chosen_peer {
                None => {
                    trace!("No peer was chosen for Gossip.");
                    None
                },
                Some(chosen_peer) => {
                    let (chosen_peer_name, chosen_peer_address) = match chosen_peer {
                        Either::Left((key, value)) => {
                            // TODO: We could be reading multiple values here.
                            let peer_info_value = value.as_reveal_ref().1.as_reveal_ref().inner.first().unwrap();
                            let peer_info_deserialized = serde_json::from_str::<MemberData<Addr>>(peer_info_value).unwrap();
                            let peer_endpoint = peer_info_deserialized.protocols.iter().find(|protocol| protocol.name == "gossip").unwrap().clone().endpoint;
                            (key.row_key.clone(), peer_endpoint)
                        },
                        Either::Right(seed_node) => (seed_node.id.clone(), seed_node.address.clone())
                    };

                    trace!("Chosen peer: {:?}:{:?}", chosen_peer_name, chosen_peer_address);
                    Some((id, infecting_write, chosen_peer_address))
                }
            }


        })
        -> flatten()
        -> inspect(|(message_id, infecting_write, peer_gossip_address)| trace!("{:?}: Sending write:\nMessageId:{:?}\nWrite:{:?}\nPeer Address:{:?}", context.current_tick(), message_id, infecting_write, peer_gossip_address))
        -> map(|(message_id, infecting_write, peer_gossip_address): (String, InfectingWrite, Addr)| {
            let gossip_request = GossipMessage::Gossip {
                message_id: message_id.clone(),
                member_id: member_id_4.clone(),
                writes: infecting_write.write.as_reveal_ref().inner.clone(),
            };
            (gossip_request, peer_gossip_address)
        })
        -> gossip_out;
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use dfir_rs::tokio_stream::empty;
    use dfir_rs::util::simulation::{Address, Fleet, Hostname};

    use super::*;
    use crate::membership::{MemberDataBuilder, Protocol};

    #[dfir_rs::test]
    async fn test_member_init() {
        let mut fleet = Fleet::new();

        let server_name: Hostname = "server".to_string();

        let server_client_address = Address::new(server_name.clone(), "client".to_string());
        let server_gossip_address = Address::new(server_name.clone(), "gossip".to_string());

        let (_, gossip_trigger_rx) = dfir_rs::util::unbounded_channel::<()>();

        // Create the kv server
        fleet.add_host(server_name.clone(), |ctx| {
            let client_input = ctx.new_inbox::<ClientRequest>("client".to_string());
            let client_output = ctx.new_outbox::<ClientResponse>("client".to_string());

            let gossip_input = ctx.new_inbox::<GossipMessage>("gossip".to_string());
            let gossip_output = ctx.new_outbox::<GossipMessage>("gossip".to_string());

            let member_data = MemberDataBuilder::new(server_name.clone())
                .add_protocol(Protocol::new(
                    "client".into(),
                    server_client_address.clone(),
                ))
                .add_protocol(Protocol::new(
                    "gossip".into(),
                    server_gossip_address.clone(),
                ))
                .build();

            server(
                client_input,
                client_output,
                gossip_input,
                gossip_output,
                gossip_trigger_rx,
                member_data,
                vec![],
                empty(),
            )
        });

        let client_name: Hostname = "client".to_string();

        let key = "/sys/members/server".parse::<Key>().unwrap();

        let (trigger_tx, trigger_rx) = dfir_rs::util::unbounded_channel::<()>();
        let (response_tx, mut response_rx) = dfir_rs::util::unbounded_channel::<ClientResponse>();

        let key_clone = key.clone();
        let server_client_address_clone = server_client_address.clone();

        fleet.add_host(client_name.clone(), |ctx| {
            let client_tx = ctx.new_outbox::<ClientRequest>("client".to_string());
            let client_rx = ctx.new_inbox::<ClientResponse>("client".to_string());

            dfir_syntax! {

                client_output = dest_sink(client_tx);

                source_stream(trigger_rx)
                    -> map(|_| (ClientRequest::Get { key: key_clone.clone() }, server_client_address_clone.clone()) )
                    -> client_output;

                client_input = source_stream(client_rx)
                    -> for_each(|(resp, _addr)| response_tx.send(resp).unwrap());

            }
        });

        // Send a trigger to the client to send a get request.
        trigger_tx.send(()).unwrap();

        let expected_member_data = MemberDataBuilder::new(server_name.clone())
            .add_protocol(Protocol::new(
                "client".to_string(),
                server_client_address.clone(),
            ))
            .add_protocol(Protocol::new(
                "gossip".to_string(),
                server_gossip_address.clone(),
            ))
            .build();

        loop {
            fleet.run_single_tick_all_hosts().await;

            let responses = dfir_rs::util::collect_ready_async::<Vec<_>, _>(&mut response_rx).await;

            if !responses.is_empty() {
                assert_eq!(
                    responses,
                    &[(ClientResponse::Get {
                        key: key.clone(),
                        value: HashSet::from([
                            serde_json::to_string(&expected_member_data).unwrap()
                        ])
                    })]
                );
                break;
            }
        }
    }

    #[dfir_rs::test]
    async fn test_multiple_values_same_tick() {
        let mut fleet = Fleet::new();

        let server_name: Hostname = "server".to_string();

        let server_client_address = Address::new(server_name.clone(), "client".to_string());

        let (_, gossip_trigger_rx) = dfir_rs::util::unbounded_channel::<()>();

        // Create the kv server
        fleet.add_host(server_name.clone(), |ctx| {
            let client_input = ctx.new_inbox::<ClientRequest>("client".to_string());
            let client_output = ctx.new_outbox::<ClientResponse>("client".to_string());

            let gossip_input = ctx.new_inbox::<GossipMessage>("gossip".to_string());
            let gossip_output = ctx.new_outbox::<GossipMessage>("gossip".to_string());
            let server_gossip_address = Address::new(server_name.clone(), "gossip".to_string());

            let member_data = MemberDataBuilder::new(server_name.clone())
                .add_protocol(Protocol::new(
                    "client".into(),
                    server_client_address.clone(),
                ))
                .add_protocol(Protocol::new(
                    "gossip".into(),
                    server_gossip_address.clone(),
                ))
                .build();

            server(
                client_input,
                client_output,
                gossip_input,
                gossip_output,
                gossip_trigger_rx,
                member_data,
                vec![],
                empty(),
            )
        });

        let key = Key {
            namespace: Namespace::System,
            table: "table".to_string(),
            row_key: "row".to_string(),
        };
        let val_a = "A".to_string();
        let val_b = "B".to_string();

        let writer_name: Hostname = "writer".to_string();

        let (writer_trigger_tx, writer_trigger_rx) = dfir_rs::util::unbounded_channel::<String>();
        let key_clone = key.clone();
        let server_client_address_clone = server_client_address.clone();

        fleet.add_host(writer_name.clone(), |ctx| {
            let client_tx = ctx.new_outbox::<ClientRequest>("client".to_string());
            dfir_syntax! {
                client_output = dest_sink(client_tx);

                source_stream(writer_trigger_rx)
                    -> map(|value| (ClientRequest::Set { key: key_clone.clone(), value: value.clone()}, server_client_address_clone.clone()) )
                    -> client_output;
            }
        });

        // Send two messages from the writer.
        let writer = fleet.get_host_mut(&writer_name).unwrap();
        writer_trigger_tx.send(val_a.clone()).unwrap();
        writer.run_tick();

        writer_trigger_tx.send(val_b.clone()).unwrap();
        writer.run_tick();

        // Transmit messages across the network.
        fleet.process_network().await;

        // Run the server.
        let server = fleet.get_host_mut(&server_name).unwrap();
        server.run_tick();

        // Read the value back.
        let reader_name: Hostname = "reader".to_string();

        let (reader_trigger_tx, reader_trigger_rx) = dfir_rs::util::unbounded_channel::<()>();
        let (response_tx, mut response_rx) = dfir_rs::util::unbounded_channel::<ClientResponse>();

        let key_clone = key.clone();
        let server_client_address_clone = server_client_address.clone();

        fleet.add_host(reader_name.clone(), |ctx| {
            let client_tx = ctx.new_outbox::<ClientRequest>("client".to_string());
            let client_rx = ctx.new_inbox::<ClientResponse>("client".to_string());

            dfir_syntax! {
                client_output = dest_sink(client_tx);

                source_stream(reader_trigger_rx)
                    -> map(|_| (ClientRequest::Get { key: key_clone.clone() }, server_client_address_clone.clone()) )
                    -> client_output;

                client_input = source_stream(client_rx)
                    -> for_each(|(resp, _addr)| response_tx.send(resp).unwrap());

            }
        });

        reader_trigger_tx.send(()).unwrap();

        loop {
            fleet.run_single_tick_all_hosts().await;

            let responses = dfir_rs::util::collect_ready_async::<Vec<_>, _>(&mut response_rx).await;

            if !responses.is_empty() {
                assert_eq!(
                    responses,
                    &[ClientResponse::Get {
                        key,
                        value: HashSet::from([val_a, val_b])
                    }]
                );
                break;
            }
        }
    }

    #[dfir_rs::test]
    async fn test_gossip() {
        let subscriber = tracing_subscriber::FmtSubscriber::builder()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_test_writer()
            .finish();

        let _ = tracing::subscriber::set_global_default(subscriber);

        let mut fleet = Fleet::new();

        let server_a: Hostname = "server_a".to_string();
        let server_b: Hostname = "server_b".to_string();

        let server_a_client_address = Address::new(server_a.clone(), "client".to_string());
        let server_b_client_address = Address::new(server_b.clone(), "client".to_string());

        let server_a_gossip_address = Address::new(server_a.clone(), "gossip".to_string());
        let server_b_gossip_address = Address::new(server_b.clone(), "gossip".to_string());

        let seed_nodes = vec![
            SeedNode {
                id: server_a.clone(),
                address: server_a_gossip_address.clone(),
            },
            SeedNode {
                id: server_b.clone(),
                address: server_b_gossip_address.clone(),
            },
        ];

        let (gossip_trigger_tx_a, gossip_trigger_rx_a) = dfir_rs::util::unbounded_channel::<()>();

        let seed_nodes_clone = seed_nodes.clone();
        fleet.add_host(server_a.clone(), |ctx| {
            let client_input = ctx.new_inbox::<ClientRequest>("client".to_string());
            let client_output = ctx.new_outbox::<ClientResponse>("client".to_string());

            let gossip_input = ctx.new_inbox::<GossipMessage>("gossip".to_string());
            let gossip_output = ctx.new_outbox::<GossipMessage>("gossip".to_string());

            let member_data = MemberDataBuilder::new(server_a.clone())
                .add_protocol(Protocol::new(
                    "client".into(),
                    server_a_client_address.clone(),
                ))
                .add_protocol(Protocol::new(
                    "gossip".into(),
                    server_a_gossip_address.clone(),
                ))
                .build();

            server(
                client_input,
                client_output,
                gossip_input,
                gossip_output,
                gossip_trigger_rx_a,
                member_data,
                seed_nodes_clone,
                empty(),
            )
        });

        let (_, gossip_trigger_rx_b) = dfir_rs::util::unbounded_channel::<()>();

        let seed_nodes_clone = seed_nodes.clone();
        fleet.add_host(server_b.clone(), |ctx| {
            let client_input = ctx.new_inbox::<ClientRequest>("client".to_string());
            let client_output = ctx.new_outbox::<ClientResponse>("client".to_string());

            let gossip_input = ctx.new_inbox::<GossipMessage>("gossip".to_string());
            let gossip_output = ctx.new_outbox::<GossipMessage>("gossip".to_string());

            let member_data = MemberDataBuilder::new(server_b.clone())
                .add_protocol(Protocol::new(
                    "client".into(),
                    server_b_client_address.clone(),
                ))
                .add_protocol(Protocol::new(
                    "gossip".into(),
                    server_b_gossip_address.clone(),
                ))
                .build();

            server(
                client_input,
                client_output,
                gossip_input,
                gossip_output,
                gossip_trigger_rx_b,
                member_data,
                seed_nodes_clone,
                empty(),
            )
        });

        let key = Key {
            namespace: Namespace::User,
            table: "table".to_string(),
            row_key: "row".to_string(),
        };

        let writer_name: Hostname = "writer".to_string();

        let (writer_trigger_tx, writer_trigger_rx) = dfir_rs::util::unbounded_channel::<String>();

        let key_clone = key.clone();
        let server_a_client_address_clone = server_a_client_address.clone();

        fleet.add_host(writer_name.clone(), |ctx| {
            let client_tx = ctx.new_outbox::<ClientRequest>("client".to_string());
            dfir_syntax! {
                client_output = dest_sink(client_tx);

                source_stream(writer_trigger_rx)
                    -> map(|value| (ClientRequest::Set { key: key_clone.clone(), value: value.clone()}, server_a_client_address_clone.clone()) )
                    -> client_output;
            }
        });

        let reader_name: Hostname = "reader".to_string();

        let (reader_trigger_tx, reader_trigger_rx) = dfir_rs::util::unbounded_channel::<()>();
        let (response_tx, mut response_rx) = dfir_rs::util::unbounded_channel::<ClientResponse>();

        let key_clone = key.clone();
        let server_b_client_address_clone = server_b_client_address.clone();

        fleet.add_host(reader_name.clone(), |ctx| {
            let client_tx = ctx.new_outbox::<ClientRequest>("client".to_string());
            let client_rx = ctx.new_inbox::<ClientResponse>("client".to_string());

            dfir_syntax! {
                client_output = dest_sink(client_tx);

                source_stream(reader_trigger_rx)
                    -> map(|_| (ClientRequest::Get { key: key_clone.clone() }, server_b_client_address_clone.clone()) )
                    -> client_output;

                client_input = source_stream(client_rx)
                    -> for_each(|(resp, _addr)| response_tx.send(resp).unwrap());

            }
        });

        let value = "VALUE".to_string();
        writer_trigger_tx.send(value.clone()).unwrap();

        loop {
            reader_trigger_tx.send(()).unwrap();
            fleet.run_single_tick_all_hosts().await;
            let responses = dfir_rs::util::collect_ready_async::<Vec<_>, _>(&mut response_rx).await;

            if !responses.is_empty() {
                assert_eq!(
                    responses,
                    &[ClientResponse::Get {
                        key,
                        value: HashSet::from([value.clone()])
                    }]
                );
                break;
            }

            gossip_trigger_tx_a.send(()).unwrap();
        }
    }
}
