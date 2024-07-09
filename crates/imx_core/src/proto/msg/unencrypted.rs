use anyhow::Result;
use bytes::Bytes;

use crate::net::Session;
use crate::proto::msg::MsgWrap;
use crate::proto::{ByteBuffer, MtSer};

/// 非加密消息
pub struct Unencrypted {
    pub(crate) session: Session,
}

impl Unencrypted {
    pub fn new(session: Session) -> Self {
        Self { session }
    }
}

impl MsgWrap for Unencrypted {
    fn wrap(&mut self, data: &[u8]) -> Result<(i64, Bytes)> {
        let msg_id = self.session.new_msg_id();
        let data_len = data.len();

        // 8 + 8 + 4
        let mut buf = ByteBuffer::with_capacity(20 + data_len);
        // auth_key_id 64-bits
        buf.put_i64(0);
        // message_id 64-bits
        buf.put_i64(msg_id);
        // message_data_length 32-bits
        buf.put_u32(data_len as u32);
        // message_data
        buf.put_all(&data);

        Ok((msg_id, buf.to_bytes()))
    }

    fn unwrap(&mut self, data: &[u8]) -> Result<(i64, Bytes)> {
        todo!()
    }
}