//! A non-`serde` [`SimCodec`] used by the simulator's own tests.
//!
//! It is public because the generated simulation dylib must be able to name it.

use crate::sim::codec::SimCodec;

/// A test message that does not implement `serde` traits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RawMessage {
    pub id: u64,
    pub value: u64,
}

/// Encodes [`RawMessage`] as little-endian bytes.
#[derive(Clone, Copy, Debug, Default)]
pub struct RawMessageCodec;

impl SimCodec<RawMessage> for RawMessageCodec {
    fn encode(value: &RawMessage) -> Vec<u8> {
        let mut out = Vec::with_capacity(16);
        out.extend_from_slice(&value.id.to_le_bytes());
        out.extend_from_slice(&value.value.to_le_bytes());
        out
    }

    fn decode(bytes: &[u8]) -> RawMessage {
        let (id, value) = bytes.split_at(8);
        RawMessage {
            id: u64::from_le_bytes(id.try_into().unwrap()),
            value: u64::from_le_bytes(value.try_into().unwrap()),
        }
    }
}
