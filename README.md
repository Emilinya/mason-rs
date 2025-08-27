[![Tests](https://github.com/Emilinya/mason-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/Emilinya/mason-rs/actions/workflows/ci.yml)

# mason-rs: MASON implementation for rust

This is a rust implementation of [MASON](https://github.com/mortie/mason),
a JSON-like object notation.

## API

MASON data can be deserialized to a Rust data structure using
```rust
pub fn from_reader<'de, T, R>(reader: R) -> Result<T>
where
    T: Deserialize<'de>,
    R: Read + 'de;
```

There are also two alternate functions for common use cases:
```rust
pub fn from_slice<'de, T: Deserialize<'de>>(bytes: &'de [u8]) -> Result<T>;

pub fn from_str<'de, T: Deserialize<'de>>(string: &'de str) -> Result<T>;
```

A Rust data structure can be serialized to MASON using 
```rust
pub fn to_writer<T: Serialize, W: Write>(value: &T, writer: &mut W) -> Result<()>;
```

See [the documentation](https://docs.rs/mason-rs/latest/mason_rs/) for more info.

## Running tests

To run tests, run `cargo test` or `make check`.
This will download the MASON test suite and run it against this implementation.
