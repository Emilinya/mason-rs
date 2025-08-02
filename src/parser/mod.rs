use std::io::{self, Read};

use crate::{
    parser::{value::parse_value, whitespace::skip_whitespace},
    peek_reader::PeekReader,
    utils,
    value::Value,
};

mod value;
mod whitespace;

pub fn parse_document<R: Read>(reader: &mut PeekReader<R>) -> io::Result<Value> {
    skip_whitespace(reader)?;
    let value = parse_value(reader, 100, true)?;
    skip_whitespace(reader)?;
    if let Some(garbage) = reader.peek()? {
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
