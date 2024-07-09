use std::ops::{Deref, DerefMut};

use bytes::{Buf, BufMut, Bytes, BytesMut};
use serde::{Serialize, Serializer};

/// **LittleEndian**
#[derive(Debug, Clone)]
pub struct ByteBuffer(BytesMut);

impl ByteBuffer {
    #[inline]
    pub fn new() -> Self {
        Self(BytesMut::new())
    }

    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self(BytesMut::with_capacity(capacity))
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// 预留 [additional] bytes 的空间, 如果缓冲区剩余空间不足, 将会扩容
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional);
    }

    #[inline]
    pub fn to_bytes(self) -> Bytes {
        self.0.freeze()
    }

    #[inline]
    pub fn clear(&mut self) {
        self.0.clear()
    }

    #[inline]
    pub fn get_u8(&mut self) -> u8 {
        self.0.get_u8()
    }

    #[inline]
    pub fn get_i32(&mut self) -> i32 {
        self.0.get_i32()
    }

    #[inline]
    pub fn get_i64(&mut self) -> i64 {
        self.0.get_i64()
    }

    #[inline]
    pub fn put_all(&mut self, src: &[u8]) {
        self.0.extend_from_slice(src);
    }

    #[inline]
    pub fn put_u8(&mut self, n: u8) {
        self.0.put_u8(n);
    }

    #[inline]
    pub fn put_i32(&mut self, n: i32) {
        self.0.put_i32_le(n);
    }

    #[inline]
    pub fn put_u32(&mut self, n: u32) {
        self.0.put_u32_le(n);
    }

    #[inline]
    pub fn put_i64(&mut self, n: i64) {
        self.0.put_i64_le(n);
    }

    #[inline]
    pub fn put_u64(&mut self, n: u64) {
        self.0.put_u64_le(n);
    }

    #[inline]
    pub fn put_int(&mut self, n: i64, nbytes: usize) {
        self.0.put_int_le(n, nbytes);
    }

    #[inline]
    pub fn put_uint(&mut self, n: u64, nbytes: usize) {
        self.0.put_uint_le(n, nbytes);
    }
}

/*impl Write for ByteBuffer {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.extend_from_slice(buf);
        Ok(buf.len())
    }

    #[inline]
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        let len = bufs.iter().map(|b| b.len()).sum();
        self.0.reserve(len);
        for buf in bufs {
            self.0.extend_from_slice(buf);
        }
        Ok(len)
    }

    #[inline]
    fn is_write_vectored(&self) -> bool {
        true
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.0.extend_from_slice(buf);
        Ok(())
    }
}*/

impl Serialize for ByteBuffer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        self.0.serialize(serializer)
    }
}

impl From<Vec<u8>> for ByteBuffer {
    #[inline]
    fn from(value: Vec<u8>) -> Self {
        Self(BytesMut::from(value.as_slice()))
    }
}

impl <const N: usize> From<[u8; N]> for ByteBuffer {
    #[inline]
    fn from(value: [u8; N]) -> Self {
        Self(BytesMut::from(value.as_slice()))
    }
}

impl<'a> From<&'a [u8]> for ByteBuffer {
    #[inline]
    fn from(value: &'a [u8]) -> Self {
        Self(BytesMut::from(value))
    }
}

impl AsRef<[u8]> for ByteBuffer {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl AsMut<[u8]> for ByteBuffer {
    #[inline]
    fn as_mut(&mut self) -> &mut [u8] {
        self.0.as_mut()
    }
}

impl Deref for ByteBuffer {
    type Target = [u8];
    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl DerefMut for ByteBuffer {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}
