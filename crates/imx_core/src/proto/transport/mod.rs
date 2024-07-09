use std::cmp::min;

use anyhow::Result;
use cipher::{KeyIvInit, StreamCipher, StreamCipherCoreWrapper};
use rand::{RngCore, thread_rng};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub use abridged::Abridged;
pub use full::Full;
pub use intermediate::Intermediate;
pub use padded_intermediate::PaddedIntermediate;
use crate::proto::ByteBuffer;

mod abridged;
mod intermediate;
mod padded_intermediate;
mod full;

pub(crate) type Aes256Ctr128LE = ctr::Ctr128LE<aes::Aes256>;

/// https://core.telegram.org/mtproto/mtproto-transports
pub trait Transport {
    /// Panics if `input.len()` is not divisible by 4.
    fn pack(&mut self, input: &[u8], output: &mut ByteBuffer);

    /// If ok, returns how many bytes of `input` were used.
    fn unpack(&mut self, input: &[u8], output: &mut ByteBuffer) -> Result<usize>;
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum TransportType {
    Abridged = 0xef,
    Intermediate = 0xee,
    PaddedIntermediate = 0xdd,
    Full,
}

#[derive(Error, Clone, Debug, PartialEq)]
pub enum Error {
    /// Not enough bytes are provided.
    #[error("transport error: need more bytes")]
    MissingBytes,
    /// The length is either too short or too long to represent a valid packet.
    #[error("transport error: bad len (got {got})")]
    BadLen { got: u32 },
    /// The sequence number received does not match the expected value.
    #[error("bad seq (expected {expected}, got {got})")]
    BadSeq { expected: u32, got: u32 },
    /// The checksum of the packet does not match its expected value.
    #[error("bad crc (expected {expected:x}, got {got:x})")]
    BadCrc { expected: u32, got: u32 },
}

/// [Transport obfuscation](https://core.telegram.org/mtproto/mtproto-transports#transport-obfuscation)
///
/// 使用 [WebSocket](https://core.telegram.org/mtproto/transports#websocket) 时需要,
/// Android 客户端源码也使用了
#[derive(Clone)]
pub struct Obfuscation {
    secret: String,
    encrypt_cipher: Option<Aes256Ctr128LE>,
    decrypt_cipher: Option<Aes256Ctr128LE>,
    encrypt_key: [u8; 32],
    encrypt_iv: [u8; 16],
    decrypt_key: [u8; 32],
    decrypt_iv: [u8; 16],
}

impl Obfuscation {
    fn new(secret: String) -> Self {
        Self {
            secret,
            encrypt_cipher: None,
            decrypt_cipher: None,
            encrypt_key: [0; 32],
            encrypt_iv: [0; 16],
            decrypt_key: [0; 32],
            decrypt_iv: [0; 16],
        }
    }

    /// 生成 64 bytes 随机初始化 payload
    fn init_header(&mut self, transport_id: Option<u8>) -> [u8; 64] {
        let mut rng = thread_rng();
        let mut header = [0; 64];
        let mut buf = [0; 4];
        loop {
            rng.fill_bytes(&mut header);

            // 被认为是 abridged 类型
            if header[0] == 0xef { continue; }

            // 被认为是 intermediate/padded intermediate 类型, 或者其它 http methods
            buf.copy_from_slice(&header[..4]);
            let v1 = u32::from_le_bytes(buf);
            if v1 == 0x44414548 || v1 == 0x54534f50 || v1 == 0x20544547 || v1 == 0x4954504f
                || v1 == 0x02010316 || v1 == 0xdddddddd || v1 == 0xeeeeeeee { continue; }

            // 被认为是 full 类型, 且 seqno=0 的情况
            buf.copy_from_slice(&header[4..8]);
            let v2 = u32::from_le_bytes(buf);
            if v2 == 0x00000000 { continue; }

            // 除去以上会与其他类型冲突的情况, 随机数才有效
            // 56-60 填充 transport 协议 id, 没有协议 id 的类型 full 保持随机数
            if let Some(transport_id) = transport_id {
                header[56] = transport_id;
                header[57] = transport_id;
                header[58] = transport_id;
                header[59] = transport_id;
            }

            // 60-62 填充 dc_id (测试服为 dc_id + 10000), media 类型的 dc_id 取负数 (测试服为 -(dc_id + 10000))
            // 使用 proxy 服务器才需要填充 dc_id, 否则保持随机数

            // 剩余位保持随机数
            break;
        }

        let mut temp = [0; 64];
        temp.copy_from_slice(&header);
        self.encrypt_key.copy_from_slice(&temp[8..40]);
        self.encrypt_iv.copy_from_slice(&temp[40..56]);
        temp.reverse();
        self.decrypt_key.copy_from_slice(&temp[8..40]);
        self.decrypt_iv.copy_from_slice(&temp[40..56]);

        encrypt_key_with_secret(&self.secret, &mut self.encrypt_key);
        encrypt_key_with_secret(&self.secret, &mut self.decrypt_key);

        temp.copy_from_slice(&header);
        self.aes256_ctr128_encrypt(&mut temp);

        header[56..].copy_from_slice(&temp[56..]);

        header
    }

    fn aes256_ctr128_encrypt(&mut self, data: &mut [u8]) {
        let cipher = self.encrypt_cipher.get_or_insert_with(|| {
            StreamCipherCoreWrapper::new(&self.encrypt_key.into(), &self.encrypt_iv.into())
        });
        cipher.apply_keystream(data);
    }

    fn aes256_ctr128_decrypt(&mut self, data: &mut [u8]) {
        let cipher = self.decrypt_cipher.get_or_insert_with(|| {
            StreamCipherCoreWrapper::new(&self.decrypt_key.into(), &self.decrypt_iv.into())
        });
        cipher.apply_keystream(data);
    }

}

fn encrypt_key_with_secret(secret: &str, bytes: &mut [u8]) {
    if bytes.len() < 32 { return; }
    if secret.is_empty() { return; }

    let mut chars = secret.chars().collect::<Vec<char>>();

    let mut start = 0;
    let mut end = min(16, chars.len());
    let first_char = chars[0] as u32;
    if chars.len() >= 17 && (first_char == 0xdd || first_char == 0xee) {
        start = 1;
        end = 17;
    }

    let mut hasher: Sha256 = Sha256::new();
    hasher.update(&bytes[..32]);
    for ch in &chars[start..end] {
        hasher.update(ch.to_string().as_bytes());
    }
    let hash = hasher.finalize();

    bytes[..32].copy_from_slice(&hash);
}