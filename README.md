# Lexdata

This rust library is designed to produce compact lexical
representations for XSD data types. However other systems, not using
XSD, may find the library useful as several core data types are
implemented, namely:

- [x] Strings
- [ ] Large Integers
- [ ] i32
- [ ] i64
- [ ] f32
- [ ] f64
- [ ] Dates
- [ ] Date Time
- [ ] Large Floats
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

```rust
```
