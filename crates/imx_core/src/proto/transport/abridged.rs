use anyhow::{bail, Result};
use bytes::BufMut;
use log::info;
use crate::proto::ByteBuffer;

use crate::proto::transport::{Error, Obfuscation, Transport};

/// [Abridged](https://core.telegram.org/mtproto/mtproto-transports#abridged)
///
/// The lightest protocol available.
/// - Overhead: Very small
/// - Minimum envelope length: 1 byte
/// - Maximum envelope length: 4 bytes
///
/// Payload structure:
/// ```text
/// +-+----...----+
/// |l|  payload  |
/// +-+----...----+
/// OR
/// +-+---+----...----+
/// |h|len|  payload  +
/// +-+---+----...----+
/// ```
/// Before sending anything into the underlying socket (see [transports](https://core.telegram.org/mtproto/transports)),
/// the client must first send `0xef` as the first byte (the server **will not** send `0xef` as the
/// first byte in the first reply).
///
/// Then, payloads are wrapped in the following envelope:
/// - Length: payload length, divided by four, and encoded as a single byte, only if the resulting
/// packet length is a value between `0x01..0x7e`.
/// - Payload: the MTProto payload
///
/// If the packet length divided by four is bigger than or equal to 127 (>= `0x7f`), the following
/// envelope must be used, instead:
/// - Header: A single byte of value `0x7f`
/// - Length: payload length, divided by four, and encoded as 3 length bytes (little endian)
/// - Payload: the MTProto payload
pub struct Abridged {
    obfuscation: Option<Obfuscation>,
    ack: bool,
    first_packet_sent: bool,
}

impl Abridged {
    pub fn new() -> Self {
        Self {
            obfuscation: None,
            ack: false,
            first_packet_sent: false,
        }
    }

    pub fn ack(self, ack: bool) -> Self {
        Self {
            obfuscation: self.obfuscation,
            ack,
            first_packet_sent: self.first_packet_sent,
        }
    }

    pub fn obfuscation(self, secret: Option<String>) -> Self {
        Self {
            obfuscation: secret.map(|s| Obfuscation::new(s)),
            ack: self.ack,
            first_packet_sent: self.first_packet_sent,
        }
    }
}

impl Transport for Abridged {

    fn pack(&mut self, input: &[u8], output: &mut ByteBuffer) {
        assert_eq!(input.len() % 4, 0);

        if !self.first_packet_sent {
            match &mut self.obfuscation {
                Some(obf) => {
                    let header = obf.init_header(Some(0xef));
                    output.put_all(&header);
                }
                None => {
                    output.put_u8(0xef);
                }
            }
            self.first_packet_sent = true;
        }

        let start = output.len();
        let len = input.len() / 4;
        if len < 0x7f {
            let mut first = len as u8;
            if self.ack {
                first |= 1 << 7; // 0x80
            }
            output.put_u8(first);
            output.put_all(input);
        } else {
            let mut first = 0x7f;
            if self.ack {
                first |= 1 << 7; // 0x80
            }
            output.put_u8(first);
            output.put_uint(len as _, 3);
            output.put_all(input);
        }
        let end = output.len();

        if let Some(obf) = &mut self.obfuscation {
            info!("encrypt before: {:?}", output);
            obf.aes256_ctr128_encrypt(&mut output[start..end]);
            info!("encrypt after: {:?}", output);
        }
    }

    fn unpack(&mut self, input: &[u8], output: &mut ByteBuffer) -> Result<usize> {
        if input.is_empty() { bail!(Error::MissingBytes); }

        // todo obfuscation
        // match &mut self.obfuscation {
        //     Some(obf) => {
        //         obf.aes256_ctr128_decrypt(input)
        //     }
        //     None => {
        //         // todo
        //     }
        // }

        let header_len;
        let len = input[0];
        let len = if len < 0x7f {
            header_len = 1;
            len as u32
        } else {
            if input.len() < 4 { bail!(Error::MissingBytes); }

            header_len = 4;
            let mut len = [0; 4];
            len[..3].copy_from_slice(&input[1..4]);
            u32::from_le_bytes(len)
        };

        let len = len as usize * 4;
        if input.len() < header_len + len { bail!(Error::MissingBytes); }

        output.put_all(&input[header_len..header_len + len]);
        Ok(header_len + len)
    }
}