#![allow(unused_variables)]
use std::io;

use byteorder::ReadBytesExt;
#[cfg(feature = "log")]
use log::debug;
use serde::{de, Deserialize};
use serde::de::{DeserializeOwned, DeserializeSeed, Visitor};
use with_crc::{CRC_BOOL_FALSE, CRC_BOOL_TRUE};

use crate::{bail, MtEndian, Error, Result, cfg_log};
use crate::error::{DeErrorKind, DeSerdeType};
use crate::utils::{safe_float_cast, safe_int_cast, safe_uint_cast};


/// Deserialize an instance of type `T` from bytes of binary MTProto.
pub fn from_bytes<'de, T>(bytes: &'de [u8], enum_variant_ids: &[&'static str]) -> Result<T>
    where
        T: Deserialize<'de>
{
    let mut de = Deserializer::new(bytes, enum_variant_ids);
    let value: T = Deserialize::deserialize(&mut de)?;

    Ok(value)
}

/// Deserialize an instance of type `T` from bytes of binary MTProto and return unused bytes.
pub fn from_bytes_reuse<'de, T>(bytes: &'de [u8], enum_variant_ids: &[&'static str]) -> Result<(T, &'de [u8])>
    where
        T: Deserialize<'de>
{
    let mut de = Deserializer::new(bytes, enum_variant_ids);
    let value: T = Deserialize::deserialize(&mut de)?;

    Ok((value, de.reader))
}

/// Deserialize an instance of type `T` from bytes of binary MTProto using a seed.
pub fn from_bytes_seed<'de, S, T>(seed: S, bytes: &'de [u8], enum_variant_ids: &[&'static str]) -> Result<T>
    where
        S: DeserializeSeed<'de, Value = T>
{
    let mut de = Deserializer::new(bytes, enum_variant_ids);
    let value: T = DeserializeSeed::deserialize(seed, &mut de)?;

    Ok(value)
}

/// Deserialize an instance of type `T` from an IO stream of binary MTProto.
pub fn from_reader<R, T>(reader: R, enum_variant_ids: &[&'static str]) -> Result<T>
    where
        R: io::Read,
        T: DeserializeOwned,
{
    let mut de = Deserializer::new(reader, enum_variant_ids);
    let value: T = Deserialize::deserialize(&mut de)?;

    Ok(value)
}

/// Deserialize an instance of type `T` from an IO stream of binary MTProto and return unused part
/// of IO stream.
pub fn from_reader_reuse<R, T>(reader: R, enum_variant_ids: &[&'static str]) -> Result<(T, R)>
    where
        R: io::Read,
        T: DeserializeOwned,
{
    let mut de = Deserializer::new(reader, enum_variant_ids);
    let value: T = Deserialize::deserialize(&mut de)?;

    Ok((value, de.reader))
}

/// Deserialize an instance of type `T` from an IO stream of binary MTProto using a seed.
pub fn from_reader_seed<R, S, T>(seed: S, reader: R, enum_variant_ids: &[&'static str]) -> Result<T>
    where
        S: for<'de> DeserializeSeed<'de, Value = T>,
        R: io::Read,
{
    let mut de = Deserializer::new(reader, enum_variant_ids);
    let value: T = DeserializeSeed::deserialize(seed, &mut de)?;

    Ok(value)
}


#[derive(Debug)]
pub struct Deserializer<'ids, R: io::Read> {
    pub(crate) reader: R,
    enum_variant_ids: &'ids [&'static str],
}

impl<'ids, R: io::Read> Deserializer<'ids, R> {
    /// Create a MTProto deserializer from an `io::Read` and enum variant hint.
    pub fn new(reader: R, enum_variant_ids: &'ids [&'static str]) -> Deserializer<'ids, R> {
        Deserializer { reader, enum_variant_ids }
    }

    /// Unwraps the `Deserializer` and returns the underlying `io::Read`.
    pub fn into_reader(self) -> R {
        self.reader
    }

    /// Consumes the `Deserializer` and returns remaining unprocessed bytes.
    pub fn remaining_bytes(mut self) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        let bytes_read = self.reader.read_to_end(&mut buf)?;

        cfg_log! { debug!("Remaining bytes read of length {}: {:?}", bytes_read, buf); }

        Ok(buf)
    }

    fn get_str_info(&mut self) -> Result<(usize, usize)> {
        let first_byte = self.reader.read_u8()?;
        let len;
        let rem;

        match first_byte {
            0 ..= 253 => {
                len = usize::from(first_byte);
                rem = (len + 1) % 4;
            },
            254 => {
                let uncasted = self.reader.read_u24::<MtEndian>()?;
                if uncasted <= 253 {
                    bail!(DeErrorKind::BytesLenPrefix254LessThan254(uncasted));
                }

                len = safe_uint_cast::<u32, usize>(uncasted)?;
                rem = len % 4;
            },
            255 => {
                return Err(de::Error::invalid_value(
                    de::Unexpected::Unsigned(255),
                    &"a byte in [0..254] range"));
            },
            #[allow(unreachable_patterns)]
            _ => unreachable!("other match arms should have exhaustively covered every value"),
        }

        let padding = (4 - rem) % 4;

        Ok((len, padding))
    }

    fn read_string(&mut self) -> Result<String> {
        let s_bytes = self.read_byte_buf()?;
        let s = String::from_utf8(s_bytes)?;

        Ok(s)
    }

    fn read_byte_buf(&mut self) -> Result<Vec<u8>> {
        let (len, padding) = self.get_str_info()?;

        let mut b = vec![0; len];
        self.reader.read_exact(&mut b)?;

        let mut p = [0; 3];
        let ps = p.get_mut(0..padding)
            .unwrap_or_else(|| unreachable!("padding must be of length 3 or less"));
        self.reader.read_exact(ps)?;

        if ps.iter().any(|b| *b != 0) {
            bail!(DeErrorKind::NonZeroBytesPadding);
        }

        Ok(b)
    }
}

impl<'ids, 'a> Deserializer<'ids, &'a [u8]> {
    /// Length of unprocessed data in the byte buffer.
    pub fn remaining_length(&self) -> usize {
        self.reader.len()
    }
}

impl<'de, 'a, 'ids, R: io::Read> de::Deserializer<'de> for &'a mut Deserializer<'ids, R> {
    type Error = Error;

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        bail!(DeErrorKind::UnsupportedSerdeType(DeSerdeType::Any));
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        let id_value = self.reader.read_u32::<MtEndian>()?;

        let value = match id_value {
            CRC_BOOL_FALSE => false,
            CRC_BOOL_TRUE => true,
            _ => {
                return Err(de::Error::invalid_value(
                    de::Unexpected::Signed(i64::from(id_value)),
                    &format!("either {} for false or {} for true", CRC_BOOL_FALSE, CRC_BOOL_TRUE).as_str()));
            }
        };

        cfg_log! { debug!("Deserialized bool: {}", value); }

        visitor.visit_bool(value)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        let v = self.reader.read_i32::<MtEndian>()?;
        cfg_log! { debug!("Deserialized i32: {:#x}", v); }
        let casted = safe_int_cast(v)?;
        cfg_log! { debug!("Casted to i8: {:#x}", casted); }
        visitor.visit_i8(casted)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        let v = self.reader.read_i32::<MtEndian>()?;
        cfg_log! { debug!("Deserialized i32: {:#x}", v); }
        let casted = safe_int_cast(v)?;
        cfg_log! { debug!("Casted to i16: {:#x}", casted); }
        visitor.visit_i16(casted)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        let v = self.reader.read_i32::<MtEndian>()?;
        cfg_log! { debug!("Deserialized i32: {:#x}", v); }
        visitor.visit_i32(v)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        let v = self.reader.read_i64::<MtEndian>()?;
        cfg_log! { debug!("Deserialized i64: {:#x}", v); }
        visitor.visit_i64(v)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        let v = self.reader.read_u32::<MtEndian>()?;
        cfg_log! { debug!("Deserialized u32: {:#x}", v); }
        let casted = safe_uint_cast(v)?;
        cfg_log! { debug!("Casted to u8: {:#x}", casted); }
        visitor.visit_u8(casted)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        let v = self.reader.read_u32::<MtEndian>()?;
        cfg_log! { debug!("Deserialized u32: {:#x}", v); }
        let casted = safe_uint_cast(v)?;
        cfg_log! { debug!("Casted to u16: {:#x}", casted); }
        visitor.visit_u16(casted)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        let v = self.reader.read_u32::<MtEndian>()?;
        cfg_log! { debug!("Deserialized u32: {:#x}", v); }
        visitor.visit_u32(v)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        let v = self.reader.read_u64::<MtEndian>()?;
        cfg_log! { debug!("Deserialized u64: {:#x}", v); }
        visitor.visit_u64(v)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        let value = self.reader.read_f64::<MtEndian>()?;
        cfg_log! { debug!("Deserialized f64: {}", value); }

        let casted = safe_float_cast(value)?;
        cfg_log! { debug!("Casted to f32: {}", casted); }

        visitor.visit_f32(casted)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        let value = self.reader.read_f64::<MtEndian>()?;
        cfg_log! { debug!("Deserialized f64: {}", value); }

        visitor.visit_f64(value)
    }

    fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        bail!(DeErrorKind::UnsupportedSerdeType(DeSerdeType::Char));
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        let s = self.read_string()?;
        cfg_log! { debug!("Deserialized str: {:?}", s); }
        visitor.visit_str(&s)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        let s = self.read_string()?;
        cfg_log! { debug!("Deserialized string: {:?}", s); }
        visitor.visit_string(s)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        let b = self.read_byte_buf()?;
        cfg_log! { debug!("Deserialized bytes: {:?}", b); }
        visitor.visit_bytes(&b)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        let b = self.read_byte_buf()?;
        cfg_log! { debug!("Deserialized byte buffer: {:?}", b); }
        visitor.visit_byte_buf(b)
    }

    fn deserialize_option<V>(self, _visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        bail!(DeErrorKind::UnsupportedSerdeType(DeSerdeType::Option));
    }

    fn deserialize_unit<V>(self, _visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        bail!(DeErrorKind::UnsupportedSerdeType(DeSerdeType::Unit));
    }

    fn deserialize_unit_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        cfg_log! { debug!("Deserialized unit struct {}", name); }
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        cfg_log! { debug!("Deserializing newtype struct {}", name); }
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        let len = self.reader.read_u32::<MtEndian>()?;
        cfg_log! { debug!("Deserializing seq of len {}", len); }

        visitor.visit_seq(SeqAccess::new(self, len))
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        cfg_log! { debug!("Deserializing tuple of len {}", len); }
        visitor.visit_seq(SeqAccess::new(self, safe_uint_cast(len)?))
    }

    fn deserialize_tuple_struct<V>(self, name: &'static str, len: usize, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        cfg_log! { debug!("Deserializing tuple struct {} of len {}", name, len); }
        visitor.visit_seq(SeqAccess::new(self, safe_uint_cast(len)?))
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        let len = self.reader.read_u32::<MtEndian>()?;
        cfg_log! { debug!("Deserializing map of len {}", len); }

        visitor.visit_map(MapAccess::new(self, len))
    }

    fn deserialize_struct<V>(self, name: &'static str, fields: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        cfg_log! { debug!("Deserializing struct {} with fields {:?}", name, fields); }
        visitor.visit_seq(SeqAccess::new(self, safe_uint_cast(fields.len())?))
    }

    fn deserialize_enum<V>(self, name: &'static str, variants: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        cfg_log! { debug!("Deserializing enum {} with variants {:?}", name, variants); }
        visitor.visit_enum(EnumVariantAccess::new(self))
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        cfg_log! { debug!("Deserializing identifier"); }
        let (variant_id, rest) = self.enum_variant_ids.split_first()
            .ok_or_else(|| Error::from(DeErrorKind::NoEnumVariantId))?;

        cfg_log! { debug!("Deserialized variant_id {}", variant_id); }
        self.enum_variant_ids = rest;

        visitor.visit_str(variant_id)
    }

    fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        bail!(DeErrorKind::UnsupportedSerdeType(DeSerdeType::IgnoredAny));
    }
}

#[derive(Debug)]
struct SeqAccess<'a, 'ids: 'a, R: io::Read> {
    de: &'a mut Deserializer<'ids, R>,
    len: u32,
    next_index: u32,
}

impl<'a, 'ids, R: io::Read> SeqAccess<'a, 'ids, R> {
    fn new(de: &'a mut Deserializer<'ids, R>, len: u32) -> SeqAccess<'a, 'ids, R> {
        SeqAccess { de, len, next_index: 0 }
    }
}

impl<'de, 'a, 'ids, R> de::SeqAccess<'de> for SeqAccess<'a, 'ids, R>
    where R: 'a + io::Read
{
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
        where T: DeserializeSeed<'de>
    {
        if self.next_index < self.len {
            self.next_index += 1;
        } else {
            cfg_log! { debug!("SeqAccess::next_element_seed() is called when no elements is left to deserialize"); }
            return Ok(None);
        }

            cfg_log! { debug!("Deserializing sequence element"); }
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn size_hint(&self) -> Option<usize> {
        safe_uint_cast(self.len - self.next_index).ok()
    }
}


#[derive(Debug)]
struct MapAccess<'a, 'ids: 'a, R: io::Read> {
    de: &'a mut Deserializer<'ids, R>,
    len: u32,
    next_index: u32,
}

impl<'a, 'ids, R: io::Read> MapAccess<'a, 'ids, R> {
    fn new(de: &'a mut Deserializer<'ids, R>, len: u32) -> MapAccess<'a, 'ids, R> {
        MapAccess { de, len, next_index: 0 }
    }
}

impl<'de, 'a, 'ids, R> de::MapAccess<'de> for MapAccess<'a, 'ids, R>
    where R: 'a + io::Read
{
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
        where K: DeserializeSeed<'de>
    {
        if self.next_index < self.len {
            self.next_index += 1;
        } else {
            cfg_log! { debug!("MapAccess::next_key_seed() is called when no elements is left to deserialize"); }
            return Ok(None);
        }

            cfg_log! { debug!("Deserializing map key"); }
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
        where V: DeserializeSeed<'de>
    {
        cfg_log! { debug!("Deserializing map value"); }
        seed.deserialize(&mut *self.de)
    }

    fn size_hint(&self) -> Option<usize> {
        safe_uint_cast(self.len - self.next_index).ok()
    }
}


#[derive(Debug)]
struct EnumVariantAccess<'a, 'ids: 'a, R: io::Read> {
    de: &'a mut Deserializer<'ids, R>,
}

impl<'a, 'ids, R: io::Read> EnumVariantAccess<'a, 'ids, R> {
    fn new(de: &'a mut Deserializer<'ids, R>) -> EnumVariantAccess<'a, 'ids, R> {
        EnumVariantAccess { de }
    }
}

impl<'de, 'a, 'ids, R> de::EnumAccess<'de> for EnumVariantAccess<'a, 'ids, R>
    where R: 'a + io::Read
{
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
        where V: DeserializeSeed<'de>
    {
        cfg_log! { debug!("Deserializing enum variant"); }
        let value = seed.deserialize(&mut *self.de)?;

        Ok((value, self))
    }
}

impl<'de, 'a, 'ids, R> de::VariantAccess<'de> for EnumVariantAccess<'a, 'ids, R>
    where R: 'a + io::Read
{
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        cfg_log! { debug!("Deserialized unit variant"); }
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
        where T: DeserializeSeed<'de>
    {
        cfg_log! { debug!("Deserializing newtype variant"); }
        seed.deserialize(self.de)
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        cfg_log! { debug!("Deserializing tuple variant"); }
        de::Deserializer::deserialize_tuple_struct(self.de, "", len, visitor)
    }

    fn struct_variant<V>(self, fields: &'static [&'static str], visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        cfg_log! { debug!("Deserializing struct variant"); }
        de::Deserializer::deserialize_struct(self.de, "", fields, visitor)
    }
}
