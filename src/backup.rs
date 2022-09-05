use gmp_mpfr_sys::gmp;
use byteorder::{
    BigEndian,
    WriteBytesExt
};

// give a compile error when not on a 64 bit architecture
#[cfg(not(target_pointer_width = "64"))]
compile_error!("not on a 64 bit architecture");

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

const TERMINAL : u8 = 0;
const FIRST_SIGN : u8 = 0b10000000u8;
const FIRST_TERMINAL : u8 = 0b00000000u8;
const CONTINUATION : u8 = 0b10000000u8;
const FIRST_CONTINUATION : u8 = 0b01000000u8;
const BASE_MASK : u8 = !CONTINUATION;
const FIRST_MASK : u8 = !(FIRST_SIGN | FIRST_CONTINUATION);
const FIRST_MAX : u8 = FIRST_CONTINUATION;

fn size_enc(size : usize) -> Vec<u8> {
    let mut remainder = size;
    let mut v = vec![];
    let mut last = true;
    while remainder > 0 {
        if remainder >= CONTINUATION as usize {
            let continued = if last {TERMINAL} else {CONTINUATION};
            let byte = continued | ((remainder & BASE_MASK as usize) as u8);
            v.push(byte);
        }else if remainder >= FIRST_MAX as usize {
            // special case where we fit in 7 bits but not 6
            // and we need a zero padded initial byte.
            let continued = if last {TERMINAL} else {CONTINUATION};
            let byte = continued | ((remainder & BASE_MASK as usize) as u8);
            v.push(byte);
            let byte = FIRST_SIGN | FIRST_CONTINUATION;
            v.push(byte)
        }else{
            let continued = if last {FIRST_TERMINAL} else {FIRST_CONTINUATION};
            let byte = FIRST_SIGN | continued | ((remainder & FIRST_MASK as usize) as u8);
            v.push(byte)
        }
        remainder = remainder >> 7;
        last = false;
    }
    v.reverse();
    v
}

fn size_dec(v : &[u8]) -> (bool,usize,usize) {
    let mut size : usize = 0;
    let mut sign = true;
    for i in 0..v.len() {
        let vi = v[i] as u8;
        if i == 0 {
            sign  = if vi != 0 && vi & FIRST_SIGN == 0 { false } else { true };
            let vi = if sign { vi } else { !vi };
            let val = (vi & FIRST_MASK) as usize;
            if vi & FIRST_CONTINUATION == 0 {
                return (sign,val,i+1)
            }else{
                size = size + val
            }
        }else{
            let vi = if sign { vi } else { !vi };
            let val = (vi & BASE_MASK) as usize;
            if vi & CONTINUATION == 0 {
                return (sign,size+val,i+1)
            }else{
                size = size + val
            }
        }
        size = size << 7;
    }
    (sign,size,v.len())
}

fn limb_vec(l : gmp_mpfr_sys::gmp::limb_t) -> Vec<u8> {
    let mut wtr = vec![];
    wtr.write_u64::<BigEndian>(l).unwrap();
    wtr
}

fn negate(v : &mut [u8]) -> () {
    for i in 0..v.len() {
        v[i] = !v[i]
    }
}

const BYTES_PER_WORD : usize = 8;

fn convert_mpz_lex(z : *mut gmp_mpfr_sys::gmp::mpz_t) -> Vec<u8> {
    // This should not be hard-coded, but arch dependent
    let sign = unsafe{ gmp::mpz_sgn(z) };
    let size = unsafe{ gmp::mpz_size(z) };
    if size == 0 {
        return vec![FIRST_SIGN]
    }else{
        // we need to get the first limb to get the true size
        // as we need leading zeros to be neglected
        let limb0 = unsafe{ gmp::mpz_getlimbn(z,size as i64 -1) };
        let zeros = limb0.leading_zeros();
        let zero_bytes = zeros as usize / BYTES_PER_WORD;
        let bytes = size * BYTES_PER_WORD - zero_bytes;
        let mut vec = size_enc(bytes);
        let mut limb_vector = limb_vec(limb0)[zero_bytes..BYTES_PER_WORD].to_vec();

        for i in 2..size+1 {
            let j = size as i64 - i as i64;
            let limb_num = unsafe{ gmp::mpz_getlimbn(z,j) };
            let mut limb = limb_vec(limb_num);
            limb_vector.append(&mut limb);
        }
        vec.append(&mut limb_vector);
        if sign == -1 {
            negate(&mut vec);
        }
        return vec
    }
}

// we have to pass in the pointer since we don't know anything about lifetimes.
fn convert_lex_mpz(v : &[u8], z : *mut gmp_mpfr_sys::gmp::mpz_t) -> () {
    let (sign,size,offset) = size_dec(v);
    unsafe { gmp::mpz_init_set_ui(z, 0) };
    if size == 0 { return () };
    for i in offset..size+1 {
        if i != offset {
            unsafe{
                gmp::mpz_mul_2exp(z,z,8);
            }

        }
        let val = if sign {v[i]} else {!v[i]};
        unsafe{
            gmp::mpz_add_ui(z,z,val as u64);
        }
    }
    if !sign {
        unsafe{
            gmp::mpz_mul_si(z,z,-1);
        }
    }
}

fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {

    use core::mem::MaybeUninit;
    use gmp_mpfr_sys::gmp;
    use std::ffi::{CString};
    use core::{slice, str};
    use libc::c_char;

    fn str_from_cstr<'a>(cstr: *const c_char) -> &'a str {
        let s = unsafe { slice::from_raw_parts(cstr as *const u8, libc::strlen(cstr)) };
        str::from_utf8(s).expect("version not utf8")
    }

    fn number_lexical_round_trip<'a>(s1: &'a str) -> () {
        let lex = number_lexical(s1);
        let s2 = lexical_number(&lex);
        assert_eq!(s1,s2);
    }

    fn number_lexical<'a>(s: &'a str) -> Vec<u8> {
        let mut z = MaybeUninit::uninit();
        unsafe{ gmp::mpz_init(z.as_mut_ptr()) };
        let mut z = unsafe { z.assume_init() };
        let cstring = CString::new(s).expect("Cstring::new failed");
        let cstring_ptr = cstring.as_ptr();
        if -1 == unsafe{ gmp::mpz_init_set_str(&mut z, cstring_ptr, 10)}{
            panic!("mpz_init_set_str failed")
        };

        let vec = crate::convert_mpz_lex(&mut z);
        unsafe{gmp::mpz_clear(&mut z)};
        vec
    }

    fn lexical_number(v: &[u8]) -> String {
        let mut z = MaybeUninit::uninit();
        unsafe { gmp::mpz_init(z.as_mut_ptr()) };
        let mut z = unsafe{ z.assume_init() };

        crate::convert_lex_mpz(v,&mut z);

        let str_size = unsafe { gmp::mpz_sizeinbase(&mut z, 10) } + 2;
        let mut c_chars : Vec<i8> = vec![0;str_size];
        let cstring_ptr = c_chars.as_mut_ptr();

        unsafe {
            gmp::mpz_get_str(cstring_ptr, 10, &mut z);
            gmp::mpz_clear(&mut z);
        }

        str_from_cstr(cstring_ptr).to_string()
    }

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
    }

    #[test]
    fn mpz_lex_conversions() {

        let s = "0";
        number_lexical_round_trip(s);

        let s = "1";
        number_lexical_round_trip(s);

        let s = "2";
        number_lexical_round_trip(s);

        let s = "4095";
        number_lexical_round_trip(s);

        let s = "23423498723432";
        number_lexical_round_trip(s);

        let s = "-12";
        number_lexical_round_trip(s);

        let s = "8729234234234234";
        number_lexical_round_trip(s);

        let s = "172923423423423423429";
        number_lexical_round_trip(s);

        let s = "87292342342342342342342346547768087384729384729";
        number_lexical_round_trip(s);

        let s = "9802348729234234223423423432456342342342342346547768087384729384729";
        number_lexical_round_trip(s);

        let s = "-9802348729234234223423423432456342342342342346547768087384729384729";
        number_lexical_round_trip(s);

    }

    #[test]
    fn sort_lexicals() {
        let v = vec!["2342343",
                     "87292342342342342342342346547768087384729384729",
                     "0",
                     "-23423",
                     "-23",
                     "9",
                     "-9802348729234234223423423432456342342342342346547768087384729384729"];
        let mut vecs : Vec<Vec<u8>> = v.iter().map(|s| number_lexical(s)).collect();
        vecs.sort();
        let strs1 : Vec<String> = vecs.iter().map(|l| lexical_number(l)).collect();
        let strs2 = vec!["-9802348729234234223423423432456342342342342346547768087384729384729",
                         "-23423",
                         "-23",
                         "0",
                         "9",
                         "2342343",
                         "87292342342342342342342346547768087384729384729"];
        assert_eq!(strs1,strs2)
    }

}
