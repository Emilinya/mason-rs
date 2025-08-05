use std::{
    collections::HashMap,
    io::{self, BufRead, Read},
};

use super::{Value, parse_value};
use crate::{
    deserialize::whitespace::{parse_sep, skip_whitespace},
    peek_reader::PeekReader,
    utils,
};

pub fn parse_object<R: Read>(
    reader: &mut PeekReader<R>,
    depth: u8,
) -> io::Result<HashMap<String, Value>> {
    // skip opening brackets and whitespace
    if reader.read_byte()? != Some(b'{') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "object does not start with '{'",
        ));
    }
    skip_whitespace(reader)?;

    if reader.peek()? == Some(b'}') {
        reader.consume(1);
        return Ok(HashMap::new());
    }

    let first_key = parse_identifier(reader)?;
    parse_key_value_pairs_after_key(reader, first_key, depth, false)
}

pub fn parse_key_value_pairs_after_key<R: Read>(
    reader: &mut PeekReader<R>,
    first_key: String,
    depth: u8,
    top_level: bool,
) -> io::Result<HashMap<String, Value>> {
    let eof_err = io::Error::new(io::ErrorKind::UnexpectedEof, "got EOF while parsing object");

    // skip colon and whitespace after key
    if reader.read_byte()? != Some(b':') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "key value pairs after key does not start with ':'",
        ));
    }
    skip_whitespace(reader)?;

    let first_value = parse_value(reader, depth - 1, false)?;

    let mut object = HashMap::new();
    object.insert(first_key, first_value);

    loop {
        let valid_sep = parse_sep(reader)?;
        skip_whitespace(reader)?;

        let Some(next_byte) = reader.peek()? else {
            if top_level {
                return Ok(object);
            } else {
                return Err(eof_err);
            }
        };
        if next_byte == b'}' {
            reader.consume(1);
            return Ok(object);
        } else if !valid_sep {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid separator {}", utils::to_char(next_byte)),
            ));
        }

        let (key, value) = parse_key_value_pair(reader, depth)?;
        object.insert(key, value);
    }
}

pub fn parse_identifier<R: Read>(reader: &mut PeekReader<R>) -> io::Result<String> {
    let Some(first_byte) = reader.read_byte()? else {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "Got EOF when parsing key",
        ));
    };

    if first_byte == b'"' {
        let byte_key = utils::read_until_unquote(reader)?;
        String::from_utf8(byte_key).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "got non-utf8 key: {}",
                    String::from_utf8_lossy(err.as_bytes())
                ),
            )
        })
    } else {
        let c = utils::to_char(first_byte);
        if !(c.is_ascii_alphabetic() || c == '_') {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("key identifier starts with invalid char: {c:?}",),
            ));
        }

        let mut key = vec![c];
        loop {
            let byte = reader.peek()?;
            if let Some(byte) = byte {
                if utils::to_char(byte).is_ascii_alphanumeric() || matches!(byte, b'_' | b'-') {
                    reader.consume(1);
                    key.push(utils::to_char(byte));
                    continue;
                }
            }
            return Ok(key.into_iter().collect());
        }
    }
}

fn parse_key_value_pair<R: Read>(
    reader: &mut PeekReader<R>,
    depth: u8,
) -> io::Result<(String, Value)> {
    let key = parse_identifier(reader)?;

    // skip whitespace before colon
    skip_whitespace(reader)?;

    let Some(next_byte) = reader.read_byte()? else {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "Got EOF when parsing key-value pair",
        ));
    };
    if next_byte != b':' {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "got unexpected byte {:?} after key",
                utils::to_char(next_byte)
            ),
        ));
    }

    // skip whitespace after colon
    skip_whitespace(reader)?;

    let value = parse_value(reader, depth - 1, false)?;

    Ok((key, value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_array() {
        let data = "{}";
        let mut reader = PeekReader::new(data.as_bytes());
        assert_eq!(parse_object(&mut reader, 100).unwrap(), HashMap::new());

        let map: HashMap<String, Value> = HashMap::from([
            ("key1".to_owned(), Value::Number(1.0)),
            (" a fancy! key \n".to_owned(), Value::Number(6.0)),
            ("🏳️‍⚧️".to_owned(), Value::Bool(true)),
            ("key4".to_owned(), Value::Null),
        ]);

        let data = "{key1: 1, \" a fancy! key \n\": 6, \"🏳️‍⚧️\": true, key4: null}";
        let mut reader = PeekReader::new(data.as_bytes());
        assert_eq!(parse_object(&mut reader, 100).unwrap(), map);

        let data = "\
        {/* hey :)*/ key1:   \t 1 // so true
        \t \" a fancy! key \n\"  : /*
        so
        here is a comment */ 6 /* hi :)*/ , \t \"🏳️‍⚧️\" \t  : true  ,   
        key4: null
        \t\r\n
        }";
        let mut reader = PeekReader::new(data.as_bytes());
        assert_eq!(parse_object(&mut reader, 100).unwrap(), map);
    }
}
