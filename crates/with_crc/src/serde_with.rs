use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::WithCrc;

pub fn serialize<T, S>(value: &T, s: S) -> Result<S::Ok, S::Error>
    where
        T: Serialize + WithCrc,
        S: Serializer {
    SerWithCrc{ crc: value.crc(), inner: value }.serialize(s)
}

pub fn deserialize<'de, T, D>(_d: D) -> Result<T, D::Error>
    where
        T: Deserialize<'de> + WithCrc,
        D: Deserializer<'de> {
    todo!()
}

#[derive(Serialize)]
struct SerWithCrc<'a, T: Serialize + WithCrc>{
    crc: u32,
    #[serde(flatten)]
    inner: &'a T,
}