use crate::buffer_pool::{AutoReturnBufferDeserializer, BufferPool};
use crate::protocol::KvsRequest;
use serde::de::{SeqAccess, Visitor};
use std::cell::RefCell;
use std::rc::Rc;

pub struct KvsRequestPutVisitor<const SIZE: usize> {
    pub collector: Rc<RefCell<BufferPool<SIZE>>>,
}
impl<'de, const SIZE: usize> Visitor<'de> for KvsRequestPutVisitor<SIZE> {
    type Value = KvsRequest<SIZE>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("KvsRequest::Put")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let key: u64 = seq.next_element()?.unwrap();
        let value = seq
            .next_element_seed(AutoReturnBufferDeserializer {
                collector: self.collector,
            })?
            .unwrap();

        Ok(KvsRequest::Put { key, value })
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut key = None;
        let mut buffer = None;

        loop {
            let k: Option<String> = map.next_key()?;
            if let Some(k) = k {
                match k.as_str() {
                    "key" => {
                        key = Some(map.next_value()?);
                    }
                    "value" => {
                        buffer = Some(map.next_value_seed(AutoReturnBufferDeserializer {
                            collector: self.collector.clone(),
                        })?);
                    }
                    _ => panic!(),
                }
            } else {
                break;
            }
        }

        assert!(key.is_some());
        assert!(buffer.is_some());

        Ok(KvsRequest::Put {
            key: key.unwrap(),
            value: buffer.unwrap(),
        })
    }
}
