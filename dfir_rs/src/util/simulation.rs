//! # Hydro/DFIR Deterministic Simulation Testing Framework
//!
//! This module provides a deterministic simulation testing framework for testing Hydro/DFIR
//! processes.
//!
//! It can be used to test complex interactions between multiple Hydro/DFIR processes in a
//! deterministic manner by running them in a single-threaded environment. The framework also
//! provides a "virtual network" implementation that allows production processes to exchange
//! messages within the simulation. More importantly, the network is fully under control of the
//! unit test and the test can introduce faults such as message delays, message drops and
//! network partitions.
//!
//! ## Overview
//!
//! Conceptually, the simulation contains a "Fleet", which is a collection of "Hosts". These
//! aren't real hosts, but rather a collection of individual Hydro/DFIR processes (one per host)
//! that can communicate with each other over a virtual network. Every host has a "hostname"
//! which uniquely identifies it within the fleet.
//!
//! ```text
//!  ┌───────────────────────────────────────────────────────────────────────────────────────────┐
//!  │SIMULATION                                                                                 │
//!  │ ┌───────────────────────────────────────────────────────────────────────────────────────┐ │
//!  │ │FLEET                                                                                  │ │
//!  │ │ ┌───────────────────────────────┐                   ┌───────────────────────────────┐ │ │
//!  │ │ │HOST                           │                   │HOST                           │ │ │
//!  │ │ │ ┌──────┐   ┌──────┐  ┌──────┐ │                   │ ┌──────┐   ┌──────┐  ┌──────┐ │ │ │
//!  │ │ │ │INBOX │   │INBOX │  │INBOX │ │                 ┌-┼-►INBOX │   │INBOX │  │INBOX │ │ │ │
//!  │ │ │ └──┬───┘   └──┬───┘  └──┬───┘ │                 │ │ └──┬───┘   └──┬───┘  └──┬───┘ │ │ │
//!  │ │ │ ┌──▼──────────▼─────────▼───┐ │                 │ │ ┌──▼──────────▼─────────▼───┐ │ │ │
//!  │ │ │ │                           │ │                 │ │ │                           │ │ │ │
//!  │ │ │ │        TRANSDUCER         │ │                 │ │ │        TRANSDUCER         │ │ │ │
//!  │ │ │ │                           │ │                 │ │ │                           │ │ │ │
//!  │ │ │ └───┬─────────┬──────────┬──┘ │                 │ │ └───┬─────────┬─────────┬───┘ │ │ │
//!  │ │ │  ┌──▼───┐  ┌──▼───┐  ┌───▼──┐ │                 │ │  ┌──▼───┐  ┌──▼───┐  ┌──▼───┐ │ │ │
//!  │ │ │  │OUTBOX│  │OUTBOX│  │OUTBOX┼-┼--┐              │ │  │OUTBOX│  │OUTBOX│  │OUTBOX│ │ │ │
//!  │ │ │  └──────┘  └──────┘  └──────┘ │  │              │ │  └──────┘  └──────┘  └──────┘ │ │ │
//!  │ │ └───────────────────────────────┘  │              │ └───────────────────────────────┘ │ │
//!  │ └────────────────────────────────────┼──────────────┼───────────────────────────────────┘ │
//!  │                                    ┌─┼──────────────┼─┐                                   │
//!  │                                    │ └--------------┘ │                                   │
//!  │                                    │ NETWORK MESSAGE  │                                   │
//!  │                                    │    PROCESSING    │                                   │
//!  │                                    └──────────────────┘                                   │
//!  └───────────────────────────────────────────────────────────────────────────────────────────┘
//! ```
//! ## Network Processing
//!
//! ### Outboxes & Inboxes
//! When a process wishes to send a message to another process, it sends the message to an
//! "outbox" on its host. The unit test invokes the simulation's network message processing logic
//! at some desired cadence to pick up all messages from all outboxes and deliver them to the
//! corresponding inboxes on the destination hosts. The network message processing logic is the
//! point at which failures can be injected to change the behavior of the network.
//!
//! ### Interface Names
//! Every inbox and outbox is associated with an "interface name". This is a string that uniquely
//! identifies the interface on the host. When a process sends a message, it specifies the
//! destination hostname and the interface name on that host to which the message should be
//! delivered.
//!
//! ## Progress of Time in the Simulation
//! The single-threaded unit test can drive time forward on every host by invoking the `run_tick`
//! method on the host. This ultimately runs a single tick on the process. The unit test is
//! also responsible for invoking the network message processing at the time of its choosing and
//! can interleave the progress of time on various hosts and network processing as it sees fit.
//!
//! ## Examples
//! Check the tests module for examples on how to use the simulation framework.
use std::any::Any;
use std::collections::HashMap;
use std::fmt::Debug;
use std::pin::Pin;

use futures::{Sink, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::Stream;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::trace;

use crate::scheduled::graph::Dfir;
use crate::util::{collect_ready_async, unbounded_channel};

/// A hostname is a unique identifier for a host in the simulation. It is used to address messages
/// to a specific host (and thus a specific Hydro/DFIR process).
pub type Hostname = String;

/// An interface name is a unique identifier for an inbox or an outbox on host.
type InterfaceName = String;

/// An address is a combination of a hostname and an interface name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Address {
    host: Hostname,
    interface: InterfaceName,
}

impl Address {
    /// Create a new address with the given hostname and interface name.
    pub fn new(host: Hostname, interface: InterfaceName) -> Self {
        Address { host, interface }
    }
}

/// A message sender is used to send messages to an inbox on a host.
pub trait MessageSender {
    /// Send a message to the inbox on the host.
    fn send(&self, message: MessageWithAddress);
}

impl<T: 'static> MessageSender for UnboundedSender<(T, Address)> {
    fn send(&self, message: (Box<dyn Any>, Address)) {
        match message.0.downcast::<T>() {
            Ok(msg) => {
                self.send((*msg, message.1)).unwrap();
            }
            Err(e) => {
                panic!("Failed to downcast message to expected type: {:?}", e);
            }
        }
    }
}

/// A message with an delivery address.
pub type MessageWithAddress = (Box<dyn Any>, Address);

/// An inbox is used by a host to receive messages for the process.
pub struct Inbox {
    sender: Box<dyn MessageSender>,
}

/// Processes can send messages to other processes by putting those messages in an outbox
/// on their host.
pub struct Outbox {
    receiver: Pin<Box<dyn Stream<Item = MessageWithAddress>>>,
}

/// A host is a single Hydro/DFIR process running in the simulation. It has a unique hostname
/// and can communicate with other hosts over the virtual network. It has a collection of inboxes
/// and outboxes.
pub struct Host {
    name: Hostname,
    process: Dfir<'static>,
    inputs: HashMap<InterfaceName, Inbox>,
    output: HashMap<InterfaceName, Outbox>,
}

impl Host {
    /// Run a single tick on the host's process. Returns true if any work was done by the
    /// process. This effectively "advances" time on the process.
    pub async fn run_tick(&mut self) -> bool {
        self.process.run_tick().await
    }
}

/// A builder for constructing a host in the simulation.
pub struct HostBuilder {
    name: Hostname,
    process: Option<Dfir<'static>>,
    inboxes: HashMap<InterfaceName, Inbox>,
    outboxes: HashMap<InterfaceName, Outbox>,
}

/// Used in conjunction with the `HostBuilder` to construct a host in the simulation.
pub struct ProcessBuilderContext<'context> {
    inboxes: &'context mut HashMap<InterfaceName, Inbox>,
    outboxes: &'context mut HashMap<InterfaceName, Outbox>,
}

fn sink_from_fn<T>(f: impl FnMut(T)) -> impl Sink<T, Error = crate::Never> {
    sinktools::for_each(f)
}

impl ProcessBuilderContext<'_> {
    /// Create a new inbox on the host with the given interface name. Returns a stream that can
    /// be read by the process using the source_stream dfir operator.
    pub fn new_inbox<T: 'static>(
        &mut self,
        interface: InterfaceName,
    ) -> UnboundedReceiverStream<(T, Address)> {
        let (sender, receiver) = unbounded_channel::<(T, Address)>();
        self.inboxes.insert(
            interface,
            Inbox {
                sender: Box::new(sender),
            },
        );
        receiver
    }

    /// Creates a new outbox on the host with the given interface name. Returns a sink that can
    /// be written to by the process using the dest_sink dfir operator.
    pub fn new_outbox<T: 'static>(
        &mut self,
        interface: InterfaceName,
    ) -> impl use<T> + Sink<(T, Address), Error = crate::Never> {
        let (sender, receiver) = unbounded_channel::<(T, Address)>();

        let receiver = receiver.map(|(msg, addr)| (Box::new(msg) as Box<dyn Any>, addr));

        self.outboxes.insert(
            interface,
            Outbox {
                receiver: Box::pin(receiver),
            },
        );

        sink_from_fn(move |message: (T, Address)| sender.send((message.0, message.1)).unwrap())
    }
}

impl HostBuilder {
    /// Creates a new instance of HostBuilder for a given hostname,
    pub fn new(name: Hostname) -> Self {
        HostBuilder {
            name,
            process: None,
            inboxes: Default::default(),
            outboxes: Default::default(),
        }
    }

    /// Supplies the (mandatory) process that runs on this host.
    pub fn with_process<F>(mut self, builder: F) -> Self
    where
        F: FnOnce(&mut ProcessBuilderContext) -> Dfir<'static>,
    {
        let mut context = ProcessBuilderContext {
            inboxes: &mut self.inboxes,
            outboxes: &mut self.outboxes,
        };
        let process = builder(&mut context);
        self.process = Some(process);
        self
    }

    /// Builds the host with the supplied configuration.
    pub fn build(self) -> Host {
        if self.process.is_none() {
            panic!("Process is required to build a host");
        }

        Host {
            name: self.name,
            process: self.process.unwrap(),
            inputs: self.inboxes,
            output: self.outboxes,
        }
    }
}

/// A fleet is a collection of hosts in the simulation. It is responsible for running the
/// simulation and processing network messages.
pub struct Fleet {
    hosts: HashMap<String, Host>,
}

impl Fleet {
    /// Creates a new instance of Fleet.
    pub fn new() -> Self {
        Fleet {
            hosts: HashMap::new(),
        }
    }

    /// Adds a new host to the fleet with the given name and process.
    pub fn add_host<F>(&mut self, name: String, process_builder: F) -> &Host
    where
        F: FnOnce(&mut ProcessBuilderContext) -> Dfir<'static>,
    {
        let host = HostBuilder::new(name.clone())
            .with_process(process_builder)
            .build();
        assert!(
            self.hosts.insert(host.name.clone(), host).is_none(),
            "Host with name {} already exists",
            name
        );
        self.get_host(&name).unwrap()
    }

    /// Get a host by name.
    pub fn get_host(&self, name: &str) -> Option<&Host> {
        self.hosts.get(name)
    }

    /// Get a host by name.
    pub fn get_host_mut(&mut self, name: &str) -> Option<&mut Host> {
        self.hosts.get_mut(name)
    }

    /// Advance time on all hosts by a single tick. Returns true if any work was done by any of the
    /// hosts. After ticking once on all the hosts, the method also processes network messages.
    ///
    /// The order in which the ticks are processed is not guaranteed.
    pub async fn run_single_tick_all_hosts(&mut self) -> bool {
        let mut work_done: bool = false;

        #[expect(
            clippy::disallowed_methods,
            reason = "nondeterministic iteration order, TODO(mingwei)"
        )]
        for (name, host) in self.hosts.iter_mut() {
            trace!("Running tick for host: {}", name);
            work_done |= host.run_tick().await;
        }

        self.process_network().await;

        work_done
    }

    /// Process all network messages in the simulation. This method picks up all messages from all
    /// outboxes on all hosts and delivers them to the corresponding inboxes on the destination.
    ///
    /// The order in which the messages are processed is not guaranteed.
    pub async fn process_network(&mut self) {
        let mut all_messages: Vec<(Address, MessageWithAddress)> = Vec::new();

        // Collect all messages from all outboxes on all hosts.
        #[expect(
            clippy::disallowed_methods,
            reason = "nondeterministic iteration order, TODO(mingwei)"
        )]
        for (name, host) in self.hosts.iter_mut() {
            for (interface, output) in host.output.iter_mut() {
                let src_address = Address::new(name.clone(), interface.clone());
                let all_messages_on_interface: Vec<_> =
                    collect_ready_async(&mut output.receiver).await;
                for message_on_interface in all_messages_on_interface {
                    all_messages.push((src_address.clone(), message_on_interface));
                }
            }
        }

        // Deliver all messages to the corresponding inboxes on the destination hosts.
        for (src_address, (msg, addr)) in all_messages {
            if let Some(destination_host) = self.hosts.get(&addr.host) {
                if let Some(input) = destination_host.inputs.get(&addr.interface) {
                    input.sender.send((msg, src_address.clone()));
                } else {
                    trace!(
                        "No interface named {:?} found on host {:?}. Dropping message {:?}.",
                        addr.interface, addr.host, msg
                    );
                }
            } else {
                trace!(
                    "No host named {:?} found. Dropping message {:?}.",
                    addr.host, msg
                );
            }
        }
    }

    /// Tick all hosts until all hosts are quiescent (i.e. no new work is done by any host). Ticking
    /// is done in "rounds". At each round, all hosts are ticked once and then network messages are
    /// processed. The process continues until no work is done by any host in a round.
    pub async fn run_until_quiescent(&mut self) {
        while self.run_single_tick_all_hosts().await {}
    }
}

impl Default for Fleet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use dfir_macro::{dfir_syntax, dfir_test};
    use futures::StreamExt;

    use crate::util::simulation::{Address, Fleet, Hostname};
    use crate::util::unbounded_channel;

    /// A simple test to demonstrate use of the simulation framework. Implements an echo server
    /// and client.
    #[dfir_test]
    async fn test_echo() {
        let mut fleet = Fleet::new();

        // Hostnames for the server and client
        let server: Hostname = "server".to_owned();
        let client: Hostname = "client".to_owned();

        // Interface name for the echo "protocol"
        let interface: String = "echo".to_owned();

        let server_address = Address::new(server.clone(), interface.clone());

        // Create the echo server
        fleet.add_host(server.clone(), |ctx| {
            let network_input = ctx.new_inbox::<String>(interface.clone());
            let network_output = ctx.new_outbox::<String>(interface.clone());
            dfir_syntax! {
                out = dest_sink(network_output);

                source_stream(network_input)
                    -> inspect(|(msg, addr)| println!("Received {:?} from {:?}", msg, addr))
                    -> out;
            }
        });

        // The client trigger channel is used to trigger the client into sending a message to the
        // server. This allows the unit test to control when the client sends a message.
        let (client_trigger_tx, client_trigger_rx) = unbounded_channel::<String>();
        let (client_response_tx, mut client_response_rx) = unbounded_channel::<String>();

        fleet.add_host(client.clone(), |ctx| {
            let network_out = ctx.new_outbox::<String>(interface.clone());
            let network_in = ctx.new_inbox::<String>(interface.clone());

            dfir_syntax! {
                out = dest_sink(network_out);

                source_stream(client_trigger_rx)
                    -> map(|msg| (msg, server_address.clone()))
                    -> out;

                source_stream(network_in)
                    -> inspect(|(msg, addr)| println!("Received {:?} from {:?}", msg, addr))
                    -> for_each(|(msg, _addr)| client_response_tx.send(msg).unwrap());

            }
        });

        // Trigger the client to send a message.
        client_trigger_tx.send("Hello, world!".to_owned()).unwrap();

        // Run the simulation until no new work is done by any host.
        fleet.run_until_quiescent().await;

        // Check that the message was received.
        let response = client_response_rx.next().await.unwrap();
        assert_eq!(response, "Hello, world!");
    }
}
