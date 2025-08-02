mod index;
mod parser;
mod peek_reader;
mod unescape_string;
mod utils;
mod value;

#[cfg(test)]
mod tests;

use std::io::{self, Read};

use crate::peek_reader::PeekReader;

pub use value::Value;

/// Deserialize a `Value` from an I/O stream of MASON.
///
/// The content of the I/O stream is buffered in memory using a [`std::io::BufReader`].
///
/// It is expected that the input stream ends after the deserialized value.
/// If the stream does not end, such as in the case of a persistent socket connection,
/// this function will not return.
///
/// # Example
///
/// ```
/// use std::fs::File;
///
/// fn main() {
/// # }
/// # fn fake_main() {
///     let value = mason_rs::from_reader(File::open("test.mason").unwrap()).unwrap();
///     println!("{:?}", value);
/// }
/// ```
///
/// # Errors
///
/// This function can fail if the I/O stream is not valid MASON, or if any errors were
/// encountered while reading from the stream.
pub fn from_reader(reader: impl Read) -> io::Result<Value> {
    let mut peek_reader = PeekReader::new(reader);
    parser::parse_document(&mut peek_reader)
}

/// Deserialize a `Value` from a slice of MASON bytes.
///
/// # Example
///
/// ```
/// # use mason_rs::Value;
///
/// let data = mason_rs::from_bytes(b"[1.0, true, null]").unwrap();
/// assert_eq!(data, Value::Array(vec![Value::Number(1.0), Value::Bool(true), Value::Null]))
/// ```
///
/// # Errors
///
/// This function can fail if the byte slice is not valid MASON.
pub fn from_bytes(bytes: &[u8]) -> io::Result<Value> {
    from_reader(bytes)
}

/// Deserialize a `Value` from a MASON string.
///
/// # Example
///
/// ```
/// # use mason_rs::Value;
///
/// let data = mason_rs::from_string("[1.0, true, null]").unwrap();
/// assert_eq!(data, Value::Array(vec![Value::Number(1.0), Value::Bool(true), Value::Null]))
///
/// ```
///
/// # Errors
///
/// This function can fail if the string is not valid MASON.
pub fn from_string(string: &str) -> io::Result<Value> {
    from_reader(string.as_bytes())
}
