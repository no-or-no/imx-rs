pub use serde_with::*;
pub use with_crc_derive::WithCrc;

mod serde_with;

/// Type id of the bool true value.
pub const CRC_BOOL_TRUE: u32 = 0x997275b5;
/// Type id of the bool false value.
pub const CRC_BOOL_FALSE: u32 = 0xbc799737;
/// Type id of the int type.
pub const CRC_INT: u32 = 0xa8509bda;
/// Type id of the long type.
pub const CRC_LONG: u32 = 0x22076cba;
/// Type id of the double type.
pub const CRC_DOUBLE: u32 = 0x2210c154;
/// Type id of the string type.
pub const CRC_STRING: u32 = 0xb5286e24;
/// Type id of the vector type.
pub const CRC_VECTOR: u32 = 0x1cb5c415;

pub trait WithCrc {
    fn crc(&self) -> u32;
}

impl<'a, T: WithCrc> WithCrc for &'a T {
    fn crc(&self) -> u32 { (*self).crc() }
}

impl<T: WithCrc> WithCrc for Box<T> {
    fn crc(&self) -> u32 { (**self).crc() }
}

impl WithCrc for bool {
    fn crc(&self) -> u32 {
        match *self {
            false => CRC_BOOL_FALSE,
            true => CRC_BOOL_TRUE,
        }
    }
}

impl<'a> WithCrc for &'a str {
    fn crc(&self) -> u32 { CRC_STRING }
}

impl<T> WithCrc for Vec<T> {
    fn crc(&self) -> u32 { CRC_VECTOR }
}

macro_rules! impl_with_crc {
    ($($typo:ty => $type_id:expr,)*) => {
        $(
            impl WithCrc for $typo {
                fn crc(&self) -> u32 {
                    $type_id
                }
            }
        )*
    };
}

impl_with_crc! {
    i8  => CRC_INT,
    i16 => CRC_INT,
    i32 => CRC_INT,
    i64 => CRC_LONG,

    u8  => CRC_INT,
    u16 => CRC_INT,
    u32 => CRC_INT,
    u64 => CRC_LONG,

    f32 => CRC_DOUBLE,
    f64 => CRC_DOUBLE,

    String => CRC_STRING,
}
