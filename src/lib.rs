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
    Date,
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
        _ => panic!("Unimplemented aspect"),
    }
}

fn aspect_byte(a: Aspect) -> u8 {
    eprintln!("Aspect {a:?}");
    let res = a as u8;
    eprintln!("has value {res:?}");
    res
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
}

pub fn value_to_storage(v: Value, a: Aspect) -> Result<Vec<u8>, LexDataError> {
    match v {
        Value::String(s) => string_to_storage(s, a),
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

const BYTE_SIGN_MASK: u8 = 0b1000_0000;
fn int32_to_storage(i: i32, a: Aspect) -> Result<Vec<u8>, LexDataError> {
    let storage_type = aspect_storage(a);
    if storage_type == StorageType::Int32 {
        let aspect_u8 = aspect_byte(a);
        eprintln!("Aspect: {aspect_u8:?}");
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
    if storage_type == StorageType::Int64 {
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
// Leave in reverse order for the convenience of the caller
fn size_encode(size: u32) -> Vec<u8> {
    if size == 0 {
        return vec![0];
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
            sign = !(vi != 0 && vi & FIRST_SIGN == 0);
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
    let storage_type = aspect_storage(a);
    if storage_type == StorageType::BigInt {
        let is_neg = bigint < 0;
        let mut int = bigint.abs();
        eprintln!("Int: {int:?}");
        let size = int.significant_bits() + 1;
        eprintln!("Size: {size:?}");
        let num_bytes = (size / 8) + u32::from(size % 8 != 0);
        eprintln!("num_bytes: {num_bytes:?}");
        let size_bytes = size_encode(num_bytes);
        eprintln!("Size bytes: {size_bytes:?}");
        let mut number_vec = Vec::with_capacity(size_bytes.len() + num_bytes as usize + 1);
        for _ in 0..num_bytes {
            let byte = int.to_u8_wrapping();
            number_vec.push(byte);
            eprintln!("byte: {byte:?}");
            int >>= 8;
        }
        number_vec.extend(size_bytes);
        eprintln!("is_neg: {is_neg:?}");
        if is_neg {
            for i in 0..number_vec.len() {
                eprintln!("number_vec[i] (before): {:?}", number_vec[i]);
                number_vec[i] = !number_vec[i];
                eprintln!("number_vec[i] (after): {:?}", number_vec[i]);
            }
        }
        let aspect_u8 = aspect_byte(a);
        number_vec.push(aspect_u8);
        number_vec.reverse();
        eprintln!("Number vec: {number_vec:?}");
        Ok(number_vec)
    } else {
        Err(LexDataError::UnexpectedAspect(format!(
            "The aspect {a:?} did not match Bignum value type"
        )))
    }
}

fn storage_to_bigint(bytes: &[u8]) -> Result<Value, LexDataError> {
    let (is_pos, size, idx) = size_decode(bytes);
    eprintln!("bytes: {bytes:?}");
    eprintln!("size: {size:?}");
    eprintln!("idx: {idx:?}");
    eprintln!("is_pos: {is_pos:?}");
    let mut int = Integer::new();
    if size == 0 {
        return Ok(Value::BigInt(int));
    }
    for (i, b) in bytes[idx..(size + 1) as usize].iter().enumerate() {
        eprintln!("b: {b:?}");
        int += if is_pos { *b } else { !*b };
        if i < size as usize - 1 {
            int <<= 8;
        }
    }
    if !is_pos {
        int = -int;
    }
    eprintln!("int: {int:?}");
    Ok(Value::BigInt(int))
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
    eprintln!("store bytes (out): {:?}", bytes);
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
    eprintln!("Aspect byte: {aspect_byte:?}");
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
            StorageType::BigNum => todo!(),
            StorageType::Date => todo!(),
        }
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
            eprintln!("Test: {storage:?}");
            byte_vec.push(storage)
        }
        byte_vec.sort();
        let mut result_vec = Vec::with_capacity(int.len());
        for b in byte_vec.iter() {
            eprintln!("Running after sort: {b:?}");
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
            eprintln!("Test: {storage:?}");
            byte_vec.push(storage)
        }
        byte_vec.sort();
        let mut result_vec = Vec::with_capacity(int.len());
        for b in byte_vec.iter() {
            eprintln!("Running after sort: {b:?}");
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
        assert_eq!(bytes, vec![0]);
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
            eprintln!("Test: {storage:?}");
            byte_vec.push(storage)
        }
        byte_vec.sort();
        let mut result_vec = Vec::with_capacity(int.len());
        for b in byte_vec.iter() {
            eprintln!("Running after sort: {b:?}");
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
}
