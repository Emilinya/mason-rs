use std::io::{self, BufRead, Read};

use super::{Value, parse_value};
use crate::{
    buf_buf_reader::BufBufReader,
    parser::whitespace::{parse_sep, skip_whitespace},
    utils,
};

pub fn parse_array<R: Read>(reader: &mut BufBufReader<R>, depth: u8) -> io::Result<Vec<Value>> {
    let eof_err = io::Error::new(io::ErrorKind::UnexpectedEof, "got EOF while parsing array");

    // skip opening brackets and whitespace
    if reader.read_byte()? != Some(b'[') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "array did not start with '['",
        ));
    }
    skip_whitespace(reader)?;

    let mut array = Vec::new();
    loop {
        let Some(next_byte) = reader.peak()? else {
            return Err(eof_err);
        };

        if next_byte == b']' {
            reader.consume(1);
            return Ok(array);
        }
        array.push(parse_value(reader, depth - 1, false)?);

        let valid_sep = parse_sep(reader)?;
        skip_whitespace(reader)?;

        let Some(next_byte) = reader.peak()? else {
            return Err(eof_err);
        };
        if !valid_sep && next_byte != b']' {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid separator {}", utils::to_char(next_byte)),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_array() {
        let data = "[]";
        let mut reader = BufBufReader::new(data.as_bytes());
        assert_eq!(parse_array(&mut reader, 100).unwrap(), vec![]);

        let data = "[1, 6, false, null]";
        let mut reader = BufBufReader::new(data.as_bytes());
        assert_eq!(
            parse_array(&mut reader, 100).unwrap(),
            vec![
                Value::Number(1.0),
                Value::Number(6.0),
                Value::Bool(false),
                Value::Null
            ]
        );

        let data = "\
        [1 // so true
        6 /* hi :)*/ , \t  false  ,   
        null
        \t\r\n
        ]";
        let mut reader = BufBufReader::new(data.as_bytes());
        assert_eq!(
            parse_array(&mut reader, 100).unwrap(),
            vec![
                Value::Number(1.0),
                Value::Number(6.0),
                Value::Bool(false),
                Value::Null
            ]
        );
    }
}
