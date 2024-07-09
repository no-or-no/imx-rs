use anyhow::{bail, Result};
use bytes::{Buf, BufMut};

use crate::proto::ByteBuffer;
use crate::proto::transport::{Error, Obfuscation, Transport};

/// [Intermediate](https://core.telegram.org/mtproto/mtproto-transports#intermediate)
///
/// In case 4-byte data alignment is needed, an intermediate version of the original protocol may be used.
/// - Overhead: small
/// - Minimum envelope length: 4 bytes
/// - Maximum envelope length: 4 bytes
///
/// Payload structure:
/// ```text
/// +----+----...----+
/// +len.+  payload  +
/// +----+----...----+
/// ```
/// Before sending anything into the underlying socket (see [transports](https://core.telegram.org/mtproto/transports)),
/// the client must first send `0xeeeeeeee` as the first int (four bytes, the server **will not**
/// send `0xeeeeeeee` as the first int in the first reply).
///
/// Then, payloads are wrapped in the following envelope:
/// - Length: payload length encoded as 4 length bytes (little endian)
/// - Payload: the MTProto payload
pub struct Intermediate {
    obfuscation: Option<Obfuscation>,
    ack: bool,
    first_packet_sent: bool,
}

impl Intermediate {
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

impl Transport for Intermediate {

    fn pack(&mut self, input: &[u8], output: &mut ByteBuffer) {
        assert_eq!(input.len() % 4, 0);

        if !self.first_packet_sent {
            match &mut self.obfuscation {
                Some(obf) => {
                    let header = obf.init_header(Some(0xee));
                    output.put_all(&header);
                }
                None => {
                    output.put_u32(0xeeeeeeee);
                }
            }
            self.first_packet_sent = true;
        }

        let start = output.len();
        output.put_u32(input.len() as _);
        output.put_all(input);
        let end = output.len();

        if let Some(obf) = &mut self.obfuscation {
            obf.aes256_ctr128_encrypt(&mut output[start..end]);
        }
    }

    fn unpack(&mut self, input: &[u8], output: &mut ByteBuffer) -> Result<usize> {
        if input.len() < 4 { bail!(Error::MissingBytes); }

        let slice = &mut &input[..];

        let len = slice.get_u32_le() as usize;
        if slice.len() < len { bail!(Error::MissingBytes); }

        output.put_all(&slice[..len]);

        Ok(len + 4)
    }
}