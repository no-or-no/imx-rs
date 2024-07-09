use thiserror::Error;
use crate::{IntMax, UIntMax};

#[macro_export]
macro_rules! bail {
    ($e:expr) => {
        return Err($e.into())
    };
    ($fmt:expr, $($arg:tt)+) => {
        return Err(format!($fmt, $($arg)+).into())
    };
}

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
    #[error("io err: {0}")]
    Io(#[from] std::io::Error),
    #[error("from utf-8 err: {0}")]
    FromUtf8(#[from] std::string::FromUtf8Error),
    #[error("serialization error in serde_mt: {0}")]
    Ser(SerErrorKind),
    #[error("deserialization error in serde_mt: {0}")]
    De(DeErrorKind),
    #[error("error while casting a signed integer: {0}")]
    SignedIntegerCast(IntMax),
    #[error("error while casting an unsigned integer: {0}")]
    UnsignedIntegerCast(UIntMax),
    #[error("error while casting a floating-point number: {0}")]
    FloatCast(f64),
    #[error("string of length {0} is too long to serialize")]
    StringTooLong(usize),
    #[error("byte sequence of length {0} is too long to serialize")]
    ByteSeqTooLong(usize),
    #[error("sequence of length {0} is too long to serialize")]
    SeqTooLong(usize),
}

impl serde::ser::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        SerErrorKind::Msg(msg.to_string()).into()
    }
}

impl serde::de::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        DeErrorKind::Msg(msg.to_string()).into()
    }
}

impl From<SerErrorKind> for Error {
    fn from(kind: SerErrorKind) -> Error {
        Error::Ser(kind).into()
    }
}

impl From<DeErrorKind> for Error {
    fn from(kind: DeErrorKind) -> Error {
        Error::De(kind).into()
    }
}


#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SerErrorKind {
    /// A convenient variant for String.
    // #[error("custom string: {0}")]
    Msg(String),
    /// Excess elements found, stores the needed count.
    // #[error("excess elements, need no more than {0}")]
    ExcessElements(u32),
    /// Cannot serialize maps with unknown length.
    // #[error("maps with ahead-of-time unknown length are not supported")]
    MapsWithUnknownLengthUnsupported,
    /// Not enough elements, stores the actual and needed count.
    // #[error("not enough elements: have {0}, need {1}")]
    NotEnoughElements(u32, u32),
    /// Cannot serialize sequences with unknown length.
    // #[error("seqs with ahead-of-time unknown length are not supported")]
    SeqsWithUnknownLengthUnsupported,
    /// A string that cannot be serialized because it exceeds a certain length limit.
    // #[error("string of length {0} is too long to serialize")]
    StringTooLong(usize),
    /// This `serde` data format doesn't support several types in the Serde data model.
    // #[error("{0} type is not supported for serialization")]
    UnsupportedSerdeType(SerSerdeType),
}

impl std::fmt::Display for SerErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            SerErrorKind::Msg(ref string) => {
                write!(f, "custom string: {}", string)
            },
            SerErrorKind::ExcessElements(len) => {
                write!(f, "excess elements, need no more than {}", len)
            },
            SerErrorKind::MapsWithUnknownLengthUnsupported => {
                write!(f, "maps with ahead-of-time unknown length are not supported")
            },
            SerErrorKind::NotEnoughElements(unexpected_len, expected_len) => {
                write!(f, "not enough elements: have {}, need {}", unexpected_len, expected_len)
            },
            SerErrorKind::SeqsWithUnknownLengthUnsupported => {
                write!(f, "seqs with ahead-of-time unknown length are not supported")
            },
            SerErrorKind::StringTooLong(len) => {
                write!(f, "string of length {} is too long to serialize", len)
            },
            SerErrorKind::UnsupportedSerdeType(ref type_) => {
                write!(f, "{} type is not supported for serialization", type_)
            },
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SerSerdeType {
    /// Single character type.
    Char,
    /// None value of Option type.
    None,
    /// Some value of Option type.
    Some,
    /// Unit `()` type.
    Unit,
}

impl std::fmt::Display for SerSerdeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let repr = match *self {
            SerSerdeType::Char => "char",
            SerSerdeType::None => "none",
            SerSerdeType::Some => "some",
            SerSerdeType::Unit => "unit",
        };

        f.write_str(repr)
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DeErrorKind {
    /// A convenient variant for String.
    // #[error("custom string: {0}")]
    Msg(String),
    /// Bytes sequence with the 0xfe prefix, but with length less than 254.
    // #[error("byte sequence has the 0xfe prefix with length less than 254 {0}")]
    BytesLenPrefix254LessThan254(u32),
    /// Padding for a bytes sequence that has at least one non-zero byte.
    // #[error("byte sequence has a padding with a non-zero byte")]
    NonZeroBytesPadding,
    /// This `serde` data format doesn't support several types in the Serde data model.
    // #[error("{0} type is not supported for deserialization")]
    UnsupportedSerdeType(DeSerdeType),
    /// Not enough elements, stores the already deserialized and expected count.
    // #[error("not enough elements: have {0}, need {1}")]
    NotEnoughElements(u32, u32),
    /// A wrong map key found while deserializing.
    // #[error("invalid map key {0:?}, expected {1:?}")]
    InvalidMapKey(String, &'static str),
    /// A wrong type id found while deserializing.
    // #[error("invalid type id {0}, expected {1:?}")]
    InvalidTypeId(u32, &'static [u32]),
    /// The deserialized type id and the one known from value aren't the same.
    // #[error("type id mismatch: deserialized {0}, but {1} found from value")]
    TypeIdMismatch(u32, u32),
    /// No enum variant id found in the deserializer to continue deserialization.
    // #[error("no enum variant id found in deserializer")]
    NoEnumVariantId,
    /// The deserialized size and the predicted one aren't the same.
    // #[error("size mismatch: deserialized {0}, predicted {1}")]
    SizeMismatch(u32, u32),
}

impl std::fmt::Display for DeErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            DeErrorKind::Msg(ref string) => {
                write!(f, "custom string: {}", string)
            },
            DeErrorKind::BytesLenPrefix254LessThan254(len) => {
                write!(f, "byte sequence has the 0xfe prefix with length less than 254 {}", len)
            },
            DeErrorKind::NonZeroBytesPadding => {
                write!(f, "byte sequence has a padding with a non-zero byte")
            },
            DeErrorKind::UnsupportedSerdeType(ref type_) => {
                write!(f, "{} type is not supported for deserialization", type_)
            },
            DeErrorKind::NotEnoughElements(deserialized_count, expected_count) => {
                write!(f, "not enough elements: have {}, need {}", deserialized_count, expected_count)
            },
            DeErrorKind::InvalidMapKey(ref found_key, expected_key) => {
                write!(f, "invalid map key {:?}, expected {:?}", found_key, expected_key)
            },
            DeErrorKind::InvalidTypeId(found_type_id, valid_type_ids) => {
                write!(f, "invalid type id {}, expected {:?}", found_type_id, valid_type_ids)
            },
            DeErrorKind::TypeIdMismatch(deserialized_type_id, static_type_id) => {
                write!(f, "type id mismatch: deserialized {}, but {} found from value",
                       deserialized_type_id, static_type_id)
            },
            DeErrorKind::NoEnumVariantId => {
                write!(f, "no enum variant id found in deserializer")
            },
            DeErrorKind::SizeMismatch(deserialized_size, static_size_hint) => {
                write!(f, "size mismatch: deserialized {}, predicted {}",
                       deserialized_size, static_size_hint)
            },
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DeSerdeType {
    /// `serde_mt` doesn't support `*_any` hint.
    Any,
    /// Single character type.
    Char,
    /// Option type.
    Option,
    /// Unit `()` type.
    Unit,
    /// `serde_mt` doesn't support `*_ignored_any` hint.
    IgnoredAny,
}

impl std::fmt::Display for DeSerdeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let repr = match *self {
            DeSerdeType::Any => "any",
            DeSerdeType::Char => "char",
            DeSerdeType::Option => "option",
            DeSerdeType::Unit => "unit",
            DeSerdeType::IgnoredAny => "ignored_any",
        };

        f.write_str(repr)
    }
}
