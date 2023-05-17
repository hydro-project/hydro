use std::cell::RefCell;
use std::rc::Rc;

use bincode::options;
use serde::de::DeserializeSeed;
use serde::Serialize;

use super::magic_buffer::MagicBuffer;
use crate::buffer_pool::BufferPool;
use crate::protocol::serialization::KvsRequestDeserializer;
use crate::protocol::KvsRequest;

fn assert_eq_req<const SIZE: usize>(r1: &KvsRequest<SIZE>, r2: &KvsRequest<SIZE>) {
    match (r1, r2) {
        (KvsRequest::Gossip { map: map1 }, KvsRequest::Gossip { map: map2 }) => {
            assert_eq!(map1, map2);
        }
        (KvsRequest::Delete { key: key1 }, KvsRequest::Delete { key: key2 }) => {
            assert_eq!(key1, key2);
        }
        (KvsRequest::Get { key: key1 }, KvsRequest::Get { key: key2 }) => {
            assert_eq!(key1, key2);
        }
        (
            KvsRequest::Put {
                key: key1,
                value: value1,
            },
            KvsRequest::Put {
                key: key2,
                value: value2,
            },
        ) => {
            assert_eq!(key1, key2);
            assert_eq!(*value1.inner.borrow(), *value2.inner.borrow());
        }
        _ => panic!(),
    }
}

fn serialize_deserialize<'de, S, D, const SIZE: usize>(
    req: &KvsRequest<SIZE>,
    buffer_pool: &Rc<RefCell<BufferPool<SIZE>>>,
    mut serializer: S,
    mut deserializer: D,
) -> KvsRequest<SIZE>
where
    for<'a> &'a mut S: serde::Serializer,
    for<'b> &'b mut D: serde::Deserializer<'de>,
{
    Serialize::serialize(&req, &mut serializer).unwrap();

    let req2 = {
        KvsRequestDeserializer {
            collector: Rc::clone(&buffer_pool),
        }
        .deserialize(&mut deserializer)
        .unwrap()
    };

    req2
}

pub fn check_all<const SIZE: usize>(
    buffer_pool: &Rc<RefCell<BufferPool<SIZE>>>,
    req: &KvsRequest<SIZE>,
) {
    {
        let buffer = MagicBuffer::default();

        let req2 = serialize_deserialize(
            &req,
            &buffer_pool,
            bincode::Serializer::new(buffer.clone(), options()),
            bincode::Deserializer::with_reader(buffer.clone(), options()),
        );

        assert_eq_req(&req, &req2);
    }

    {
        let buffer = MagicBuffer::default();

        let req2 = serialize_deserialize(
            &req,
            &buffer_pool,
            serde_json::Serializer::new(buffer.clone()),
            serde_json::Deserializer::from_reader(buffer.clone()),
        );

        assert_eq_req(&req, &req2);
    }
}
