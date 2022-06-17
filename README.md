# Lexdata

This rust library is designed to produce compact lexical
representations of various data types.

## Roadmap

The roadmap of data types which Lexdata intends to support and which
are already implemented are:

- [x] Large integers (GMP)
- [ ] Large Floats
- [ ] Dates
- [ ] Rationals

## Discussion

More discussion on the strategies used, and how they are implemented
can be found in our discussion in TerminusDB on [Lexical
representation of data
types](https://github.com/terminusdb/terminusdb-store/blob/main/docs/LEXICAL.md).

## Large Integers

The functions for conversion of large integers from GMP are
`convert_mpz_lex` and `convert_lex_mpz`. These take the GMP
representation which is given by *limbs*, and a *size* which is
expressed as the number of limbs.  Each limb an order of magnitude of
the integer in scales of `2^word_size`.

Lexdata stores large integers in two parts. A size part and a number
part.

```
| size | number |
```

A size is represented as follows:

```
s = sign
i = continuation
o = number
|sioooooo|iooooooo|...|iooooooo|
```

Positively signed numbers start will start with a size using a 1
bit. This ensures that they will be lexically after
negatives. Negative numbers are represented as the twos complement, so
that they sort "backwards" with larger magnitudes as smaller.

To extend the size, we use a continuation bit `i` which is `1` if
there are following bytes. This tells us whether there will be another
segment following. Placing it at the front of the byte ensures that
this will sort higher than sizes that terminate at this byte. The
final byte will use a `0`.

```
sign = positive
size = 12:

|10001100|

size = 4095
|11011111|01111111|

```

The number part is reprsented using 8-bit words as *limbs* analogous
to the mechanism of GMP. The use of bytes increases the compactness
for small numbers, which is an advantage.

```
The number part of 12:

|00001100|

The full represenation of 12, inclusing size of 1:

|10000001|00001100|

The full representation of -12, including size of 1:

|01111110|11110011|
```

When marshalling from GMP, care has to be taken to remove the leading
zero-bytes of the representation in order to ensure lexical
comparison.

For example:

```
A one limb GMP number: 604534244652:

|00000000|00000000|00000000|10001100|11000001|00001100|10000101|00101100|

And it's representation in Lexdata:


  terminal size byte
   |
pos|  size5  ______________limbs______________________
  ||   /|\  /                                         \
 |10000101|10001100|11000001|00001100|10000101|00101100|
```
