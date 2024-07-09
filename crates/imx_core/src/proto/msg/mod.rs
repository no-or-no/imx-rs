use crate::proto::MtSer;

pub use encrypted::Encrypted;
pub use unencrypted::Unencrypted;
use anyhow::Result;
use bytes::Bytes;

mod encrypted;
mod unencrypted;

/// [Mobile Transport Protocol](https://core.telegram.org/mtproto/description#schematic-presentation-of-messages)
pub trait MsgWrap {
    /// returns msg_id
    fn wrap(&mut self, data: &[u8]) -> Result<(i64, Bytes)>;

    fn unwrap(&mut self, data: &[u8]) -> Result<(i64, Bytes)>;
}