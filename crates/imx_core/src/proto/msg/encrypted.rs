use anyhow::Result;
use bytes::Bytes;

use crate::net::{AuthKey, Session};
use crate::proto::{ByteBuffer, MtSer};
use crate::proto::msg::MsgWrap;

/// 加密消息
pub struct Encrypted {
    pub(crate) session: Session,
    pub(crate) auth_key: AuthKey,
}

impl Encrypted {
    pub fn new(session: Session, auth_key: AuthKey) -> Self {
        Self { session, auth_key }
    }
}


impl MsgWrap for Encrypted {
    fn wrap(&mut self, data: &[u8]) -> Result<(i64, Bytes)> {
        let msg_id = self.session.new_msg_id();
        let data_len = data.len();
        // todo

        let mut buf = ByteBuffer::with_capacity(data_len);
        // todo

        Ok((msg_id, buf.to_bytes()))
    }

    fn unwrap(&mut self, data: &[u8]) -> Result<(i64, Bytes)> {
        todo!()
    }
}