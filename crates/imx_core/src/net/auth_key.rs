use crate::sha1;

#[derive(Debug, Copy, Clone)]
pub struct AuthKey {
    pub id: i64,
    bytes: [u8; 256],
}

impl AuthKey {
    pub fn from_bytes(bytes: [u8; 256]) -> Self {
        let sha = sha1!(bytes);
        let id = {
            let mut arr = [0; 8];
            arr.copy_from_slice(&sha[12..]);
            i64::from_le_bytes(arr)
        };
        Self { id, bytes, }
    }

    pub fn to_bytes(&self) -> [u8; 256] { self.bytes }
}

impl From<[u8; 256]> for AuthKey {
    fn from(value: [u8; 256]) -> Self {
        Self::from_bytes(value)
    }
}

