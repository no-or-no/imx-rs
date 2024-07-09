use anyhow::Result;
use bytes::Bytes;
use serde::{Deserialize, Serialize};

pub use byte_buffer::ByteBuffer;
pub use funcs::*;
pub use types::*;
use with_crc::WithCrc;

pub mod transport;
pub mod msg;

mod funcs;

mod types;
mod byte_buffer;

pub trait MtSer: Serialize + WithCrc + Sized {
    fn to_bytes(&self) -> Result<Bytes> {
        to_bytes(self)
    }
}

pub trait MtDe: Deserialize<'static> + WithCrc + Sized {
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        todo!()
    }
}

impl<T: Serialize + WithCrc + Sized> MtSer for T {}
impl<T: Deserialize<'static> + WithCrc + Sized> MtDe for T {}

pub trait MtRpc: MtSer {
    type Return: MtDe;
}

/// 序列化
pub fn to_bytes<T: MtSer>(value: &T) -> Result<Bytes> {
    #[cfg(all(feature = "serde_json", not(feature = "serde_mt")))]
    {
        let mut ser = serde_json::Serializer::new(Vec::new());
        with_crc::serialize(value, &mut ser)?;
        let bytes = ser.into_inner();
        // println!("----- json: {}", String::from_utf8(bytes.clone())?);
        return Ok(bytes.into());
    }
    #[cfg(any(feature = "serde_mt", not(feature = "serde_json")))]
    {
        let mut ser = serde_mt::Serializer::new(Vec::new());
        value.crc().serialize(&mut ser)?;
        value.serialize(&mut ser)?;
        let bytes = ser.into_inner();
        // println!("----- mt: {:?}", bytes);
        return Ok(bytes.into());
    }
}

pub fn from_bytes<T: MtDe>(bytes: &[u8]) -> Result<T> {
    // #[cfg(all(feature = "serde_json", not(feature = "serde_mt")))]
    // {
    //     let de = serde_json::Deserializer::new(bytes);
    //     let t: T = with_crc::deserialize(de)?;
    //     let bytes = ser.into_inner();
    //     // println!("----- json: {}", String::from_utf8(bytes.clone())?);
    //     return Ok(bytes.into());
    // }
    // #[cfg(any(feature = "serde_mt", not(feature = "serde_json")))]
    // {
    //     let mut ser = serde_mt::Deserializer::new();
    //     value.crc().serialize(&mut ser)?;
    //     value.serialize(&mut ser)?;
    //     let bytes = ser.into_inner();
    //     // println!("----- mt: {:?}", bytes);
    //     return Ok(bytes.into());
    // }
    todo!()
}
