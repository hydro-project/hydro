use serde::de::{SeqAccess, Visitor};

use crate::protocol::KvsRequest;

pub struct KvsRequestGetVisitor<const SIZE: usize>;
impl<'de, const SIZE: usize> Visitor<'de> for KvsRequestGetVisitor<SIZE> {
    type Value = KvsRequest<SIZE>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("KvsRequest::Get")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let key: u64 = seq.next_element()?.unwrap();

        Ok(KvsRequest::Get { key })
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut key = None;

        loop {
            let k: Option<String> = map.next_key()?;
            if let Some(k) = k {
                match k.as_str() {
                    "key" => {
                        key = Some(map.next_value()?);
                    }
                    _ => panic!(),
                }
            } else {
                break;
            }
        }

        assert!(key.is_some());

        Ok(KvsRequest::Get { key: key.unwrap() })
    }
}
