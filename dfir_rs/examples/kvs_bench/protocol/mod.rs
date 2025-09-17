mod serialization;

#[cfg(test)]
mod test;

use dfir_rs::DemuxEnum;
use lattices::map_union::MapUnionHashMap;
use lattices::set_union::SetUnionHashSet;
use lattices::{DomPair, Max, Point, WithBot};
pub use serialization::KvsRequestDeserializer;

use crate::buffer_pool::AutoReturnBuffer;

pub type NodeId = usize;

pub type MyLastWriteWins<const SIZE: usize> =
    DomPair<Max<u128>, WithBot<Point<AutoReturnBuffer<SIZE>, ()>>>;
pub type MySetUnion = SetUnionHashSet<(NodeId, usize)>;

#[derive(Clone, Debug)]
pub enum KvsRequest<const SIZE: usize> {
    Put {
        key: u64,
        value: AutoReturnBuffer<SIZE>,
    },
    Get {
        key: u64,
    },
    Gossip {
        map: MapUnionHashMap<u64, MyLastWriteWins<SIZE>>,
    },
    Delete {
        key: u64,
    },
}

#[derive(Clone, Debug)]
pub enum KvsResponse<const SIZE: usize> {
    _PutResponse {
        key: u64,
    },
    #[expect(dead_code, reason = "benchmark discards responses")]
    GetResponse {
        key: u64,
        reg: MyLastWriteWins<SIZE>,
    },
}

#[derive(DemuxEnum)]
pub enum KvsRequestDemux<const SIZE: usize> {
    Get {
        key: u64,
        addr: NodeId,
    },
    /// Put or Delete.
    Update {
        key: u64,
        update: MyLastWriteWins<SIZE>,
    },
    Gossip {
        map: MapUnionHashMap<u64, MyLastWriteWins<SIZE>>
    },
}