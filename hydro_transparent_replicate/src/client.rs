//! Client interface for sending commands to the replicated service.
//!
//! This module provides [`ReplicatedClient`], an async TCP client that connects
//! to all replicas in the cluster and sends commands using bincode serialization.
//! The client transparently handles:
//!
//! - **Proxying**: Any replica accepts commands and forwards to the primary.
//! - **Failover**: On connection failure, the client retries on the next replica.
//! - **Primary tracking**: The client tracks which replica is likely the primary
//!   for optimization (sends directly to primary when known).
//!
//! # Protocol-Side Client Handling (Task 13.2)
//!
//! The server-side integration (accepting TCP connections on each replica, proxying
//! non-primary requests to the primary, and routing responses back) is handled by
//! the Hydro deployment framework. In a full deployment:
//!
//! 1. Each replica accepts `ClientRequest<C>` from TCP connections.
//! 2. Non-primary replicas forward requests to the current primary (proxy).
//! 3. The primary processes commands through the replication pipeline.
//! 4. `ClientResponse<R>` is routed back to the originating client.
//!
//! This is analogous to how `basic_primary_backup.rs` tests work with `TrybuildHost`
//! — the deployment framework wires TCP sources into the dataflow's `client_commands`
//! stream and routes responses back via TCP sinks.
//!
//! # Example
//!
//! ```rust,ignore
//! use hydro_transparent_replicate::client::ReplicatedClient;
//! use hydro_transparent_replicate::backends::redb::{RedbService, RedbCommand, RedbResponse};
//!
//! #[tokio::main]
//! async fn main() {
//!     let addrs = vec![
//!         "127.0.0.1:9000".parse().unwrap(),
//!         "127.0.0.1:9001".parse().unwrap(),
//!         "127.0.0.1:9002".parse().unwrap(),
//!     ];
//!     let mut client = ReplicatedClient::<RedbService>::new(addrs).await.unwrap();
//!
//!     let resp = client.execute(RedbCommand::Put {
//!         key: b"hello".to_vec(),
//!         value: b"world".to_vec(),
//!     }).await.unwrap();
//!     println!("Response: {:?}", resp);
//! }
//! ```

use std::io;
use std::marker::PhantomData;
use std::net::SocketAddr;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::messages::{ClientRequest, ClientResponse};
use crate::ReplicableService;

/// Errors that can occur during client operations.
#[derive(Debug)]
pub enum ClientError {
    /// All replicas are unreachable.
    AllReplicasDown,
    /// IO error on the underlying TCP connection.
    Io(io::Error),
    /// Serialization/deserialization error.
    Codec(String),
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientError::AllReplicasDown => write!(f, "all replicas are unreachable"),
            ClientError::Io(e) => write!(f, "IO error: {}", e),
            ClientError::Codec(msg) => write!(f, "codec error: {}", msg),
        }
    }
}

impl std::error::Error for ClientError {}

impl From<io::Error> for ClientError {
    fn from(e: io::Error) -> Self {
        ClientError::Io(e)
    }
}

/// A client for sending commands to a transparently replicated service.
///
/// Maintains TCP connections to all replicas in the cluster. Commands are sent
/// as [`ClientRequest`] messages serialized with bincode (length-prefixed), and
/// responses are received as [`ClientResponse`] messages.
///
/// The client tracks which replica is likely the primary for optimization. On
/// connection failure, it rotates to the next replica and retries.
///
/// # Type Parameters
///
/// * `S` - The [`ReplicableService`] implementation. Determines the `Command`
///   and `Response` types used for serialization.
pub struct ReplicatedClient<S: ReplicableService> {
    /// Addresses of all replicas in the cluster.
    addrs: Vec<SocketAddr>,
    /// Active TCP connections (lazily established). `None` if not yet connected
    /// or if the connection was dropped after a failure.
    connections: Vec<Option<TcpStream>>,
    /// Index of the replica we believe is the current primary.
    /// Commands are sent here first for lower latency.
    current_primary_idx: usize,
    /// Monotonically increasing request ID for deduplication.
    next_request_id: u64,
    /// Unique client identifier.
    client_id: u64,
    /// Phantom data for the service type.
    _phantom: PhantomData<S>,
}

impl<S: ReplicableService> ReplicatedClient<S> {
    /// Create a new client connected to the given replica addresses.
    ///
    /// Connections are established lazily on first use. The client assumes
    /// replica 0 is the initial primary (matching the default view where
    /// `members[0]` is the primary).
    ///
    /// # Arguments
    ///
    /// * `addrs` - Socket addresses of all replicas in the cluster.
    ///
    /// # Returns
    ///
    /// A new `ReplicatedClient` ready to send commands.
    pub fn new(addrs: Vec<SocketAddr>) -> Self {
        let num_replicas = addrs.len();
        // Generate a pseudo-unique client ID from the current time.
        let client_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);

        let connections = (0..num_replicas).map(|_| None).collect();

        ReplicatedClient {
            addrs,
            connections,
            current_primary_idx: 0,
            next_request_id: 0,
            client_id,
            _phantom: PhantomData,
        }
    }

    /// Execute a command on the replicated service.
    ///
    /// Sends the command to the current suspected primary. If the connection
    /// fails, rotates to the next replica and retries. Any replica can accept
    /// the command and proxy it to the actual primary.
    ///
    /// # Arguments
    ///
    /// * `command` - The command to execute on the replicated service.
    ///
    /// # Returns
    ///
    /// The response from the service, or an error if all replicas are unreachable.
    pub async fn execute(&mut self, command: S::Command) -> Result<S::Response, ClientError> {
        let request_id = self.next_request_id;
        self.next_request_id += 1;

        let request = ClientRequest {
            client_id: self.client_id,
            request_id,
            command,
        };

        let num_replicas = self.addrs.len();

        // Try each replica starting from the suspected primary.
        for attempt in 0..num_replicas {
            let idx = (self.current_primary_idx + attempt) % num_replicas;

            match self.send_and_receive(idx, &request).await {
                Ok(response) => {
                    // Success — update primary hint to this replica (it either
                    // is the primary or successfully proxied to it).
                    self.current_primary_idx = idx;
                    return Ok(response.response);
                }
                Err(_) => {
                    // Connection failed — drop it and try the next replica.
                    self.connections[idx] = None;
                    continue;
                }
            }
        }

        Err(ClientError::AllReplicasDown)
    }

    /// Send a request to a specific replica and receive the response.
    ///
    /// Establishes a connection if one doesn't exist. Uses length-prefixed
    /// bincode framing: `[4-byte big-endian length][bincode payload]`.
    async fn send_and_receive(
        &mut self,
        idx: usize,
        request: &ClientRequest<S::Command>,
    ) -> Result<ClientResponse<S::Response>, ClientError> {
        // Ensure we have a connection to this replica.
        if self.connections[idx].is_none() {
            let stream = TcpStream::connect(self.addrs[idx]).await?;
            self.connections[idx] = Some(stream);
        }

        let stream = self.connections[idx].as_mut().unwrap();

        // Serialize the request with bincode.
        let payload = bincode::serialize(request)
            .map_err(|e| ClientError::Codec(e.to_string()))?;

        // Write length-prefixed frame: [4-byte big-endian length][payload].
        let len = payload.len() as u32;
        stream.write_all(&len.to_be_bytes()).await?;
        stream.write_all(&payload).await?;
        stream.flush().await?;

        // Read the response: [4-byte big-endian length][payload].
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).await?;
        let resp_len = u32::from_be_bytes(len_buf) as usize;

        let mut resp_buf = vec![0u8; resp_len];
        stream.read_exact(&mut resp_buf).await?;

        // Deserialize the response.
        let response: ClientResponse<S::Response> = bincode::deserialize(&resp_buf)
            .map_err(|e| ClientError::Codec(e.to_string()))?;

        Ok(response)
    }

    /// Update the suspected primary index.
    ///
    /// Call this if you have external knowledge about which replica is the
    /// current primary (e.g., from a view change notification).
    pub fn set_primary_hint(&mut self, idx: usize) {
        if idx < self.addrs.len() {
            self.current_primary_idx = idx;
        }
    }

    /// Returns the number of replicas this client is configured to connect to.
    pub fn num_replicas(&self) -> usize {
        self.addrs.len()
    }

    /// Returns the current suspected primary index.
    pub fn current_primary(&self) -> usize {
        self.current_primary_idx
    }
}
