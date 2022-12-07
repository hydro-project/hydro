use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(PartialEq, Clone, Serialize, Deserialize, Debug)]
pub struct EchoMsg {
    pub nonce: String,
    pub payload: String,
    pub addr: SocketAddr,
}
#[derive(PartialEq, Clone, Serialize, Deserialize, Debug)]
pub struct EchoResponse {
    pub nonce: String,
    pub payload: String,
    pub ts: DateTime<Utc>,
}
