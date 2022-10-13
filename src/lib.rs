//! Lexdata
//!
//! This library is designed to produce compact lexical
//! representations of various data types. Lexical representations are
//! useful because they enable prefix-based indexing strategies which
//! can support range queries. For instance, they can be used directly
//! in radix trees or in front-coded dictionaries to obtain all
//! elements below, above or between some bounds without a scan.

use std::str::from_utf8;

use rug::Integer;
//use std::cmp::Ordering;

use bytes::Bytes;

// We need one of these for each strategy used to store our data
enum StorageType {
    String,
    Int32,
    Int64,
    Float32,
    Float64,
    BigInt,
    BigNum,
    Date,
    True,
    False,
}

// Since XSD requires storage of the constraints on the data,
// we do this by adding an aspect tag.
enum Aspect {
    // Core
    String,
    Boolean,
    Decimal,
    Integer,
    // IEEE
    Double,
    Float,
    //Time and Date
    Date,
    Time,
    DateTime,
    DateTimpStamp,
    // Recurring and partial dates
    GYear,
    GMonth,
    GDay,
    GYearMonth,
    GMonthDay,
    Duration,
    YearMonthDuration,
    DayTimeDuration,
    // Limited Range Integer Numbers
    Byte,
    Short,
    Int,
    Long,
    UnsignedByte,
    UnsignedShort,
    UnsignedInt,
    UnsignedLong,
    PositiveInteger,
    NonNegativeInteger,
    // Encoded binary
    HexBinary,
    Base64Binary,
    // Miscellaneous
    AnyURI,
    Language,
    NormalizedString,
    Token,
    NmToken,
    Name,
    NCName,
    NOtation,
    QName,
    ID,
    IdRef,
    Entity,
    // RDF
    XMLLiteral,
    PlainLiteral,
    LangString,
    // RDFS
    Literal,
}

enum Value {
    String(String),
    Int32(u32),
    BigNum(Integer),
    Int64(u64),
    Float32(f32),
    Float64(f64),
    Boolean(bool),
}

pub fn aspect_storage(aspect: Aspect) -> StorageType {
    match aspect {
        Aspect::String => StorageType::String,
        Aspect::Decimal => StorageType::BigNum,
        Aspect::Integer => StorageType::BigInt,
        _ => panic!("Unimplemented aspect"),
    }
}

// 6 bits, leaving 2 more to grow
// surely there is a conversion between enum and ints
pub fn aspect_byte(a: Aspect) -> u8 {
    match a {
        Aspect::String => 0,
        Aspect::Boolean => 1,
        Aspect::Decimal => 2,
        Aspect::Integer => 3,
        Aspect::Double => 4,
        Aspect::Float => 5,
        Aspect::Date => 6,
        Aspect::Time => 7,
        Aspect::DateTime => 8,
        Aspect::DateTimpStamp => 9,
        Aspect::GYear => 10,
        Aspect::GMonth => 11,
        Aspect::GDay => 12,
        Aspect::GYearMonth => 13,
        Aspect::GMonthDay => 14,
        Aspect::Duration => 15,
        Aspect::YearMonthDuration => 16,
        Aspect::DayTimeDuration => 17,
        Aspect::Byte => 18,
        Aspect::Short => 19,
        Aspect::Int => 20,
        Aspect::Long => 21,
        Aspect::UnsignedByte => 22,
        Aspect::UnsignedShort => 23,
        Aspect::UnsignedInt => 24,
        Aspect::UnsignedLong => 25,
        Aspect::PositiveInteger => 26,
        Aspect::NonNegativeInteger => 27,
        Aspect::HexBinary => 28,
        Aspect::Base64Binary => 29,
        Aspect::AnyURI => 30,
        Aspect::Language => 31,
        Aspect::NormalizedString => 32,
        Aspect::Token => 33,
        Aspect::NmToken => 34,
        Aspect::Name => 35,
        Aspect::NCName => 36,
        Aspect::NOtation => 37,
        Aspect::QName => 38,
        Aspect::ID => 39,
        Aspect::IdRef => 40,
        Aspect::Entity => 41,
        Aspect::XMLLiteral => 42,
        Aspect::PlainLiteral => 43,
        Aspect::LangString => 44,
        Aspect::Literal => 45,
        // Simplify storage to one byte for bools
        Aspect::True => 46,
        Aspect::False => 47,
    }
}

pub fn byte_aspect(b: u8) -> Aspect {
    Aspect::String
}

pub fn value_to_storage(v: Value, a: Aspect) -> Result<Bytes> {
    match v {
        Value::String(v) => {
            let storage = aspect_storage(a);
            match a {
                Aspect::String => Bytes::new(v),
                _ => panic!("Unexpected aspect for value"),
            },
        },
        Value::Int32(_) => todo!(),
        Value::BigNum(_) => todo!(),
        Value::Int64(_) => todo!(),
        Value::Float32(_) => todo!(),
        Value::Float64(_) => todo!(),
        Value::Boolean(b) => {
            if a == Aspect::Boolean{
                let mut bytes = Bytes::new();
                match b {
                    true => {
                        let byte = aspect_byte(Aspect::True);
                        bytes.write(byte);
                    },
                    false => {
                        let byte = aspect_byte(Aspect::False);
                        bytes.write(byte);
                    }
                }
                Ok(bytes)
            }else{
                panic!("This should be an error type");
            }
        }
    }
}

pub fn string_from_bytes(bytes) -> Value{
    let string = from_utf8(bytes).expect("The database should not store strings in non utf8 format");
    Value::String(string.to_owned())
}

pub fn storage_to_value(b: Bytes) -> (Value,Aspect) {
    let bytes = b.iter();
    let ty_byte = bytes.next().expect("Empty values are disallowed");
    let aspect = byte_aspect(ty_byte);
    let ty = aspect_storage(aspect);
    if aspect == Aspect::True {
        (Value::Boolean(true),Aspect::Boolean)
    }else if aspect == Aspect::False {
        (Value::Boolean(false),Aspect::Boolean)
    }else{
        match ty {
            StorageType::String => (string_from_bytes(bytes),aspect),
            StorageType::Int32 => todo!(),
            StorageType::Int64 => todo!(),
            StorageType::Float32 => todo!(),
            StorageType::Float64 => todo!(),
            StorageType::BigInt => todo!(),
            StorageType::BigNum => todo!(),
            StorageType::Date => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {
}
