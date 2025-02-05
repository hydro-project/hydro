use std::cell::RefCell;
use std::rc::Rc;

use lattices::Point;
use serde::de::{DeserializeSeed, Visitor};
use serde::{Deserializer, Serialize, Serializer};

use crate::buffer_pool::{AutoReturnBuffer, AutoReturnBufferDeserializer, BufferPool};

#[repr(transparent)]
pub struct PointWrapper<'a, const SIZE: usize>(pub &'a Point<AutoReturnBuffer<SIZE>, ()>);

impl<const SIZE: usize> Serialize for PointWrapper<'_, SIZE> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_newtype_struct("Point", &self.0.val)
    }
}

pub struct PointDeserializer<const SIZE: usize> {
    pub collector: Rc<RefCell<BufferPool<SIZE>>>,
}
impl<'de, const SIZE: usize> DeserializeSeed<'de> for PointDeserializer<SIZE> {
    type Value = Point<AutoReturnBuffer<SIZE>, ()>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct V<const SIZE: usize> {
            pub collector: Rc<RefCell<BufferPool<SIZE>>>,
        }
        impl<'de, const SIZE: usize> Visitor<'de> for V<SIZE> {
            type Value = Point<AutoReturnBuffer<SIZE>, ()>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str(std::any::type_name::<Self::Value>())
            }

            fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct V<const SIZE: usize> {
                    pub collector: Rc<RefCell<BufferPool<SIZE>>>,
                }
                impl<'de, const SIZE: usize> Visitor<'de> for V<SIZE> {
                    type Value = AutoReturnBuffer<SIZE>;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str(std::any::type_name::<Self::Value>())
                    }

                    fn visit_newtype_struct<D>(
                        self,
                        deserializer: D,
                    ) -> Result<Self::Value, D::Error>
                    where
                        D: Deserializer<'de>,
                    {
                        DeserializeSeed::deserialize(
                            AutoReturnBufferDeserializer {
                                collector: self.collector,
                            },
                            deserializer,
                        )
                    }
                }

                let inner = deserializer.deserialize_newtype_struct(
                    "Point",
                    V {
                        collector: self.collector,
                    },
                )?;

                Ok(Point::new(inner))
            }
        }

        deserializer.deserialize_newtype_struct(
            "Point",
            V {
                collector: self.collector,
            },
        )
    }
}
