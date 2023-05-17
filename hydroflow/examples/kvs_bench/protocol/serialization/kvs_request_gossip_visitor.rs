use super::lattices::MapUnionHashMapDeserializer;
use crate::buffer_pool::BufferPool;
use crate::protocol::KvsRequest;
use serde::de::{SeqAccess, Visitor};
use std::cell::RefCell;
use std::rc::Rc;

pub struct KvsRequestGossipVisitor<const SIZE: usize> {
    pub collector: Rc<RefCell<BufferPool<SIZE>>>,
}
impl<'de, const SIZE: usize> Visitor<'de> for KvsRequestGossipVisitor<SIZE> {
    type Value = KvsRequest<SIZE>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("KvsRequest::Gossip")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let map = seq
            .next_element_seed(MapUnionHashMapDeserializer {
                collector: self.collector,
            })?
            .unwrap();

        Ok(KvsRequest::Gossip { map })
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let k: Option<String> = map.next_key()?;
        assert_eq!(k.unwrap(), "map");

        Ok(KvsRequest::Gossip {
            map: map.next_value_seed(MapUnionHashMapDeserializer {
                collector: self.collector,
            })?,
        })
    }
}
