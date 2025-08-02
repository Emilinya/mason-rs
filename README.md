[![Tests](https://github.com/Emilinya/mason-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/Emilinya/mason-rs/actions/workflows/ci.yml)

# mason-rs: MASON implementation for rust

This is a rust implementation of [MASON](https://github.com/mortie/mason),
a JSON-like object notation.

## API

The parsing function has this interface:

```rust
pub fn from_reader(reader: impl Read) -> io::Result<Value>;
```

There are also two alternate functions for common use cases:

```rust
pub fn from_bytes(bytes: &[u8]) -> io::Result<Value>;
pub fn from_string(string: &str) -> io::Result<Value>;
```

A `Value` can be serialized using 
```rust
pub fn write_value<W: Write>(value: &Value, writer: &mut W) -> fmt::Result
```

See [the documentation](https://docs.rs/mason-rs/latest/mason_rs/) for more info.

## Running tests

To run tests, run `cargo test` or `make check`.
This will download the MASON test suite and run it against this implementation.
