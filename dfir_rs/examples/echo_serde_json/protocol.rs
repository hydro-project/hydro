use chrono::prelude::*;
use serde::{Deserialize, Serialize};

/// The `Serialize` and `Deserialize` traits allow for serialization by the `serde` crate.
#[derive(PartialEq, Clone, Serialize, Deserialize, Debug)]
pub struct EchoMsg {
    pub payload: String,
    pub ts: DateTime<Utc>,
}
