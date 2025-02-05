use chrono::prelude::*;
use serde::{Deserialize, Serialize};

/// Contains all the messages that can be exchanged between application instances. The `Serialize`
/// and `Deserialize` traits allow for serialization by the `serde` crate.
#[derive(PartialEq, Clone, Serialize, Deserialize, Debug)]
pub enum Message {
    /// Echo message contains a string payload, and a timestamp at which the message was
    /// constructed.
    Echo {
        payload: String,
        ts: DateTime<Utc>,
    },

    /// Heartbeat messages carry no information other than their type.
    Heartbeat,
    HeartbeatAck,
}
