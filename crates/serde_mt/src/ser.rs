#![allow(unused_variables)]
use std::io;

use byteorder::WriteBytesExt;
#[cfg(feature = "log")]
use log::debug;
use serde::{ser, Serialize};
use with_crc::WithCrc;

use crate::{bail, MtEndian, Error, Result, cfg_log};
use crate::error::{SerErrorKind, SerSerdeType};
use crate::utils::safe_uint_cast;


/// Serialize the given data structure as a byte vector of binary MTProto.
pub fn to_bytes<T>(value: &T) -> Result<Vec<u8>>
    where T: Serialize
{
    let mut ser = Serializer::new(Vec::new());
    value.serialize(&mut ser)?;

    Ok(ser.writer)
}

/// Serialize bytes with padding to 16 bytes as a byte vector of binary MTProto.
pub fn unsized_bytes_pad_to_bytes(value: &[u8]) -> Result<Vec<u8>> {
    let padding = (16 - value.len() % 16) % 16;
    let mut result = Vec::with_capacity(value.len() + padding);

    result.extend(value);
    for _ in 0..padding {
        result.push(0);
    }

    Ok(result)
}

/// Serialize the given data structure as binary MTProto into the IO stream.
pub fn to_writer<W, T>(writer: W, value: &T) -> Result<()>
    where
        W: io::Write,
        T: Serialize,
{
    let mut ser = Serializer::new(writer);
    value.serialize(&mut ser)?;

    Ok(())
}

/// Serialize bytes with padding to 16 bytes into the IO stream.
pub fn unsized_bytes_pad_to_writer<W>(mut writer: W, value: &[u8]) -> Result<()>
    where W: io::Write
{
    let padding = (16 - value.len() % 16) % 16;

    writer.write_all(value)?;
    for _ in 0..padding {
        writer.write_u8(0)?;
    }

    Ok(())
}


#[derive(Debug)]
pub struct Serializer<W: io::Write> {
    writer: W,
}

impl<W: io::Write> Serializer<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    pub fn writer(&mut self) -> &mut W { &mut self.writer }

    pub fn into_inner(self) -> W {
        self.writer
    }

    /// write bytes with padding
    fn serialize_bytes_pad(&mut self, value: &[u8]) -> Result<()> {
        let len = value.len();
        let rem;

        if len <= 253 {
            // If L <= 253, the serialization contains one byte with the value of L,
            // then L bytes of the string followed by 0 to 3 characters containing 0,
            // such that the overall length of the value be divisible by 4,
            // whereupon all of this is interpreted as a sequence
            // of int(L/4)+1 32-bit little-endian integers.

            self.writer.write_u8(len as u8)?; // `as` is safe: [0..253] \subseteq [0..255]

            rem = (len + 1) % 4;
        } else if len <= 0xff_ff_ff {
            // If L >= 254, the serialization contains byte 254, followed by 3
            // bytes with the string length L in little-endian order, followed by L
            // bytes of the string, further followed by 0 to 3 null padding bytes.

            self.writer.write_u8(254)?;
            self.writer.write_u24::<MtEndian>(len as u32)?; // `as` is safe: [0..0xff_ff_ff] \subseteq [0..0xff_ff_ff_ff]

            rem = len % 4;
        } else {
            bail!(Error::StringTooLong(len));
        }

        // Write each character in the string
        self.writer.write_all(value)?;

        // [...] string followed by 0 to 3 characters containing 0,
        // such that the overall length of the value be divisible by 4 [...]
        if rem > 0 {
            assert!(rem < 4);
            let padding = 4 - rem;
            self.writer.write_uint::<MtEndian>(0, padding)?;
        }

        Ok(())
    }
}

impl<'a, W: io::Write> serde::Serializer for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = SerializeFixedLengthSeq<'a, W>;
    type SerializeTuple = SerializeFixedLengthSeq<'a, W>;
    type SerializeTupleStruct = SerializeFixedLengthSeq<'a, W>;
    type SerializeTupleVariant = SerializeFixedLengthSeq<'a, W>;
    type SerializeMap = SerializeFixedLengthMap<'a, W>;
    type SerializeStruct = SerializeFixedLengthSeq<'a, W>;
    type SerializeStructVariant = SerializeFixedLengthSeq<'a, W>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        self.writer.write_u32::<MtEndian>(v.crc())?;
        cfg_log! { debug!("Serialized bool: {} => {:#x}", v, v.crc()); }
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.writer.write_i8(v)?;
        cfg_log! { debug!("Serialized i8: {:#}", v); }
        Ok(())
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.writer.write_i32::<MtEndian>(v.into())?;
        cfg_log! { debug!("Serialized i16 as i32: {:#x}", v); }
        Ok(())
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.writer.write_i32::<MtEndian>(v)?;
        cfg_log! { debug!("Serialized i32: {:#x}", v); }
        Ok(())
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        self.writer.write_i64::<MtEndian>(v)?;
        cfg_log! { debug!("Serialized i64: {:#x}", v); }
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.writer.write_u8(v)?;
        cfg_log! { debug!("Serialized u8: {:#}", v); }
        Ok(())
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.writer.write_u32::<MtEndian>(v.into())?;
        cfg_log! { debug!("Serialized u16 as u32: {:#x}", v); }
        Ok(())
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.writer.write_u32::<MtEndian>(v)?;
        cfg_log! { debug!("Serialized u32: {:#x}", v); }
        Ok(())
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        self.writer.write_u64::<MtEndian>(v)?;
        cfg_log! { debug!("Serialized u64: {:#x}", v); }
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.writer.write_f64::<MtEndian>(v.into())?;
        cfg_log! { debug!("Serialized f32 as f64: {}", v); }
        Ok(())
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        self.writer.write_f64::<MtEndian>(v)?;
        cfg_log! { debug!("Serialized f64: {}", v); }
        Ok(())
    }

    fn serialize_char(self, _v: char) -> Result<Self::Ok, Self::Error> {
        bail!(SerErrorKind::UnsupportedSerdeType(SerSerdeType::Char));
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.serialize_bytes_pad(v.as_bytes())?;
        cfg_log! { debug!("Serialized str: {:?}", v); }
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.serialize_bytes_pad(v)?;
        cfg_log! { debug!("Serialized bytes: {:?}", v); }
        Ok(())
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        bail!(SerErrorKind::UnsupportedSerdeType(SerSerdeType::None));
    }

    fn serialize_some<T: ?Sized>(self, _value: &T) -> Result<Self::Ok, Self::Error> where T: Serialize {
        bail!(SerErrorKind::UnsupportedSerdeType(SerSerdeType::Some));
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        bail!(SerErrorKind::UnsupportedSerdeType(SerSerdeType::Unit));
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        cfg_log! { debug!("Serialized unit struct"); }
        Ok(())
    }

    fn serialize_unit_variant(self, _name: &'static str, _variant_index: u32, _variant: &'static str) -> Result<Self::Ok, Self::Error> {
        cfg_log! { debug!("Serialized unit variant"); }
        Ok(())
    }

    fn serialize_newtype_struct<T: ?Sized>(self, name: &'static str, value: &T) -> Result<Self::Ok, Self::Error> where T: Serialize {
        cfg_log! { debug!("Serializing newtype struct {}", name); }
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(self, name: &'static str, variant_index: u32, variant: &'static str, value: &T) -> Result<Self::Ok, Self::Error> where T: Serialize {
        cfg_log! { debug!("Serializing newtype variant {}::{} (variant index {})", name, variant, variant_index); }
        value.serialize(self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        if let Some(len) = len {
            cfg_log! { debug!("Serializing seq of len {}", len); }
            SerializeFixedLengthSeq::with_serialize_len(self, safe_uint_cast(len)?)
        } else {
            bail!(SerErrorKind::SeqsWithUnknownLengthUnsupported);
        }
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        cfg_log! { debug!("Serializing tuple of len {}", len); }
        Ok(SerializeFixedLengthSeq::new(self, safe_uint_cast(len)?))
    }

    fn serialize_tuple_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeTupleStruct, Self::Error> {
        cfg_log! { debug!("Serializing tuple struct {} of len {}", name, len); }
        Ok(SerializeFixedLengthSeq::new(self, safe_uint_cast(len)?))
    }

    fn serialize_tuple_variant(self, name: &'static str, variant_index: u32, variant: &'static str, len: usize) -> Result<Self::SerializeTupleVariant, Self::Error> {
        cfg_log! { debug!("Serializing tuple variant {}::{} (variant index {}) of len {}",
            name, variant, variant_index, len); }
        Ok(SerializeFixedLengthSeq::new(self, safe_uint_cast(len)?))
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        if let Some(len) = len {
            cfg_log! { debug!("Serializing map of len {}", len); }
            SerializeFixedLengthMap::with_serialize_len(self, safe_uint_cast(len)?)
        } else {
            bail!(SerErrorKind::MapsWithUnknownLengthUnsupported);
        }
    }

    fn serialize_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeStruct, Self::Error> {
        cfg_log! { debug!("Serializing struct {} of len {}", name, len); }
        Ok(SerializeFixedLengthSeq::new(self, safe_uint_cast(len)?))
    }

    fn serialize_struct_variant(self, name: &'static str, variant_index: u32, variant: &'static str, len: usize) -> Result<Self::SerializeStructVariant, Self::Error> {
        cfg_log! { debug!("Serializing struct variant {}::{} (variant index {}) of len {}",
            name, variant, variant_index, len); }
        Ok(SerializeFixedLengthSeq::new(self, safe_uint_cast(len)?))
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

/// Helper structure for serializing fixed-length sequences.
#[derive(Debug)]
pub struct SerializeFixedLengthSeq<'a, W: io::Write> {
    ser: &'a mut Serializer<W>,
    len: u32,
    next_index: u32,
}

impl<'a, W: io::Write> SerializeFixedLengthSeq<'a, W> {

    fn new(ser: &'a mut Serializer<W>, len: u32) -> Self {
        Self { ser, len, next_index: 0 }
    }

    fn with_serialize_len(ser: &'a mut Serializer<W>, len: u32) -> Result<Self, Error> {
        ser::Serializer::serialize_u32(&mut *ser, len)?;

        Ok(Self::new(ser, len))
    }

    fn impl_serialize_seq_value<T>(
        &mut self, key: Option<&'static str>, value: &T, serializer_type: &'static str) -> Result<(), Error>
        where T: ?Sized + Serialize {
        if self.next_index < self.len {
            self.next_index += 1;
        } else {
            cfg_log! { debug!("{}::serialize_element() is called when no elements is left to serialize", serializer_type); }

            bail!(SerErrorKind::ExcessElements(self.len));
        }

        if let Some(key) = key {
            cfg_log! { debug!("Serializing field {}", key); }
        } else {
            cfg_log! { debug!("Serializing element"); }
        }

        value.serialize(&mut *self.ser)
    }

    fn impl_serialize_end(self, data_type: &'static str) -> Result<(), Error> {
        if self.next_index < self.len {
            bail!(SerErrorKind::NotEnoughElements(self.next_index, self.len))
        }

        // `self.index > self.len` here is a programming error
        assert_eq!(self.next_index, self.len);

        cfg_log! { debug!("Finished serializing {}", data_type); }

        Ok(())
    }
}

impl<'a, W> ser::SerializeSeq for SerializeFixedLengthSeq<'a, W>
    where W: 'a + io::Write
{
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<Self::Ok, Self::Error> where T: Serialize {
        self.impl_serialize_seq_value(None, value, "SerializeSeq")
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.impl_serialize_end("seq")
    }
}

impl<'a, W> ser::SerializeTuple for SerializeFixedLengthSeq<'a, W>
    where W: 'a + io::Write
{
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<Self::Ok, Self::Error> where T: Serialize {
        self.impl_serialize_seq_value(None, value, "SerializeTuple")
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.impl_serialize_end("tuple")
    }
}

impl<'a, W> ser::SerializeTupleStruct for SerializeFixedLengthSeq<'a, W>
    where W: io::Write
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<Self::Ok, Self::Error> where T: Serialize {
        self.impl_serialize_seq_value(None, value, "SerializeTupleStruct")
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.impl_serialize_end("tuple struct")
    }
}

impl<'a, W> ser::SerializeTupleVariant for SerializeFixedLengthSeq<'a, W>
    where W: io::Write
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<Self::Ok, Self::Error> where T: Serialize {
        self.impl_serialize_seq_value(None, value, "SerializeTupleVariant")
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.impl_serialize_end("tuple variant")
    }
}

impl<'a, W> ser::SerializeStruct for SerializeFixedLengthSeq<'a, W>
    where W: io::Write
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<Self::Ok, Self::Error> where T: Serialize {
        self.impl_serialize_seq_value(Some(key), value, "SerializeStruct")
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.impl_serialize_end("struct")
    }
}

impl<'a, W> ser::SerializeStructVariant for SerializeFixedLengthSeq<'a, W>
    where W: io::Write
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<Self::Ok, Self::Error> where T: Serialize {
        self.impl_serialize_seq_value(Some(key), value, "SerializeStructVariant")
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.impl_serialize_end("struct variant")
    }
}


/// Helper structure for serializing maps.
#[derive(Debug)]
pub struct SerializeFixedLengthMap<'a, W: io::Write> {
    ser: &'a mut Serializer<W>,
    len: u32,
    next_index: u32,
}

impl<'a, W: io::Write> SerializeFixedLengthMap<'a, W> {
    fn with_serialize_len(ser: &'a mut Serializer<W>, len: u32) -> Result<Self> {
        ser::Serializer::serialize_u32(&mut *ser, len)?;

        Ok(SerializeFixedLengthMap { ser, len, next_index: 0 })
    }
}

impl<'a, W> ser::SerializeMap for SerializeFixedLengthMap<'a, W>
    where W: io::Write
{
    type Ok = ();
    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<Self::Ok, Self::Error> where T: Serialize {
        if self.next_index < self.len {
            self.next_index += 1;
        } else {
            cfg_log! { debug!("SerializeMap::serialize_key() is called when no elements is left to serialize"); }

            bail!(SerErrorKind::ExcessElements(self.len));
        }

        cfg_log! { debug!("Serializing key"); }
        key.serialize(&mut *self.ser)
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<Self::Ok, Self::Error> where T: Serialize {
        cfg_log! { debug!("Serializing value"); }
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        cfg_log! { debug!("Finished serializing map"); }
        Ok(())
    }

}