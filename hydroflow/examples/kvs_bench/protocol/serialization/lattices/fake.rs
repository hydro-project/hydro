use std::cell::RefCell;
use std::rc::Rc;

use lattices::fake::Fake;
use serde::de::{DeserializeSeed, Visitor};
use serde::{Serialize, Serializer};

use crate::buffer_pool::{AutoReturnBuffer, AutoReturnBufferDeserializer, BufferPool};

#[repr(transparent)]
pub struct FakeWrapper<'a, const SIZE: usize>(pub &'a Fake<AutoReturnBuffer<SIZE>>);

impl<'a, const SIZE: usize> Serialize for FakeWrapper<'a, SIZE> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_newtype_struct("Fake", &self.0)
    }
}

pub struct FakeDeserializer<const SIZE: usize> {
    pub collector: Rc<RefCell<BufferPool<SIZE>>>,
}
impl<'de, const SIZE: usize> DeserializeSeed<'de> for FakeDeserializer<SIZE> {
    type Value = Fake<AutoReturnBuffer<SIZE>>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct V<const SIZE: usize> {
            pub collector: Rc<RefCell<BufferPool<SIZE>>>,
        }
        impl<'de, const SIZE: usize> Visitor<'de> for V<SIZE> {
            type Value = Fake<AutoReturnBuffer<SIZE>>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str(std::any::type_name::<Self::Value>())
            }

            fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: serde::Deserializer<'de>,
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
                        D: serde::Deserializer<'de>,
                    {
                        serde::de::DeserializeSeed::deserialize(
                            AutoReturnBufferDeserializer {
                                collector: self.collector,
                            },
                            deserializer,
                        )
                    }
                }

                let inner = deserializer.deserialize_newtype_struct(
                    "Fake",
                    V {
                        collector: self.collector,
                    },
                )?;

                Ok(Fake::<AutoReturnBuffer<SIZE>>(inner))
            }
        }

        deserializer.deserialize_newtype_struct(
            "Fake",
            V {
                collector: self.collector,
            },
        )
    }
}
