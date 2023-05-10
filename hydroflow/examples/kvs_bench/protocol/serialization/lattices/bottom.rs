use super::fake::FakeWrapper;
use crate::{
    buffer_pool::{AutoReturnBuffer, BufferPool},
    protocol::serialization::lattices::fake::FakeDeserializer,
};
use lattices::{bottom::Bottom, fake::Fake};
use serde::{
    de::{DeserializeSeed, Visitor},
    Serialize, Serializer,
};
use std::{cell::RefCell, rc::Rc};

#[repr(transparent)]
pub struct BottomWrapper<'a, const SIZE: usize>(pub &'a Bottom<Fake<AutoReturnBuffer<SIZE>>>);

impl<'a, const SIZE: usize> Serialize for BottomWrapper<'a, SIZE> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(inner) = &self.0 .0 {
            serializer.serialize_some(&FakeWrapper(inner))
        } else {
            serializer.serialize_none()
        }
    }
}

pub struct BottomDeserializer<const SIZE: usize> {
    pub collector: Rc<RefCell<BufferPool<SIZE>>>,
}
impl<'de, const SIZE: usize> DeserializeSeed<'de> for BottomDeserializer<SIZE> {
    type Value = Bottom<Fake<AutoReturnBuffer<SIZE>>>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct V<const SIZE: usize> {
            pub collector: Rc<RefCell<BufferPool<SIZE>>>,
        }
        impl<'de, const SIZE: usize> Visitor<'de> for V<SIZE> {
            type Value = Bottom<Fake<AutoReturnBuffer<SIZE>>>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str(std::any::type_name::<Self::Value>())
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
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

                    fn visit_newtype_struct<D>(
                        self,
                        deserializer: D,
                    ) -> Result<Self::Value, D::Error>
                    where
                        D: serde::Deserializer<'de>,
                    {
                        serde::de::DeserializeSeed::deserialize(
                            FakeDeserializer {
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

                Ok(Bottom::<Fake<AutoReturnBuffer<SIZE>>>(Some(inner)))
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Bottom::<Fake<AutoReturnBuffer<SIZE>>>(None))
            }
        }

        deserializer.deserialize_option(V {
            collector: self.collector,
        })
    }
}
