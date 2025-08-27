mod value;
mod whitespace;

use std::io::{self, Read};

use crate::{deserialize::value::parse_value, peek_reader::PeekReader, utils, value::Value};
pub(crate) use value::{
    parse_byte_string, parse_identifier, parse_number, parse_raw_string, parse_string,
};
pub(crate) use whitespace::{parse_sep, skip_whitespace};

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
