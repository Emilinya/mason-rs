use std::io::{self, Read};

use crate::{
    buf_buf_reader::BufBufReader,
    parser::{value::parse_value, whitespace::skip_whitespace},
    utils,
};

mod value;
mod whitespace;

pub use value::Value;

pub fn parse_document<R: Read>(reader: &mut BufBufReader<R>) -> io::Result<Value> {
    skip_whitespace(reader)?;
    let value = parse_value(reader, 100, true)?;
    skip_whitespace(reader)?;
    if let Some(garbage) = reader.peak()? {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Trailing garbage after document: {:?}",
                utils::to_char(garbage)
            ),
        ));
    }
    Ok(value)
}
