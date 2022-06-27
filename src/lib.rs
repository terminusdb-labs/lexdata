use rug;
use byteorder::{
    BigEndian,
    WriteBytesExt
};

//! Lexdata
//!
//! This library is designed to produce compact lexical
//! representations of various data types. Lexical representations are
//! useful because they enable prefix-based indexing strategies which
//! can support range queries. For instance, they can be used directly
//! in radix trees or in front-coded dictionaries to obtain all
//! elements below, above or between some bounds without a scan.

// mpz_to_lexical
// lexical_to_mpz
fn mpz_to_lexical(z : *mut gmp::mpz_t) -> Vec<u8> {
    //
    panic!("This is not yet implemented")
}
// mpq_to_lexical
// lexical_to_mpq

// f64_to_lexical
// lexical_to_f64

// bin_to_lexical
// lexical_to_bin
