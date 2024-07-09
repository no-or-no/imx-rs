use serde::{Deserialize, Serialize};

use with_crc::WithCrc;
#[allow(unused_imports)]
use crate::proto::*;

// #[cfg_attr(feature = "d", allow(abc))]

#[derive(WithCrc, Default, Serialize, Deserialize, Debug)]
#[crc(0xbe7e8ef1)]
pub struct ReqPQMulti {
    pub nonce: [u8; 16],
}
impl MtRpc for ReqPQMulti { type Return = ResPQ; }

#[derive(WithCrc, Default, Serialize, Deserialize, Debug)]
#[crc(0xd712e4be)]
pub struct ReqDHParams {
    pub nonce: [u8; 16],
    pub server_nonce: [u8; 16],
    pub p: [u8; 4],
    pub q: [u8; 4],
    pub public_key_fingerprint: i64,
    #[serde(with = "serde_bytes")]
    pub encrypted_data: Vec<u8>,
}
impl MtRpc for ReqDHParams { type Return = ServerDHParams; }

#[derive(WithCrc, Default, Serialize, Deserialize, Debug)]
#[crc(0xf5045f1f)]
pub struct SetClientDHParams {
    pub nonce: [u8; 16],
    pub server_nonce: [u8; 16],
    #[serde(with = "serde_bytes")]
    pub encrypted_data: Vec<u8>,
}
impl MtRpc for SetClientDHParams { type Return = SetClientDHParamsAnswer; }

#[derive(WithCrc, Default, Serialize, Deserialize, Debug)]
#[crc(0xf3427b8c)]
pub struct PingDelayDisconnect {
    pub ping_id: i64,
    pub disconnect_delay: i32,
}
impl MtRpc for PingDelayDisconnect { type Return = Pong; }