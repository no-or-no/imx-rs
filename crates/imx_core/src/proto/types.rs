use bytes::Bytes;
use serde::{Deserialize, Serialize};
use with_crc::WithCrc;

#[derive(WithCrc, Default, Serialize, Deserialize, Debug, Clone)]
#[crc(0x5bb8e511)]
pub struct Message {
    pub msg_id: i64,
    pub seqno: i32,
    pub bytes: i32,
    pub body: Bytes,
    pub outgoing_body: Option<Bytes>,
    pub unparsed_body: Bytes,
}

#[derive(WithCrc, Default, Serialize, Deserialize, Debug)]
#[crc(0x73f1f8dc)]
pub struct MsgContainer {
    pub messages: Vec<Message>,
}

#[derive(WithCrc, Serialize, Deserialize, Debug)]
#[crc(0x05162463)]
pub struct ResPQ {
    pub nonce: [u8; 16],
    pub server_nonce: [u8; 16],
    #[serde(with = "serde_bytes")]
    pub pq: Vec<u8>,
    pub server_public_key_fingerprints: Vec<i64>,
}

#[derive(WithCrc, Serialize, Deserialize, Debug)]
pub enum PQInnerData {
    #[crc(0xa9f55f95)]
    Dc {
        #[serde(with = "serde_bytes")]
        pq: Vec<u8>,
        p: [u8; 4],
        q: [u8; 4],
        nonce: [u8; 16],
        server_nonce: [u8; 16],
        new_nonce: [u8; 32],
        dc: i32,
    },
    #[crc(0x56fddf88)]
    TempDc {
        #[serde(with = "serde_bytes")]
        pq: Vec<u8>,
        p: [u8; 4],
        q: [u8; 4],
        nonce: [u8; 16],
        server_nonce: [u8; 16],
        new_nonce: [u8; 32],
        dc: i32,
        expires_in: i32,
    },
}

#[derive(WithCrc, Serialize, Deserialize, Debug)]
pub enum ServerDHParams {
    #[crc(0x79cb045d)]
    Fail {
        nonce: [u8; 16],
        server_nonce: [u8; 16],
        new_nonce_hash: [u8; 16],
    },
    #[crc(0xd0e8075c)]
    Ok {
        nonce: [u8; 16],
        server_nonce: [u8; 16],
        encrypted_answer: Vec<u8>,
    },
}

#[derive(WithCrc, Serialize, Deserialize, Debug)]
pub enum SetClientDHParamsAnswer {
    #[crc(0x46dc1fb9)]
    Retry {
        nonce: [u8; 16],
        server_nonce: [u8; 16],
        new_nonce_hash2: [u8; 16],
    },
    #[crc(0xa69dae02)]
    Fail {
        nonce: [u8; 16],
        server_nonce: [u8; 16],
        new_nonce_hash3: [u8; 16],
    },
    #[crc(0x3bcbf734)]
    Ok {
        nonce: [u8; 16],
        server_nonce: [u8; 16],
        new_nonce_hash1: [u8; 16],
    },
}

#[derive(WithCrc, Serialize, Deserialize, Debug)]
#[crc(0x347773c5)]
pub struct Pong {
    pub msg_id: i64,
    pub ping_id: i64,
}
