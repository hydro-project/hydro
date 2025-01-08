use std::net::SocketAddr;

use dfir_macro::DemuxEnum;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, DemuxEnum)]
pub enum KvsMessage {
    Put { key: String, value: String },
    Get { key: String },
}

#[derive(Clone, Debug, DemuxEnum)]
pub enum KvsMessageWithAddr {
    Put {
        key: String,
        value: String,
        addr: SocketAddr,
    },
    Get {
        key: String,
        addr: SocketAddr,
    },
}
impl KvsMessageWithAddr {
    pub fn from_message(message: KvsMessage, addr: SocketAddr) -> Self {
        match message {
            KvsMessage::Put { key, value } => Self::Put { key, value, addr },
            KvsMessage::Get { key } => Self::Get { key, addr },
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KvsResponse {
    pub key: String,
    pub value: String,
}
