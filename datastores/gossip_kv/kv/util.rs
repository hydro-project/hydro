use dfir_rs::DemuxEnum;

use crate::model::{Clock, Namespaces};
use crate::{ClientRequest, GossipMessage, Key};

/// Convenience enum to represent a client request with the address of the client. Makes it
/// possible to use `demux_enum` in the surface syntax.
#[derive(Debug, DemuxEnum)]
pub enum ClientRequestWithAddress<A> {
    /// A get request with the key and the address of the client.
    Get { key: Key, addr: A },
    /// A set request with the key, value and the address of the client.
    Set { key: Key, value: String, addr: A },
    /// A delete request with the key and the address of the client.
    Delete { key: Key, addr: A },
}

impl<A> ClientRequestWithAddress<A> {
    /// Create a `ClientRequestWithAddress` from a `ClientRequest` and an address.
    pub fn from_request_and_address(request: ClientRequest, addr: A) -> Self {
        match request {
            ClientRequest::Get { key } => Self::Get { key, addr },
            ClientRequest::Set { key, value } => Self::Set { key, value, addr },
            ClientRequest::Delete { key } => Self::Delete { key, addr },
        }
    }
}

/// Convenience enum to represent a gossip request with the address of the client. Makes it
/// possible to use `demux_enum` in the surface syntax.
#[derive(Debug, DemuxEnum)]
pub enum GossipRequestWithAddress<A> {
    /// A gossip request with the message id, writes and the address of the client.
    Gossip {
        message_id: String,
        member_id: String,
        writes: Namespaces<Clock>,
        addr: A,
    },
    /// An ack request with the message id and the address of the client.
    Ack {
        message_id: String,
        member_id: String,
        addr: A,
    },
    /// A nack request with the message id and the address of the client.
    Nack {
        message_id: String,
        member_id: String,
        addr: A,
    },
}

impl<A> GossipRequestWithAddress<A> {
    /// Create a `GossipRequestWithAddress` from a `GossipMessage` and an address.
    pub fn from_request_and_address(request: GossipMessage, addr: A) -> Self {
        match request {
            GossipMessage::Gossip {
                message_id,
                member_id,
                writes,
            } => Self::Gossip {
                message_id,
                member_id,
                writes,
                addr,
            },

            GossipMessage::Ack {
                message_id,
                member_id,
            } => Self::Ack {
                message_id,
                addr,
                member_id,
            },
            GossipMessage::Nack {
                message_id,
                member_id,
            } => Self::Nack {
                message_id,
                addr,
                member_id,
            },
        }
    }
}
