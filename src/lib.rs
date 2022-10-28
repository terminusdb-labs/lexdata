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
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use bytes::Bytes;
use chrono::{DateTime, NaiveDateTime};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::io::Cursor;

// We need one of these for each strategy used to store our data
#[derive(FromPrimitive, PartialEq, Eq, Debug, Clone, Copy)]
pub enum StorageType {
    String,
    Int32,
    Int64,
    Float32,
    Float64,
    BigInt,
    BigNum,
    DateTime,
}

// Since XSD requires storage of the constraints on the data,
// we do this by adding an aspect tag.

// ONLY add to this list at the bottom. Otherwise values will not be stable.
#[derive(FromPrimitive, PartialEq, Eq, Debug, Clone, Copy)]
pub enum Aspect {
    // Core
    String = 1,
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
    DateTimeStamp,
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
    // Individually encoded Boolean avoid storage element
    False,
    True,
}

#[derive(PartialEq, Debug)]
pub enum Value {
    String(String),
    BigInt(Integer),
    Int32(i32),
    Int64(i64),
    Float32(f32),
    Float64(f64),
    Boolean(bool),
}

pub fn aspect_storage(aspect: Aspect) -> StorageType {
    match aspect {
        Aspect::String
        | Aspect::AnyURI
        | Aspect::Language
        | Aspect::NormalizedString
        | Aspect::Token
        | Aspect::NmToken
        | Aspect::Name
        | Aspect::NCName
        | Aspect::NOtation
        | Aspect::QName
        | Aspect::ID
        | Aspect::IdRef
        | Aspect::Entity => StorageType::String,
        Aspect::Decimal => StorageType::BigNum,
        Aspect::Float => StorageType::Float32,
        Aspect::Double => StorageType::Float64,
        Aspect::Long => StorageType::Int64,
        Aspect::Int | Aspect::Short | Aspect::Byte => StorageType::Int32,
        Aspect::Integer | Aspect::PositiveInteger | Aspect::NonNegativeInteger => {
            StorageType::BigInt
        }
        Aspect::DateTime => StorageType::DateTime,
        _ => panic!("Unimplemented aspect"),
    }
}

fn aspect_byte(a: Aspect) -> u8 {
    a as u8
}

pub fn byte_aspect(b: &u8) -> Aspect {
    FromPrimitive::from_u32(*b as u32).expect("Aspect byte has no aspect representation")
}

#[derive(Debug)]
pub enum LexDataError {
    UnexpectedAspect(String),
    BadFloat32Layout(String),
    BadFloat64Layout(String),
    BadInt32Layout(String),
    BadInt64Layout(String),
    BadDateFormat(String),
}

pub fn value_to_storage(v: Value, a: Aspect) -> Result<Vec<u8>, LexDataError> {
    match v {
        Value::String(s) => {
            if a == Aspect::DateTime {
                date_time_to_storage(s, a)
            } else if a == Aspect::Decimal {
                bignum_to_storage(s, a)
            } else {
                string_to_storage(s, a)
            }
        }
        Value::Int32(i) => int32_to_storage(i, a),
        Value::BigInt(i) => bigint_to_storage(i, a),
        Value::Int64(i) => int64_to_storage(i, a),
        Value::Float32(f) => float32_to_storage(f, a),
        Value::Float64(f) => float64_to_storage(f, a),
        Value::Boolean(b) => {
            if a == Aspect::Boolean {
                let mut buf = Vec::with_capacity(1);
                match b {
                    true => {
                        let byte = aspect_byte(Aspect::True);
                        buf.push(byte);
                        Ok(buf)
                    }
                    false => {
                        let byte = aspect_byte(Aspect::False);
                        buf.push(byte);
                        Ok(buf)
                    }
                }
            } else {
                Err(LexDataError::UnexpectedAspect(format!(
                    "The aspect {a:?} did not match Boolean storage type"
                )))
            }
        }
    }
}

fn string_to_storage(v: String, a: Aspect) -> Result<Vec<u8>, LexDataError> {
    let storage_type = aspect_storage(a);
    if storage_type == StorageType::String {
        let aspect_u8 = aspect_byte(a);
        let string_bytes: &[u8] = &v.into_bytes();
        let mut result = Vec::with_capacity(string_bytes.len() + 1);
        result.push(aspect_u8);
        result.extend(string_bytes);
        Ok(result)
    } else {
        Err(LexDataError::UnexpectedAspect(format!(
            "The aspect {a:?} did not match String value type"
        )))
    }
}

fn date_time_to_storage(s: String, a: Aspect) -> Result<Vec<u8>, LexDataError> {
    match DateTime::parse_from_rfc3339(&s) {
        Ok(date_time) => {
            let timestamp = date_time.timestamp();
            int64_to_storage(timestamp, a)
        }
        Err(parse_error) => Err(LexDataError::BadDateFormat(parse_error.to_string())),
    }
}

fn storage_to_date_time(bytes: &[u8]) -> Result<Value, LexDataError> {
    match storage_to_int64(bytes) {
        Ok(Value::Int64(i)) => {
            let dt = NaiveDateTime::from_timestamp(i, 0);
            Ok(Value::String(dt.format("%Y-%m-%dT%H:%M:%SZ").to_string()))
        }
        Ok(_) => panic!("Imposible return value from storage_to_int64"),
        Err(err) => Err(err),
    }
}

const BYTE_SIGN_MASK: u8 = 0b1000_0000;
fn int32_to_storage(i: i32, a: Aspect) -> Result<Vec<u8>, LexDataError> {
    let storage_type = aspect_storage(a);
    if storage_type == StorageType::Int32 {
        let aspect_u8 = aspect_byte(a);
        let mut wtr = Vec::with_capacity(5);
        wtr.push(aspect_u8);
        wtr.write_i32::<BigEndian>(i).unwrap();
        wtr[1] ^= BYTE_SIGN_MASK;
        Ok(wtr)
    } else {
        Err(LexDataError::UnexpectedAspect(format!(
            "The aspect {a:?} did not match Int32 value type"
        )))
    }
}

fn storage_to_int32(bytes: &[u8]) -> Result<Value, LexDataError> {
    let mut vec = bytes.to_vec();
    vec[0] ^= BYTE_SIGN_MASK;
    let mut rdr = Cursor::new(vec);
    let i_result = rdr.read_i32::<BigEndian>();
    if let Ok(i) = i_result {
        Ok(Value::Int32(i))
    } else {
        Err(LexDataError::BadInt32Layout(
            "Unable to read bytes of float from storage!".to_string(),
        ))
    }
}

fn int64_to_storage(i: i64, a: Aspect) -> Result<Vec<u8>, LexDataError> {
    let storage_type = aspect_storage(a);
    if storage_type == StorageType::Int64 || storage_type == StorageType::DateTime {
        let aspect_u8 = aspect_byte(a);
        let mut wtr = Vec::with_capacity(5);
        wtr.push(aspect_u8);
        wtr.write_i64::<BigEndian>(i).unwrap();
        wtr[1] ^= BYTE_SIGN_MASK;
        Ok(wtr)
    } else {
        Err(LexDataError::UnexpectedAspect(format!(
            "The aspect {a:?} did not match Int64 value type"
        )))
    }
}

fn storage_to_int64(bytes: &[u8]) -> Result<Value, LexDataError> {
    let mut vec = bytes.to_vec();
    vec[0] ^= BYTE_SIGN_MASK;
    let mut rdr = Cursor::new(vec);
    let i_result = rdr.read_i64::<BigEndian>();
    if let Ok(i) = i_result {
        Ok(Value::Int64(i))
    } else {
        Err(LexDataError::BadInt64Layout(
            "Unable to read bytes of float from storage!".to_string(),
        ))
    }
}

const TERMINAL: u8 = 0;
const FIRST_SIGN: u8 = 0b1000_0000u8;
const FIRST_TERMINAL: u8 = 0b0000_0000u8;
const CONTINUATION: u8 = 0b1000_0000u8;
const FIRST_CONTINUATION: u8 = 0b0100_0000u8;
const BASE_MASK: u8 = !CONTINUATION;
const FIRST_MASK: u8 = !(FIRST_SIGN | FIRST_CONTINUATION);
const FIRST_MAX: u8 = FIRST_CONTINUATION;
const NEGATIVE_ZERO: u8 = 0b0111_1111;
// Leave in reverse order for the convenience of the caller
fn size_encode(size: u32) -> Vec<u8> {
    if size == 0 {
        return vec![NEGATIVE_ZERO]; // just the positive sign bit (allows negative zero)
    }
    let mut remainder = size;
    let mut v = vec![];
    let mut last = true;
    while remainder > 0 {
        if remainder >= CONTINUATION as u32 {
            let continued = if last { TERMINAL } else { CONTINUATION };
            let byte = continued | ((remainder & BASE_MASK as u32) as u8);
            v.push(byte);
        } else if remainder >= FIRST_MAX as u32 {
            // special case where we fit in 7 bits but not 6
            // and we need a zero padded initial byte.
            let continued = if last { TERMINAL } else { CONTINUATION };
            let byte = continued | ((remainder & BASE_MASK as u32) as u8);
            v.push(byte);
            let byte = FIRST_SIGN | FIRST_CONTINUATION;
            v.push(byte)
        } else {
            let continued = if last {
                FIRST_TERMINAL
            } else {
                FIRST_CONTINUATION
            };
            let byte = FIRST_SIGN | continued | ((remainder & FIRST_MASK as u32) as u8);
            v.push(byte)
        }
        remainder >>= 7;
        last = false;
    }
    v
}

fn size_decode(v: &[u8]) -> (bool, u32, usize) {
    let mut size: u32 = 0;
    let mut sign = true;
    for (i, elt) in v.iter().enumerate() {
        let vi = *elt as u8;
        if i == 0 {
            sign = vi & FIRST_SIGN != 0;
            let vi = if sign { vi } else { !vi };
            let val = (vi & FIRST_MASK) as u32;
            if vi & FIRST_CONTINUATION == 0 {
                return (sign, val, i + 1);
            } else {
                size += val
            }
        } else {
            let vi = if sign { vi } else { !vi };
            let val = (vi & BASE_MASK) as u32;
            if vi & CONTINUATION == 0 {
                return (sign, size + val, i + 1);
            } else {
                size += val
            }
        }
        size <<= 7;
    }
    (sign, size, v.len())
}

fn bigint_to_storage(bigint: Integer, a: Aspect) -> Result<Vec<u8>, LexDataError> {
    let is_neg = bigint < 0;
    let mut int = bigint.abs();
    let size = int.significant_bits() + 1;
    let num_bytes = (size / 8) + u32::from(size % 8 != 0);
    let size_bytes = size_encode(num_bytes);
    let mut number_vec = Vec::with_capacity(size_bytes.len() + num_bytes as usize + 1);
    for _ in 0..num_bytes {
        let byte = int.to_u8_wrapping();
        number_vec.push(byte);
        int >>= 8;
    }
    number_vec.extend(size_bytes);
    if is_neg {
        for i in 0..number_vec.len() {
            number_vec[i] = !number_vec[i]
        }
    }
    let aspect_u8 = aspect_byte(a);
    number_vec.push(aspect_u8);
    number_vec.reverse();
    Ok(number_vec)
}

fn storage_to_bigint(bytes: &[u8]) -> Result<Value, LexDataError> {
    let (is_pos, size, idx) = size_decode(bytes);
    let mut int = Integer::new();
    if size == 0 {
        return Ok(Value::BigInt(int));
    }
    for (i, b) in bytes[idx..(size + 1) as usize].iter().enumerate() {
        int += if is_pos { *b } else { !*b };
        if i < size as usize - 1 {
            int <<= 8;
        }
    }
    if !is_pos {
        int = -int;
    }
    Ok(Value::BigInt(int))
}

fn encode_fraction(fraction: Option<&str>) -> Vec<u8> {
    if let Some(f) = fraction {
        if f.is_empty() {
            return vec![0x00]; // a "false zero" so we don't represent it at all.
        }
        let len = f.len();
        let size = len / 2 + usize::from(len % 2 != 0);
        let mut bcd = Vec::with_capacity(size);
        for i in 0..size {
            let last = if i * 2 + 2 > len {
                i * 2 + 1
            } else {
                i * 2 + 2
            };
            let two = &f[2 * i..last];
            let mut this_int = centary_decimal_encode(two);
            this_int <<= 1;
            if i != size - 1 {
                this_int |= 1 // add continuation bit.
            }
            bcd.push(this_int)
        }
        bcd
    } else {
        vec![0x00] // a "false zero" so we don't represent no fraction as a fraction
    }
}

fn centary_decimal_encode(s: &str) -> u8 {
    if s.len() == 1 {
        let i = s.parse::<u8>().unwrap();
        i * 11 + 1
    } else {
        let i = s.parse::<u8>().unwrap();
        let o = i / 10 + 1;
        i + o + 1
    }
}

fn centary_decimal_decode(i: u8) -> String {
    let j = i - 1;
    if j % 11 == 0 {
        let num = j / 11;
        format!("{num:}")
    } else {
        let d = j / 11;
        let num = j - d - 1;
        format!("{num:02}")
    }
}

fn decode_fraction(fraction_vec: &[u8]) -> String {
    if fraction_vec == [0x00] {
        "".to_string()
    } else {
        let mut s = String::new();
        for byte in fraction_vec.iter() {
            let num = byte >> 1;
            let res = centary_decimal_decode(num);
            s.push_str(&res);
            if res.len() == 1 || byte & 1 == 0 {
                break;
            }
        }
        s
    }
}

fn bignum_to_storage(bignum: String, a: Aspect) -> Result<Vec<u8>, LexDataError> {
    let storage_type = aspect_storage(a);
    if storage_type == StorageType::BigNum {
        let mut parts = bignum.split('.');
        let bigint = parts.next().unwrap_or(&bignum);
        let fraction = parts.next();
        let integer_part = bigint.parse::<Integer>().unwrap();
        let is_neg = bignum.starts_with('-');
        let prefix = bigint_to_storage(integer_part.clone(), a)?;
        let mut prefix = if integer_part == 0 && is_neg {
            let aspect_u8 = aspect_byte(a);
            vec![aspect_u8, NEGATIVE_ZERO] // negative zero
        } else {
            prefix
        };
        let suffix = if is_neg {
            let mut suffix = encode_fraction(fraction);
            for i in 0..suffix.len() {
                suffix[i] = !suffix[i]
            }
            suffix
        } else {
            encode_fraction(fraction)
        };
        prefix.extend(suffix);
        Ok(prefix)
    } else {
        Err(LexDataError::UnexpectedAspect(format!(
            "The aspect {a:?} did not match Bignum value type"
        )))
    }
}

fn storage_to_bignum(bytes: &[u8]) -> Result<Value, LexDataError> {
    let end = bytes.len();
    let int = storage_to_bigint(&bytes[0..end])?;
    let (is_pos, size, idx) = size_decode(&bytes[0..end]);
    let start = size as usize + idx;
    let fraction_bytes = &bytes[start..end];
    let fraction = if is_pos {
        decode_fraction(fraction_bytes)
    } else {
        let mut fraction_bytes: Vec<u8> = fraction_bytes.to_vec();
        for i in 0..fraction_bytes.len() {
            fraction_bytes[i] = !fraction_bytes[i]
        }
        decode_fraction(&fraction_bytes)
    };
    let int = match int {
        Value::BigInt(int) => int,
        _ => panic!("bigint storage must return bigint"),
    };
    let decimal = if fraction.is_empty() {
        format!("{int:}")
    } else {
        let sign = if int == 0 && !is_pos { "-" } else { "" };
        format!("{sign:}{int:}.{fraction:}")
    };
    Ok(Value::String(decimal))
}

const F32_SIGN_MASK: u32 = 0x8000_0000;
const F32_COMPLEMENT: u32 = 0xffff_ffff;
fn float32_to_storage(f: f32, a: Aspect) -> Result<Vec<u8>, LexDataError> {
    let storage_type = aspect_storage(a);
    if storage_type == StorageType::Float32 {
        let aspect_u8 = aspect_byte(a);
        let g: f32 = if f.to_bits() & F32_SIGN_MASK > 0 {
            f32::from_bits(f.to_bits() ^ F32_COMPLEMENT)
        } else {
            f32::from_bits(f.to_bits() ^ F32_SIGN_MASK)
        };
        let mut wtr = Vec::with_capacity(5);
        wtr.push(aspect_u8);
        wtr.write_f32::<BigEndian>(g).unwrap();
        Ok(wtr)
    } else {
        Err(LexDataError::UnexpectedAspect(format!(
            "The aspect {a:?} did not match Float32 value type"
        )))
    }
}

fn storage_to_float32(bytes: &[u8]) -> Result<Value, LexDataError> {
    let mut rdr = Cursor::new(bytes);
    let f_result = rdr.read_f32::<BigEndian>();
    if let Ok(f) = f_result {
        let g: f32 = if f.to_bits() & F32_SIGN_MASK > 0 {
            f32::from_bits(f.to_bits() ^ F32_SIGN_MASK)
        } else {
            f32::from_bits(f.to_bits() ^ F32_COMPLEMENT)
        };
        Ok(Value::Float32(g))
    } else {
        Err(LexDataError::BadFloat32Layout(
            "Unable to read bytes of float from storage!".to_string(),
        ))
    }
}

const F64_SIGN_MASK: u64 = 0x8000_0000_0000_0000;
const F64_COMPLEMENT: u64 = 0xffff_ffff_ffff_ffff;
fn float64_to_storage(f: f64, a: Aspect) -> Result<Vec<u8>, LexDataError> {
    let storage_type = aspect_storage(a);
    if storage_type == StorageType::Float64 {
        let aspect_u8 = aspect_byte(a);
        let g: f64 = if f.signum() == -1.0 {
            f64::from_bits(f.to_bits() ^ F64_COMPLEMENT)
        } else {
            f64::from_bits(f.to_bits() ^ F64_SIGN_MASK)
        };
        let mut wtr = Vec::with_capacity(5);
        wtr.push(aspect_u8);
        wtr.write_f64::<BigEndian>(g).unwrap();
        Ok(wtr)
    } else {
        Err(LexDataError::UnexpectedAspect(format!(
            "The aspect {a:?} did not match Float64 value type"
        )))
    }
}

fn storage_to_float64(bytes: &[u8]) -> Result<Value, LexDataError> {
    let mut rdr = Cursor::new(bytes);
    let f_result = rdr.read_f64::<BigEndian>();
    if let Ok(f) = f_result {
        let g: f64 = if f.signum() == -1.0 {
            f64::from_bits(f.to_bits() ^ F64_SIGN_MASK)
        } else {
            f64::from_bits(f.to_bits() ^ F64_COMPLEMENT)
        };
        Ok(Value::Float64(g))
    } else {
        Err(LexDataError::BadFloat64Layout(
            "Unable to read bytes of float from storage!".to_string(),
        ))
    }
}

pub fn string_from_bytes(bytes: Bytes) -> Value {
    let string = from_utf8(bytes.as_ref())
        .expect("The database should not store strings in non utf8 format");
    Value::String(string.to_owned())
}

pub fn storage_to_value(bytes: Bytes) -> Result<(Value, Aspect), LexDataError> {
    let aspect_byte = bytes[0];
    let aspect: Aspect = byte_aspect(&aspect_byte);
    if aspect == Aspect::True {
        Ok((Value::Boolean(true), Aspect::Boolean))
    } else if aspect == Aspect::False {
        Ok((Value::Boolean(false), Aspect::Boolean))
    } else {
        let ty = aspect_storage(aspect);
        match ty {
            StorageType::String => Ok((string_from_bytes(bytes.slice(1..)), aspect)),
            StorageType::Int32 => storage_to_int32(&bytes.slice(1..)).map(|r| (r, aspect)),
            StorageType::Int64 => storage_to_int64(&bytes.slice(1..)).map(|r| (r, aspect)),
            StorageType::Float32 => storage_to_float32(&bytes.slice(1..)).map(|r| (r, aspect)),
            StorageType::Float64 => storage_to_float64(&bytes.slice(1..)).map(|r| (r, aspect)),
            StorageType::BigInt => storage_to_bigint(&bytes.slice(1..)).map(|r| (r, aspect)),
            StorageType::BigNum => storage_to_bignum(&bytes.slice(1..)).map(|r| (r, aspect)),
            StorageType::DateTime => storage_to_date_time(&bytes.slice(1..)).map(|r| (r, aspect)),
        }
    }
}

pub fn string_length(bytes: &[u8]) -> usize {
    let mut count = 0_usize;
    for b in bytes.iter() {
        if *b == 0 {
            break;
        }
        count += 1;
    }
    count
}

pub fn storage_size(bytes: Bytes) -> usize {
    let a = byte_aspect(&bytes[0]);
    let storage_type = aspect_storage(a);
    match storage_type {
        StorageType::String => 1 + string_length(&bytes[1..bytes.len()]),
        StorageType::Int32 => 5,
        StorageType::Int64 => 9,
        StorageType::Float32 => 5,
        StorageType::Float64 => 9,
        StorageType::BigInt => {
            let (_, size, idx) = size_decode(&bytes[1..bytes.len()]);
            size as usize + idx + 1
        }
        StorageType::BigNum => {
            let (_, size, idx) = size_decode(&bytes[1..bytes.len()]);
            let offset = size as usize + idx + 1;
            let mut count = 0_usize;
            for i in offset..bytes.len() {
                count += 1;
                if bytes[i] & 1 != 1 {
                    break;
                }
            }
            count + offset
        }
        StorageType::DateTime => 9,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(v: Value, a: Aspect) -> (Value, Aspect) {
        let storage = value_to_storage(v, a);
        let bytes = Bytes::from(storage.unwrap());
        storage_to_value(bytes).unwrap()
    }

    #[test]
    fn string_round_trip() {
        let res = round_trip(Value::String("test".to_string()), Aspect::String);
        let (v, a) = res;
        assert_eq!(v, Value::String("test".to_string()));
        assert_eq!(a, Aspect::String);
        let res = round_trip(Value::String("test".to_string()), Aspect::Token);
        let (v, a) = res;
        assert_eq!(v, Value::String("test".to_string()));
        assert_eq!(a, Aspect::Token);
        let res = round_trip(Value::String("test".to_string()), Aspect::ID);
        let (v, a) = res;
        assert_eq!(v, Value::String("test".to_string()));
        assert_eq!(a, Aspect::ID);
    }

    #[test]
    fn string_ordering() {
        let strings = [
            "entertain",
            "zig",
            "pangolin",
            "penguin",
            "peaches",
            "plums",
            "pears",
            "apple",
            "candy",
        ];
        let mut byte_vec = Vec::with_capacity(strings.len());
        for s in strings.iter() {
            let string: &str = s;
            let storage =
                value_to_storage(Value::String(string.to_string()), Aspect::String).unwrap();
            byte_vec.push(storage)
        }
        byte_vec.sort();
        let mut result_vec = Vec::with_capacity(strings.len());
        for b in byte_vec.iter() {
            let bytes = Bytes::from(b.clone());
            let (value, _) = storage_to_value(bytes).unwrap();
            if let Value::String(v) = value {
                result_vec.push(v)
            } else {
                panic!("This can't happen")
            }
        }
        assert_eq!(
            vec![
                "apple",
                "candy",
                "entertain",
                "pangolin",
                "peaches",
                "pears",
                "penguin",
                "plums",
                "zig"
            ],
            result_vec
        )
    }

    #[test]
    fn boolean_round_trip() {
        let res = round_trip(Value::Boolean(false), Aspect::Boolean);
        assert_eq!((Value::Boolean(false), Aspect::Boolean), res);
        let res = round_trip(Value::Boolean(true), Aspect::Boolean);
        assert_eq!((Value::Boolean(true), Aspect::Boolean), res);
    }

    #[test]
    fn float32_round_trip() {
        let res = round_trip(Value::Float32(-10.87_f32), Aspect::Float);
        assert_eq!((Value::Float32(-10.87_f32), Aspect::Float), res);
    }

    #[test]
    fn float32_ordering() {
        let floats = [
            32.5_f32,
            33432.53_f32,
            f32::INFINITY,
            -132.98701_f32,
            f32::MIN,
            -100.3_f32,
            f32::NEG_INFINITY,
            22.5_f32,
            0_f32,
            f32::MAX,
            0.1_f32,
            -0.1_f32,
        ];
        let mut byte_vec = Vec::with_capacity(floats.len());
        for f in floats.iter() {
            let storage = value_to_storage(Value::Float32(*f), Aspect::Float).unwrap();
            byte_vec.push(storage)
        }
        byte_vec.sort();
        let mut result_vec = Vec::with_capacity(floats.len());
        for b in byte_vec.iter() {
            let bytes = Bytes::from(b.clone());
            let (value, _) = storage_to_value(bytes).unwrap();
            if let Value::Float32(v) = value {
                result_vec.push(v)
            } else {
                panic!("This can't happen")
            }
        }
        assert_eq!(
            vec![
                f32::NEG_INFINITY,
                f32::MIN,
                -132.98701,
                -100.3,
                -0.1,
                0.0,
                0.1,
                22.5,
                32.5,
                33432.53,
                f32::MAX,
                f32::INFINITY,
            ],
            result_vec
        )
    }

    #[test]
    fn float64_ordering() {
        let floats = [
            64.5_f64,
            33464.53_f64,
            f64::INFINITY,
            -164.98701_f64,
            -100.3_f64,
            33464.533432_f64,
            f64::NEG_INFINITY,
            22.5_f64,
            0_f64,
            0.1_f64,
            -0.1_f64,
        ];
        let mut byte_vec = Vec::with_capacity(floats.len());
        for f in floats.iter() {
            let storage = value_to_storage(Value::Float64(*f), Aspect::Double).unwrap();
            byte_vec.push(storage)
        }
        byte_vec.sort();
        let mut result_vec = Vec::with_capacity(floats.len());
        for b in byte_vec.iter() {
            let bytes = Bytes::from(b.clone());
            let (value, _) = storage_to_value(bytes).unwrap();
            if let Value::Float64(v) = value {
                result_vec.push(v)
            } else {
                panic!("This can't happen")
            }
        }
        assert_eq!(
            vec![
                f64::NEG_INFINITY,
                -164.98701,
                -100.3,
                -0.1,
                0.0,
                0.1,
                22.5,
                64.5,
                33464.53,
                33464.533432,
                f64::INFINITY
            ],
            result_vec
        )
    }

    #[test]
    fn int32_ordering() {
        let int = [64_i32, 33464, 164, -100, 22, 0, 1, -1];
        let mut byte_vec = Vec::with_capacity(int.len());
        for f in int.iter() {
            let storage = value_to_storage(Value::Int32(*f), Aspect::Int).unwrap();
            byte_vec.push(storage)
        }
        byte_vec.sort();
        let mut result_vec = Vec::with_capacity(int.len());
        for b in byte_vec.iter() {
            let bytes = Bytes::from(b.clone());
            let (value, _) = storage_to_value(bytes).unwrap();
            if let Value::Int32(v) = value {
                result_vec.push(v)
            } else {
                panic!("This can't happen")
            }
        }
        assert_eq!(vec![-100, -1, 0, 1, 22, 64, 164, 33464], result_vec)
    }

    #[test]
    fn int64_ordering() {
        let int = [64_i64, 33464, 164, -100, -234234322, 22, 0, 1, -1];
        let mut byte_vec = Vec::with_capacity(int.len());
        for f in int.iter() {
            let storage = value_to_storage(Value::Int64(*f), Aspect::Long).unwrap();
            byte_vec.push(storage)
        }
        byte_vec.sort();
        let mut result_vec = Vec::with_capacity(int.len());
        for b in byte_vec.iter() {
            let bytes = Bytes::from(b.clone());
            let (value, _) = storage_to_value(bytes).unwrap();
            if let Value::Int64(v) = value {
                result_vec.push(v)
            } else {
                panic!("This can't happen")
            }
        }
        assert_eq!(
            vec![-234234322, -100, -1, 0, 1, 22, 64, 164, 33464],
            result_vec
        )
    }

    #[test]
    fn size_encode_correct() {
        let bytes = size_encode(1);
        assert_eq!(bytes, vec![129]);

        let (is_pos, size, idx) = size_decode(&bytes);
        assert!(is_pos);
        assert_eq!(size, 1);
        assert_eq!(idx, 1);

        let bytes = size_encode(12);
        assert_eq!(bytes, vec![140]);

        let bytes = size_encode(0);
        assert_eq!(bytes, vec![191]);

        let bytes = size_encode(0);
        let (is_pos, size, idx) = size_decode(&bytes);
        assert!(is_pos);
        assert_eq!(size, 0);
        assert_eq!(idx, 1);
    }

    #[test]
    fn bigint_roundtrip_negative() {
        let minus_one = "-1".parse::<Integer>().unwrap();
        let res = round_trip(Value::BigInt(minus_one.clone()), Aspect::Integer);
        assert_eq!((Value::BigInt(minus_one), Aspect::Integer), res);

        let minus_two_hundred = "-200".parse::<Integer>().unwrap();
        let res = round_trip(Value::BigInt(minus_two_hundred.clone()), Aspect::Integer);
        assert_eq!((Value::BigInt(minus_two_hundred), Aspect::Integer), res);
    }

    #[test]
    fn bigint_ordering() {
        let int_strings = [
            "64",
            "33464",
            "164",
            "-100",
            "256",
            "-923423234234322",
            "22",
            "0",
            "1",
            "-1",
            "234987394839323",
        ];
        let int = int_strings.map(|s| s.parse::<Integer>().unwrap());
        let mut byte_vec = Vec::with_capacity(int.len());
        for i in int.iter() {
            let storage = value_to_storage(Value::BigInt(i.clone()), Aspect::Integer).unwrap();
            byte_vec.push(storage)
        }
        byte_vec.sort();
        let mut result_vec = Vec::with_capacity(int.len());
        for b in byte_vec.iter() {
            let bytes = Bytes::from(b.clone());
            let (value, _) = storage_to_value(bytes).unwrap();
            if let Value::BigInt(v) = value {
                let res = format!("{v:?}");
                result_vec.push(res)
            } else {
                panic!("This can't happen")
            }
        }
        assert_eq!(
            vec![
                "-923423234234322",
                "-100",
                "-1",
                "0",
                "1",
                "22",
                "64",
                "164",
                "256",
                "33464",
                "234987394839323"
            ],
            result_vec
        )
    }

    #[test]
    fn float64_round_trip() {
        let res = round_trip(Value::Float64(-10.87_f64), Aspect::Double);
        assert_eq!((Value::Float64(-10.87_f64), Aspect::Double), res);
    }

    #[test]
    fn fraction_round_trip() {
        let s = "12325";
        let f = encode_fraction(Some(s));
        let original = decode_fraction(&f);
        assert_eq!(s, original);
    }

    #[test]
    fn fraction_round_trip_empty() {
        let s = "";
        let f = encode_fraction(Some(s));
        let original = decode_fraction(&f);
        assert_eq!(s, original);
    }

    #[test]
    fn fraction_trailing_zero() {
        let s = "10";
        let f = encode_fraction(Some(s));
        let original = decode_fraction(&f);
        assert_eq!(s, original);
    }

    #[test]
    fn fraction_mid_one() {
        let s = "101";
        let f = encode_fraction(Some(s));
        let original = decode_fraction(&f);
        assert_eq!(s, original);
    }

    #[test]
    fn fraction_leading_zeros() {
        let s = "001";
        let f = encode_fraction(Some(s));
        let original = decode_fraction(&f);
        assert_eq!(s, original);
    }

    #[test]
    fn centary_encoding() {
        let s = "0";
        let r = centary_decimal_encode(s);
        let d = centary_decimal_decode(r);
        assert_eq!(s, d);

        let s = "00";
        let r = centary_decimal_encode(s);
        let d = centary_decimal_decode(r);
        assert_eq!(s, d);

        let s = "01";
        let r = centary_decimal_encode(s);
        let d = centary_decimal_decode(r);
        assert_eq!(s, d);

        let s = "11";
        let r = centary_decimal_encode(s);
        let d = centary_decimal_decode(r);
        assert_eq!(s, d);

        let s = "22";
        let r = centary_decimal_encode(s);
        let d = centary_decimal_decode(r);
        assert_eq!(s, d);

        let s = "9";
        let r = centary_decimal_encode(s);
        let d = centary_decimal_decode(r);
        assert_eq!(s, d);

        let s = "99";
        let r = centary_decimal_encode(s);
        let d = centary_decimal_decode(r);
        assert_eq!(s, d);
    }

    #[test]
    fn order_fractions() {
        let fractions = vec!["1234", "123", "100", "10000", "32"];
        let mut encodes: Vec<_> = fractions.iter().map(|x| encode_fraction(Some(x))).collect();
        encodes.sort();
        let results: Vec<_> = encodes.iter().map(|x| decode_fraction(x)).collect();
        assert_eq!(vec!["100", "10000", "123", "1234", "32"], results);
    }

    #[test]
    fn negative_zero() {
        let negative_zero = vec![NEGATIVE_ZERO];
        let (is_pos, size, idx) = size_decode(&negative_zero);
        assert!(!is_pos);
        assert_eq!(size, 0);
        assert_eq!(idx, 1);
    }

    #[test]
    fn bignum_round_trip() {
        let fractions = vec![
            "1234.2343",
            "987.23",
            "-0.001",
            "-10.3",
            "-3233.23423",
            "-0.0",
            "0",
            "0.0",
            "0.100",
            "10000.33",
            "0.333",
            "-9871234.1928374",
        ];
        let mut encodes: Vec<_> = fractions
            .iter()
            .map(|x| bignum_to_storage(x.to_string(), Aspect::Decimal).unwrap())
            .collect();
        encodes.sort();
        let results: Vec<_> = encodes
            .iter()
            .map(|x| {
                let res = storage_to_bignum(&x[1..x.len()]).unwrap();
                match res {
                    Value::String(s) => s,
                    _ => panic!("Can't be here"),
                }
            })
            .collect();
        assert_eq!(
            vec![
                "-9871234.1928374",
                "-3233.23423",
                "-10.3",
                "-0.001",
                "-0.0",
                "0",
                "0.0",
                "0.100",
                "0.333",
                "987.23",
                "1234.2343",
                "10000.33"
            ],
            results
        );
    }

    #[test]
    fn date_time_round_trip() {
        let res = round_trip(
            Value::String("2007-03-01T13:00:00Z".to_string()),
            Aspect::DateTime,
        );
        assert_eq!(
            (
                Value::String("2007-03-01T13:00:00Z".to_string()),
                Aspect::DateTime
            ),
            res
        );
    }

    #[test]
    fn date_time_ordering() {
        let dates = vec![
            "2525-03-01T12:00:00Z",
            "2007-03-01T13:00:00Z",
            "1977-05-07T11:30:20Z",
        ];
        let mut date_bytes: Vec<_> = dates
            .iter()
            .map(|x| value_to_storage(Value::String(x.to_string()), Aspect::DateTime).unwrap())
            .collect();
        date_bytes.sort();
        let dates_sorted: Vec<_> = date_bytes
            .iter()
            .map(|x| {
                let res = &*x;
                let bytes: Bytes = Bytes::from(res.clone());
                let (res, _aspect) = storage_to_value(bytes).unwrap();
                match res {
                    Value::String(date) => date,
                    _ => panic!("Didn't work"),
                }
            })
            .collect();
        assert_eq!(
            vec![
                "1977-05-07T11:30:20Z",
                "2007-03-01T13:00:00Z",
                "2525-03-01T12:00:00Z"
            ],
            dates_sorted
        );
    }

    #[test]
    fn self_termination_tests() {
        // we need to know we are not relying on the length of an array anywhere.
        let mut num = value_to_storage(
            Value::BigInt("-3233".parse::<Integer>().unwrap()),
            Aspect::Integer,
        )
        .unwrap();
        let garbage = vec![23, 35, 128];
        num.extend(garbage);
        let (res, _) = storage_to_value(Bytes::from(num)).unwrap();
        match res {
            Value::BigInt(i) => {
                let rendered = format!("{i:?}");
                assert_eq!(rendered, "-3233");
            }
            _ => panic!("This is not good"),
        }

        let mut num =
            value_to_storage(Value::String("-3233.23423".to_string()), Aspect::Decimal).unwrap();
        let garbage = vec![183, 35, 128];
        num.extend(garbage);
        let (res, _) = storage_to_value(Bytes::from(num)).unwrap();
        match res {
            Value::String(n) => {
                assert_eq!("-3233.23423".to_string(), n);
            }
            _ => panic!("This is not good"),
        }
    }
}
