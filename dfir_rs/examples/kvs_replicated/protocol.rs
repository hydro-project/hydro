use std::net::SocketAddr;

use dfir_macro::DemuxEnum;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, DemuxEnum)]
pub enum KvsMessage {
    ServerResponse { key: String, value: String },
    ClientPut { key: String, value: String },
    ClientGet { key: String },
    PeerGossip { key: String, value: String },
    PeerJoin,
}

#[derive(Clone, Debug, DemuxEnum)]
pub enum KvsMessageWithAddr {
    ServerResponse {
        key: String,
        value: String,
        addr: SocketAddr,
    },
    ClientPut {
        key: String,
        value: String,
        addr: SocketAddr,
    },
    ClientGet {
        key: String,
        addr: SocketAddr,
    },
    PeerGossip {
        key: String,
        value: String,
        addr: SocketAddr,
    },
    PeerJoin {
        addr: SocketAddr,
    },
}
impl KvsMessageWithAddr {
    pub fn from_message(message: KvsMessage, addr: SocketAddr) -> Self {
        match message {
            KvsMessage::ServerResponse { key, value } => Self::ServerResponse { key, value, addr },
            KvsMessage::ClientPut { key, value } => Self::ClientPut { key, value, addr },
            KvsMessage::ClientGet { key } => Self::ClientGet { key, addr },
            KvsMessage::PeerGossip { key, value } => Self::PeerGossip { key, value, addr },
            KvsMessage::PeerJoin => Self::PeerJoin { addr },
        }
    }
}
