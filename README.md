# Lexdata

This rust library is designed to produce compact lexical
representations for XSD data types. However other systems, not using
XSD, may find the library useful as several core data types are
implemented, namely:

- [x] Strings
- [x] Large Integers
- [x] Large Floats
- [x] i32
- [x] i64
- [x] f32
- [x] f64
- [ ] Dates
- [ ] Date Time
- [ ] Large Rationals

Lexical representations are useful because they enable prefix-based
indexing strategies which can support range queries. For instance,
they can be used directly in radix trees or in front-coded
dictionaries to obtain all elements below, above or between some
bounds without a scan.

## Aspect

XSD types can generally be viewed as having a base type and a some
constraints, which we call aspects. These aspects restrict what kind
of data can be present. We store the aspect as a byte, in addition to
the type, for all types aside for DateTime, Date, and Boolean which
have no aspect.

In the implementation we encode both Booleans as separate
aspects (true and false) with no data to save space.

## Large Integers

Large integers are marshalled using the Rust Rug library, which wraps
GMP integers (also used in SWI-Prolog).  The functions for conversion
of large integers from GMP are `bigint_to_storage` and
`storage_to_bigint`. These take the GMP representation which is given
by *limbs*, and a *size* which is expressed as the number of limbs.
Each limb an order of magnitude of the integer in scales of
`2^word_size`.

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
pos|  size 5 ______________limbs______________________
  ||   /|\  /                                         \
 |10000101|10001100|11000001|00001100|10000101|00101100|
```
## Large Decimals

Large decimals are a composite of two representations. The first is
Large Integer representation as above, and the second is a lexically
sortable fractional part.

To lexically sort the fractional part we represent values after the
full-stop `.` as a kind of binary coded decimal. In order to be
parsimonious with bits, we actually represent pairs of decimal digits.


| decimal | encoding |
|---------|----------|
|   0     |    1     |
|   00    |    2     |
|   ...   |   ...    |
|   1     |    12    |
|   10    |    13    |
|   ...   |   ...    |
|   9     |   100    |
|   90    |   101    |
|   ...   |   ...    |
|   99    |   111    |


Since 111 < 128, we can fit this as its encoded value in 7 bits. We
also reserve the number 0 to mean "nothing comes after the full stop",
which will always be strictly less than any fraction lexically.

We use the final bit as a continuation bit. So the layout is:

```

   BCD
    |     continuation bit
    |     |
| xxxxxxx c | xxxxxxx c | ...
```

This representation allows us to keep significant digits, for instance
`0.0` will be encoded differently from `0` and from `0.00`, while
retaining appropriate lexical sorting. This is important in scientific
applications where significance should be recorded.

For negative numbers, we use the same bit-flip trick to ensure proper
lexical sorting. We inherit the sign bit from the large integer encoding.

## Date

## Float32 / Float64

## Int32 / Int64

## String

Strings are marshalled as their byte representation.

## DateTime

