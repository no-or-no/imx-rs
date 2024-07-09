use anyhow::{bail, Result};
use bytes::{Buf, BufMut};

use crate::proto::ByteBuffer;
use crate::proto::transport::{Error, Obfuscation, Transport};

/// [Full](https://core.telegram.org/mtproto/mtproto-transports#full)
///
/// The basic MTProto transport protocol
/// - Overhead: medium
/// - Minimum envelope length: 12 bytes (length+seqno+crc)
/// - Maximum envelope length: 12 bytes (length+seqno+crc)
///
/// Payload structure:
/// ```text
/// +----+----+----...----+----+
/// |len.|seq.|  payload  |crc.|
/// +----+----+----...----+----+
/// ```
/// Envelope description:
///
/// - Length: length+seqno+payload+crc length encoded as 4 length bytes (little endian, the length
/// of the length field must be included, too)
/// - Seqno: the TCP sequence number for this TCP connection (different from the
/// [MTProto sequence number](https://core.telegram.org/mtproto/description#message-sequence-number-msg-seqno)):
/// the first packet sent is numbered 0, the next one 1, etc.
/// - payload: MTProto payload
/// - crc: 4 CRC32 bytes computed using length, sequence number, and payload together.
pub struct Full {
    obfuscation: Option<Obfuscation>,
    ack: bool,
    first_packet_sent: bool,
    send_seq: u32,
    recv_seq: u32,
}

impl Full {
    pub fn new() -> Self {
        Self {
            obfuscation: None,
            ack: false,
            first_packet_sent: false,
            send_seq: 0,
            recv_seq: 0,
        }
    }

    pub fn ack(self, ack: bool) -> Self {
        Self {
            obfuscation: self.obfuscation,
            ack,
            first_packet_sent: self.first_packet_sent,
            send_seq: self.send_seq,
            recv_seq: self.recv_seq,
        }
    }

    pub fn obfuscation(self, secret: Option<String>) -> Self {
        Self {
            obfuscation: secret.map(|s| Obfuscation::new(s)),
            ack: self.ack,
            first_packet_sent: self.first_packet_sent,
            send_seq: self.send_seq,
            recv_seq: self.recv_seq,
        }
    }
}

impl Transport for Full {

    fn pack(&mut self, input: &[u8], output: &mut ByteBuffer) {
        assert_eq!(input.len() % 4, 0);

        if !self.first_packet_sent {
            if let Some(obf) = &mut self.obfuscation {
                let header = obf.init_header(None);
                output.put_all(&header);
            }
            self.first_packet_sent = true;
        }

        // payload len + length itself (4 bytes) + send seqno (4 bytes) + crc32 (4 bytes)
        let len = input.len() + 12;
        output.reserve(len);

        let buf_start = output.len();
        output.put_u32(len as _);
        output.put_u32(self.send_seq);
        output.put_all(input);

        let crc = crc32fast::hash(&output[buf_start..]);
        output.put_u32(crc);

        self.send_seq += 1;
    }

    fn unpack(&mut self, input: &[u8], output: &mut ByteBuffer) -> Result<usize> {
        // Need 4 bytes for the initial length
        if input.len() < 4 { bail!(Error::MissingBytes); }

        let total_len = input.len();
        let slice = &mut &input[..];

        // payload len
        let len = slice.get_u32_le() as usize;
        if len < 12 { bail!(Error::BadLen { got: len as u32 }); }

        if total_len < len { bail!(Error::MissingBytes); }

        // receive seq
        let seq = slice.get_u32_le();
        if seq != self.recv_seq { bail!(Error::BadSeq { expected: self.recv_seq, got: seq }); }

        // skip payload for now
        slice.advance(len - 12);

        // crc32
        let crc = slice.get_u32_le();

        // 验证 crc
        let valid_crc = crc32fast::hash(&input[..len - 4]);
        if crc != valid_crc { bail!(Error::BadCrc { expected: valid_crc, got: crc}); }

        self.recv_seq += 1;

        output.put_all(&input[8..len - 4]);
        Ok(len)
    }
}