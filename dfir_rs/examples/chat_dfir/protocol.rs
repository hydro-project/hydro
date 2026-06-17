use std::net::SocketAddr;

use chrono::prelude::*;
use dfir_macro::DemuxEnum;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize, Debug, DemuxEnum)]
pub enum Message {
    ConnectRequest,
    ConnectResponse,
    ChatMsg {
        nickname: String,
        message: String,
        ts: DateTime<Utc>,
    },
}

#[derive(Clone, Debug, DemuxEnum)]
pub enum MessageWithAddr {
    ConnectRequest {
        addr: SocketAddr,
    },
    ConnectResponse {
        addr: SocketAddr,
    },
    ChatMsg {
        addr: SocketAddr,
        nickname: String,
        message: String,
        ts: DateTime<Utc>,
    },
}
impl MessageWithAddr {
    pub fn from_message(message: Message, addr: SocketAddr) -> Self {
        match message {
            Message::ConnectRequest => Self::ConnectRequest { addr },
            Message::ConnectResponse => Self::ConnectResponse { addr },
            Message::ChatMsg {
                nickname,
                message,
                ts,
            } => Self::ChatMsg {
                addr,
                nickname,
                message,
                ts,
            },
        }
    }
}
