mod serialization;

#[cfg(test)]
mod test;

use lattices::bottom::Bottom;
use lattices::dom_pair::DomPair;
use lattices::fake::Fake;
use lattices::map_union::MapUnionHashMap;
use lattices::ord::Max;
use lattices::set_union::SetUnionHashSet;
pub use serialization::KvsRequestDeserializer;

use crate::buffer_pool::AutoReturnBuffer;

pub type NodeId = usize;

pub type MyLastWriteWins<const SIZE: usize> =
    DomPair<Max<u128>, Bottom<Fake<AutoReturnBuffer<SIZE>>>>;
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
    GetResponse {
        key: u64,
        reg: MyLastWriteWins<SIZE>,
    },
}
