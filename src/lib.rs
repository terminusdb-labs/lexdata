//! Lexdata
//!
//! This library is designed to produce compact lexical
//! representations of various data types. Lexical representations are
//! useful because they enable prefix-based indexing strategies which
//! can support range queries. For instance, they can be used directly
//! in radix trees or in front-coded dictionaries to obtain all
//! elements below, above or between some bounds without a scan.

use rug::{Integer,Complete};
use std::cmp::Ordering;

/// size_encode takes a vector representing the
/// order of a number and converts it into a vector of bytes
///
/// Format of the first byte is:
///
/// 1cxxxxxx
///
/// Format of each following byte is:
///
/// cxxxxxxx
///
/// where c is the chain bit. If zero, then
/// we are terminal, otherwise there is another size
/// to come.
///
/// The first bit is a sign bit, we will twos complement the
/// entire result and therby end up with a negative, so it
/// is always 1 here.

struct ByteLayout {
    terminal : u8,
    first_sign : u8,
    first_terminal : u8,
    continuation : u8,
    first_continuation : u8,
    base_mask : u8,
    first_mask : u8,
    first_max : u8,
}

fn size_enc(size : usize, bl : ByteLayout) -> Vec<u8> {
    let mut remainder = size;
    let mut v = vec![];
    let mut last = true;
    while remainder > 0 {
        if remainder >= bl.continuation as usize {
            let continued = if last {bl.terminal} else {bl.continuation};
            let byte = continued | ((remainder & bl.base_mask as usize) as u8);
            v.push(byte);
        }else if remainder >= bl.first_max as usize {
            // special case where we fit in 7 bits but not 6
            // and we need a zero padded initial byte.
            let continued = if last {bl.terminal} else {bl.continuation};
            let byte = continued | ((remainder & bl.base_mask as usize) as u8);
            v.push(byte);
            let byte = bl.first_sign | bl.first_continuation;
            v.push(byte)
        }else{
            let continued = if last {bl.first_terminal} else {bl.first_continuation};
            let byte = bl.first_sign | continued | ((remainder & bl.first_mask as usize) as u8);
            v.push(byte)
        }
        remainder = remainder >> 7;
        last = false;
    }
    v.reverse();
    v
}

fn size_dec(v : &[u8], bl : ByteLayout) -> (bool,usize,usize) {
    let mut size : usize = 0;
    let mut sign = true;
    for i in 0..v.len() {
        let vi = v[i] as u8;
        if i == 0 {
            sign  = if vi != 0 && vi & bl.first_sign == 0 { false } else { true };
            let vi = if sign { vi } else { !vi };
            let val = (vi & bl.first_mask) as usize;
            if vi & bl.first_continuation == 0 {
                return (sign,val,i+1)
            }else{
                size = size + val
            }
        }else{
            let vi = if sign { vi } else { !vi };
            let val = (vi & bl.base_mask) as usize;
            if vi & bl.continuation == 0 {
                return (sign,size+val,i+1)
            }else{
                size = size + val
            }
        }
        size = size << 7;
    }
    (sign,size,v.len())
}

fn negate(v : &mut [u8]) -> () {
    for i in 0..v.len() {
        v[i] = !v[i]
    }
}

const INTEGER_LAYOUT : ByteLayout<'static> = ByteLayout{
    first_sign : 0b10000000u8,
    first_terminal : 0b00000000u8,
    first_continuation: 0b01000000u8,
    terminal : 0b00000000u8,
    continuation : 0b10000000u8,
    base_mask: (! 0b10000000u8),
    first_mask : (! 0b11000000u8),
    first_max : 0b01000000u8,
}

pub fn integer_to_lexical(mut z : Integer) -> Vec<u8> {
    let negative = match z.cmp0() { Ordering::Less => true, _ => false };
    let size = z.significant_bits();
    if size == 0 {
        return vec![INTEGER_LAYOUT.first_sign]
    }else{
        let half_bytes = ((size / 7) + 1) as usize;
        let mut vec = size_enc(half_bytes,INTEGER_LAYOUT);

        // +1 is for the zero terminator
        let mut words = vec![0; half_bytes+1];
        for i in 0..half_bytes {
            // Shift left and add 1 to get rid of zeros
            let word = (((z.clone() & INTEGER_LAYOUT.base_mask).to_u32().unwrap() as u8) << 1) + 0b1;
            z = z >> 7;
            words[half_bytes - i - 1] = word
        }
        vec.append(&mut words);
        if negative {
            negate(&mut vec);
        }
        let full_length = vec.len();
        vec[full_length-1] = 0;
        return vec
    }
}

pub fn lexical_to_integer(v : &[u8]) -> Integer {
    let (sign,size,offset) = size_dec(v);
    let mut z = Integer::from(0);
    if size == 0 { return z };
    for i in offset..size+1 {
        if i != offset {
            z = z * Integer::u_pow_u(2,7).complete()
        }
        let val = v[i] >> 1; // remove added low bit to avoid zeros
        let sval = if sign {val} else {!val};
        z = z + sval
    }
    if !sign {
        z = z * -1
    }
    return z
}

const NEGATIVE_QUIET_NAN : u8 = 0b0001;
const NEGATIVE_SIGNALING_NAN : u8 = 0b0010;
const NEGATIVE_INFINITY : u8 = 0b0011;
const NEGATIVE_NUMBERS : u8 = 0b0100;
const NEGATIVE_SUBNORMAL_NUMBERS : u8 = 0b0101;
const NEGATIVE_ZERO : u8 = 0b0110;
const POSITIVE_ZERO : u8 = 0b0111;
const POSITIVE_SUBNORMAL_NUMBERS : u8 = 0b1000;
const POSITIVE_NUMBERS : u8 = 0b1001;
const POSITIVE_INFINITY : u8 = 0b1010;
const POSITIVE_SIGNALING_NAN : u8 = 0b1011;
const POSITIVE_QUIET_NAN : u8 = 0b1100;

const FLOAT_LAYOUT : ByteLayout<'static> = ByteLayout{
    first_sign : 0b10000000u8,
    first_terminal : 0b00000000u8,
    first_continuation: 0b01000000u8,
    terminal : 0b00000000u8,
    continuation : 0b10000000u8,
    base_mask: (! 0b10000000u8),
    first_mask : (! 0b11000000u8),
    first_max : 0b01000000u8,
}

pub fn f64_to_lexical(f : f64) -> Vec<u8> {
    let negative = match f.cmp0() { Ordering::Less => true, _ => false };
    let size_in_bits = z.significant_bits();
    let 
    match c {
        
    }
}

pub fn lexical_to_f64(f : f32) -> Vec<u8> {
}
// mpq_to_lexical
// lexical_to_mpq

// f64_to_lexical
// lexical_to_f64

// bin_to_lexical
// lexical_to_bin

#[cfg(test)]
mod tests {
    use rug::{Assign, Integer};
    use crate::{
        integer_to_lexical,
        lexical_to_integer
    };

    #[test]
    fn find_bytes_and_pad() {
        let size = 4095; // [0b00001111u8,0b11111111u8];
        let enc = crate::size_enc(size);
        assert_eq!(enc, vec![0b11011111u8, 0b01111111u8]);

        let size = 72057594037927935;
        let enc = crate::size_enc(size);
        assert_eq!(enc, vec![0b11000000u8, 0b11111111u8, 0b11111111u8,
                             0b11111111u8, 0b11111111u8, 0b11111111u8,
                             0b11111111u8, 0b11111111u8, 0b01111111u8 ])

    }

    #[test]
    fn in_and_out() {
        let size = 4095; // [0b00001111u8,0b11111111u8];
        assert_eq!(size, crate::size_dec(&crate::size_enc(size)).1);

        // just a random number
        let size = 23423423;
        assert_eq!(size, crate::size_dec(&crate::size_enc(size)).1);

        // boundary case for overflow spillover
        let size = 72057594037927935;
        assert_eq!(size, crate::size_dec(&crate::size_enc(size)).1);

        let size = 1;
        assert_eq!(size, crate::size_dec(&crate::size_enc(size)).1);

        let size = 0;
        assert_eq!(size, crate::size_dec(&crate::size_enc(size)).1);

    }

    fn int_lex_int(s : &str) -> String {
        let mut int = Integer::new();
        int.assign(Integer::parse(s).unwrap());
        let vec = integer_to_lexical(int);
        let res = lexical_to_integer(&vec);
        return res.to_string_radix(10);
    }

    #[test]
    fn round_trip() {
        let decimal = "10";
        let s = int_lex_int(&decimal);
        assert_eq!(s,decimal);

        let decimal = "129";
        let s = int_lex_int(decimal);
        assert_eq!(s,decimal);

        let decimal = "98765432109876543210";
        let s = int_lex_int(decimal);
        assert_eq!(s,decimal)
    }

    #[test]
    fn sort_lexicals() {
        let v = vec!["2342343",
                     "87292342342342342342342346547768087384729384729",
                     "0",
                     "-23423",
                     "10",
                     "91",
                     "-23",
                     "1",
                     "9",
                     "-9802348729234234223423423432456342342342342346547768087384729384729"];

        let mut vecs : Vec<Vec<u8>> = v.iter().map(|s| integer_to_lexical(s.parse::<Integer>().unwrap())).collect();
        vecs.sort();
        let res : Vec<String> = vecs.iter().map(|s| lexical_to_integer(s).to_string_radix(10)).collect();
        assert_eq!(res,
                   ["-44329827649690278750120583238331105073228301966427397130579212454247",
                    "-4187393",
                    "-233",
                    "0",
                    "1",
                    "9",
                    "10",
                    "91",
                    "2342343",
                    "87292342342342342342342346547768087384729384729"])
    }
}
