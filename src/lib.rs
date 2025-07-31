mod buf_buf_reader;
mod parser;
#[cfg(test)]
mod tests;
mod unescape_string;
mod utils;

use std::io::{self, Read};

use crate::buf_buf_reader::BufBufReader;

pub use parser::Value;

pub fn from_reader(reader: impl Read) -> io::Result<Value> {
    let mut buf_buf_reader = BufBufReader::new(reader);
    parser::parse_document(&mut buf_buf_reader)
}

pub fn from_bytes(bytes: &[u8]) -> io::Result<Value> {
    from_reader(bytes)
}

pub fn from_string(string: &str) -> io::Result<Value> {
    from_reader(string.as_bytes())
}
