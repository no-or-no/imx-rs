use byteorder::LittleEndian;

pub use crate::error::Error;
pub use crate::ser::{
    Serializer,
    to_bytes, unsized_bytes_pad_to_bytes,
    to_writer, unsized_bytes_pad_to_writer,
};
pub use crate::de::{
    Deserializer,
    from_bytes, from_bytes_reuse, from_bytes_seed,
    from_reader, from_reader_reuse, from_reader_seed,
};

mod error;
mod ser;
mod de;
mod utils;

pub type Result<T, E = Error> = core::result::Result<T, E>;

type MtEndian = LittleEndian;

type IntMax = i64;
type UIntMax = u64;

#[macro_export(local_inner_macros)]
macro_rules! cfg_log {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "log")]
            $item
        )*
    }
}

