use super::my_last_write_wins::MyLastWriteWinsDeserializer;
use crate::buffer_pool::BufferPool;
use crate::protocol::serialization::lattices::my_last_write_wins::MyLastWriteWinsWrapper;
use crate::protocol::MyLastWriteWins;
use lattices::map_union::MapUnionHashMap;
use serde::de::{DeserializeSeed, Visitor};
use serde::{Serialize, Serializer};
use std::cell::RefCell;
use std::rc::Rc;

#[repr(transparent)]
pub struct MapUnionHashMapWrapper<'a, const SIZE: usize>(
    pub &'a MapUnionHashMap<u64, MyLastWriteWins<SIZE>>,
);

impl<'a, const SIZE: usize> Serialize for MapUnionHashMapWrapper<'a, SIZE> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;

        let inner_map = &self.0 .0;

        let mut map_serializer = serializer.serialize_map(Some(inner_map.len()))?;

        for (k, v) in inner_map {
            map_serializer.serialize_entry(k, &MyLastWriteWinsWrapper(v))?;
        }

        map_serializer.end()
    }
}

pub struct MapUnionHashMapDeserializer<const SIZE: usize> {
    pub collector: Rc<RefCell<BufferPool<SIZE>>>,
}
impl<'de, const SIZE: usize> DeserializeSeed<'de> for MapUnionHashMapDeserializer<SIZE> {
    type Value = MapUnionHashMap<u64, MyLastWriteWins<SIZE>>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct V<const SIZE: usize> {
            pub collector: Rc<RefCell<BufferPool<SIZE>>>,
        }
        impl<'de, const SIZE: usize> Visitor<'de> for V<SIZE> {
            type Value = MapUnionHashMap<u64, MyLastWriteWins<SIZE>>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str(std::any::type_name::<Self::Value>())
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut inner_map = MapUnionHashMap::<u64, MyLastWriteWins<SIZE>>::default();

                loop {
                    let k: Option<u64> = map.next_key()?;

                    if let Some(k) = k {
                        inner_map.0.insert(
                            k,
                            map.next_value_seed(MyLastWriteWinsDeserializer {
                                collector: self.collector.clone(),
                            })?,
                        );
                    } else {
                        break;
                    }
                }

                Ok(inner_map)
            }
        }

        deserializer.deserialize_map(V {
            collector: self.collector,
        })
    }
}
