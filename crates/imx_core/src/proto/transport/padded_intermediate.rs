use anyhow::Result;
use bytes::BufMut;
use rand::{Rng, RngCore, thread_rng};

use crate::proto::ByteBuffer;
use crate::proto::transport::{Obfuscation, Transport};

/// [Padded intermediate](https://core.telegram.org/mtproto/mtproto-transports#padded-intermediate)
///
/// Padded version of the [intermediate protocol](https://core.telegram.org/mtproto/mtproto-transports#intermediate),
/// to use with [obfuscation enabled](https://core.telegram.org/mtproto/mtproto-transports#transport-obfuscation)
/// to **bypass ISP blocks**.
/// - Overhead: small-medium
/// - Minimum envelope length: random
/// - Maximum envelope length: random
///
/// Before sending anything into the underlying socket (see [transports](https://core.telegram.org/mtproto/transports)),
/// the client must first send
/// `0xdddddddd` as the first int (four bytes, the server **will not** send `0xdddddddd` as the first
/// int in the first reply).
///
/// Then, payloads are wrapped in the following envelope:
/// ```text
/// +----+----...----+----...----+
/// |tlen|  payload  |  padding  |
/// +----+----...----+----...----+
/// ```
/// Envelope description:
///
/// - Total length: payload+padding length encoded as 4 length bytes (little endian)
/// - Payload: the MTProto payload
/// - Padding: A random padding string of length `0-15`
pub struct PaddedIntermediate {
    obfuscation: Option<Obfuscation>,
    ack: bool,
    first_packet_sent: bool,
}

impl PaddedIntermediate {
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

impl Transport for PaddedIntermediate {

    fn pack(&mut self, input: &[u8], output: &mut ByteBuffer) {
        assert_eq!(input.len() % 4, 0);

        if !self.first_packet_sent {
            match &mut self.obfuscation {
                Some(obf) => {
                    let header = obf.init_header(Some(0xdd));
                    output.put_all(&header);
                }
                None => {
                    output.put_u32(0xdddddddd);
                }
            }
            self.first_packet_sent = true;
        }

        let start = output.len();
        let mut rng = thread_rng();
        let padding_len = rng.gen_range(0..16);
        output.put_u32((padding_len + input.len()) as _);
        output.put_all(input);
        if padding_len > 0 {
            let mut padding = vec![0; padding_len];
            rng.fill_bytes(&mut padding);
        }
        let end = output.len();

        if let Some(obf) = &mut self.obfuscation {
            obf.aes256_ctr128_encrypt(&mut output[start..end]);
        }
    }

    fn unpack(&mut self, input: &[u8], output: &mut ByteBuffer) -> Result<usize> {
        todo!()
    }

}