use std::cell::RefCell;
use std::rc::Rc;

use lattices::{Point, WithBot};
use serde::de::{DeserializeSeed, Visitor};
use serde::{Deserializer, Serialize, Serializer};

use super::point::PointWrapper;
use crate::buffer_pool::{AutoReturnBuffer, BufferPool};
use crate::protocol::serialization::lattices::point::PointDeserializer;

#[repr(transparent)]
pub struct WithBotWrapper<'a, const SIZE: usize>(
    pub &'a WithBot<Point<AutoReturnBuffer<SIZE>, ()>>,
);

impl<const SIZE: usize> Serialize for WithBotWrapper<'_, SIZE> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(inner) = self.0.as_reveal_ref() {
            serializer.serialize_some(&PointWrapper(inner))
        } else {
            serializer.serialize_none()
        }
    }
}

pub struct WithBotDeserializer<const SIZE: usize> {
    pub collector: Rc<RefCell<BufferPool<SIZE>>>,
}
impl<'de, const SIZE: usize> DeserializeSeed<'de> for WithBotDeserializer<SIZE> {
    type Value = WithBot<Point<AutoReturnBuffer<SIZE>, ()>>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct V<const SIZE: usize> {
            pub collector: Rc<RefCell<BufferPool<SIZE>>>,
        }
        impl<'de, const SIZE: usize> Visitor<'de> for V<SIZE> {
            type Value = WithBot<Point<AutoReturnBuffer<SIZE>, ()>>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str(std::any::type_name::<Self::Value>())
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
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

                    fn visit_newtype_struct<D>(
                        self,
                        deserializer: D,
                    ) -> Result<Self::Value, D::Error>
                    where
                        D: Deserializer<'de>,
                    {
                        DeserializeSeed::deserialize(
                            PointDeserializer {
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

                Ok(WithBot::new(Some(inner)))
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(WithBot::new(None))
            }
        }

        deserializer.deserialize_option(V {
            collector: self.collector,
        })
    }
}
