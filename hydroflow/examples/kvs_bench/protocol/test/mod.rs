mod magic_buffer;
mod util;

use std::rc::Rc;

use bincode::options;
use lattices::bottom::Bottom;
use lattices::fake::Fake;
use lattices::map_union::MapUnionHashMap;
use lattices::ord::Max;

use self::util::check_all;
use crate::protocol::{KvsRequestDeserializer, MyLastWriteWins};

type BufferPool = crate::buffer_pool::BufferPool<16>;
type KvsRequest = super::KvsRequest<16>;

#[test]
fn test_gossip() {
    let buffer_pool = BufferPool::create_buffer_pool();

    let req = {
        let buffer = BufferPool::get_from_buffer_pool(&buffer_pool);
        buffer.borrow_mut()[0] = 117;
        let reg = MyLastWriteWins::new(Max::new(49), Bottom::new(Fake::new(buffer)));
        let mut map = MapUnionHashMap::default();

        map.0.insert(7, reg);

        KvsRequest::Gossip { map }
    };

    check_all(&buffer_pool, &req);
}

#[test]
fn test_delete() {
    let buffer_pool = BufferPool::create_buffer_pool();

    check_all(&buffer_pool, &KvsRequest::Delete { key: 7 });
}

#[test]
fn test_get() {
    let buffer_pool = BufferPool::create_buffer_pool();

    check_all(&buffer_pool, &KvsRequest::Get { key: 7 });
}

#[test]
fn test_put() {
    let buffer_pool = BufferPool::create_buffer_pool();

    let req = {
        let value = BufferPool::get_from_buffer_pool(&buffer_pool);
        value.borrow_mut()[0] = 117;

        KvsRequest::Put { key: 7, value }
    };

    check_all(&buffer_pool, &req);
}

// These are very useful for debugging but don't test anything that isn't being tested above.

#[test]
fn test_json() {
    let buffer_pool = BufferPool::create_buffer_pool();

    let req = {
        let buffer = BufferPool::get_from_buffer_pool(&buffer_pool);
        buffer.borrow_mut()[0] = 117;
        let reg = MyLastWriteWins::new(Max::new(49), Bottom::new(Fake::new(buffer)));
        let mut map = MapUnionHashMap::default();

        map.0.insert(7, reg);

        KvsRequest::Gossip { map }
    };

    {
        let mut serialized = Vec::new();
        let mut serializer = serde_json::Serializer::new(std::io::Cursor::new(&mut serialized));

        serde::ser::Serialize::serialize(&req, &mut serializer).unwrap();

        println!("serialized: {}", std::str::from_utf8(&serialized).unwrap());
        println!(
            "serialized: {}",
            serde_json::to_string(&MapUnionHashMap::new_from([(
                7,
                MyLastWriteWins::new(
                    Max::new(49),
                    Bottom::new(Fake::new(BufferPool::get_from_buffer_pool(&buffer_pool)))
                )
            )]))
            .unwrap()
        );

        let mut deserializer =
            serde_json::Deserializer::from_reader(std::io::Cursor::new(&serialized));

        let req2 = serde::de::DeserializeSeed::deserialize(
            KvsRequestDeserializer {
                collector: Rc::clone(&buffer_pool),
            },
            &mut deserializer,
        )
        .unwrap();

        println!("deserialized: {req2:?}",);
    }
}

#[test]
fn test_bincode() {
    let buffer_pool = BufferPool::create_buffer_pool();

    let req = {
        let buffer = BufferPool::get_from_buffer_pool(&buffer_pool);
        buffer.borrow_mut()[0] = 117;
        let reg = MyLastWriteWins::new(Max::new(49), Bottom::new(Fake::new(buffer)));
        let mut map = MapUnionHashMap::default();

        map.0.insert(7, reg);

        KvsRequest::Gossip { map }
    };

    {
        let mut serialized = Vec::new();
        let mut serializer =
            bincode::Serializer::new(std::io::Cursor::new(&mut serialized), options());

        serde::ser::Serialize::serialize(&req, &mut serializer).unwrap();

        println!("serialized: {serialized:?}");

        let mut deserializer =
            bincode::Deserializer::with_reader(std::io::Cursor::new(&serialized), options());

        let req2 = serde::de::DeserializeSeed::deserialize(
            KvsRequestDeserializer {
                collector: Rc::clone(&buffer_pool),
            },
            &mut deserializer,
        )
        .unwrap();

        println!("deserialized: {req2:?}",);
    }
}
