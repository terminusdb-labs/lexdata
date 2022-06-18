# Lexdata

This rust library is designed to produce compact lexical
representations of various data types. Lexical representations are
useful because they enable prefix-based indexing strategies which can
support range queries. For instance, they can be used directly in
radix trees or in front-coded dictionaries to obtain all elements
below, above or between some bounds without a scan.

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

## Rationals

Every rational number has a finite, unique, lexically sortable
representation preserving the order relation on rationals *based* on
(but not identical to) a continued fraction expansion.

The continued fraction expansion has integer coefficients given by the
Euclidean algorithm and this representation has very remarkable
properties.

The continued fraction expansion is given as:

```
p/q = a_0 + 1 / (a_1 + (1 / a_n + ...))
```

We can write this short hand as:

```
p/q = [ a_0 / a_1 / ... a_n ]

```

However, this expansion has two equivalent representations. One ending
in a `1`, and one ending with `a_(n-1) - 1`. To obtain a unique
representation we will need to normalise the sequence.

Typeically this is done by chosing one of either ending with a `1`, or
a number larger than `1`. However to make it lexically sortable we
will instead choose to make an *even length sequence*, by either
expansion or contraction of the sequence to its even numbered lenghth.

For instance the continued fraction expansion of `1/3` is given as:

```
1/3 = [0 / 2 / 1] = 0 + 1 / (2 + (1 / 1))
1/3 = [0 / 3] = 0 + 1 / 3
```

We will choose the later representation.

Each prefix of the continued fraction (called a pre-convergent) forms
a unique *parent* relationship with subsequent elements bounding the
values which come after the pre-convergent. However, this bounding
behvaiour alternates in direction Bounding from below, then above,
then below.. .etc.

We will look at the expansion of 3/5 to see how this alternates:

`3/5 = [0/1/1/2]`:

| sequence    | float   | rational | bounded from |
|-------------|---------|----------|--------------|
| `[0]`       |  `0`    |  `0`     | below        |
| `[0/1]`     |  `1`    |  `1/1`   | above        |
| `[0/1/1]`   |  `0.5`  |  `1/2`   | below        |
| `[0/1/1/2]` |  `0.6`  |  `3/5`   | exact        |

This suggests that the lexical ordering must take into account the
alternation present in the sequence of bounds.

We can do this conceptually by altering the sign of the elements in
the sequence, to get a directly sortable representation. For instance,
the above even numbered sequence can be written as:

```
seq(3/5) = [0,-1,1,-2]
```

Using the property that all sequences have a unique parent which
bounds it from the appropriate direction, our representation is
conceptually finished for *positive* rationals.

To get the *negative* rationals, we need to add a sign bit, with `1`
for positive, and `0` for negative, at the beginning of our sequence,
and then flip the signs on the remaining sequence for negatives,
ensuring that larger negatives are sorted "backwards".

```
lexical_seq(3/5)  = [1, 0,-1, 1,-2]
lexical_seq(-3/5) = [0,-0, 1,-1, 2]
```

To represent these as bit sequences we can use tricks similar to our
represenation of sizes for each element of the sequence. Here only the
first bit need give a sign, since each subsequent sign is known from
context. Hence the first segment is identical to our size
representation for bignums, but subsequent byte representations can be
as follows:

```
c = continuation
o = number bits

|cooo oooo|
```

Such that a number is represented by sequences of bytes ignoring the
continuation bit and the continuation bit is set to zero to terminate.

```
128:

      1      000 0000 =  128
      |             |
cont  | termination |
 | ___|__  | _______|
 |/      \ |/       |
|1000 0001|0000 0000|
```

As no coefficient may be zero, we can zero terminate the sequence with
a final zero in our above representation.

For instance, to represent `3/5`, we can write it as:

```
3/5 = [+,0,-1,1,-2]

                -1        1        -2
 sign     cont  |   cont  |   cont  |
 |cont 0   |    |    |    |    |    |      zero termination
 || ___|_  | ___|__  | ___|__  | ___|__   ____|__
 ||/     \ |/      \ |/      \ |/      \ /       \
|1000 0000|1111 1110|0000 0001|1111 1101|0000 0000|
```

Recall, that for negative polarity terms, the continuation bit is also
inverted, such that `0` denotes further bytes, and `1` no further
bytes (preserving the appropriate inverted ordering).

## Floats

TBD

### Dates

TBD
