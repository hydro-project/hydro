use std::net::SocketAddr;

use chrono::prelude::*;
use dfir_rs::DemuxEnum;
use serde::{Deserialize, Serialize};

/// Contains all the messages that can be exchanged between application instances. The `Serialize`
/// and `Deserialize` traits allow for serialization by the `serde` crate.
#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    /// Echo message contains a string payload, and a timestamp at which the message was
    /// constructed.
    Echo { payload: String, ts: DateTime<Utc> },

    /// Heartbeat messages carry no information other than their type.
    Heartbeat,
}

/// This type derives `DemuxEnum` so it can be used with the `demux_enum()` operator.
#[derive(Serialize, Deserialize, Debug, DemuxEnum)]
pub enum MessageWithAddr {
    Echo {
        addr: SocketAddr,
        payload: String,
        ts: DateTime<Utc>,
    },
    Heartbeat {
        addr: SocketAddr,
    },
}
impl From<(Message, SocketAddr)> for MessageWithAddr {
    fn from((msg, addr): (Message, SocketAddr)) -> Self {
        match msg {
            Message::Echo { payload, ts } => MessageWithAddr::Echo { addr, payload, ts },
            Message::Heartbeat => MessageWithAddr::Heartbeat { addr },
        }
    }
}
