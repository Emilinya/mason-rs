use std::{
    collections::HashMap,
    fs::File,
    io::{self, BufRead, BufReader, ErrorKind, Read},
};

use crate::unescape_string::unescape_string;

#[cfg(test)]
mod tests;
mod unescape_string;
pub mod utils;

#[derive(Debug, Clone)]
pub enum Value {
    Object(HashMap<String, Value>),
    Array(Vec<Value>),
    String(String),
    ByteString(Vec<u8>),
    Number(f64),
    Bool(bool),
    Null,
}

pub struct Parser<R: Read> {
    reader: BufReader<R>,
}

impl<R: Read> Parser<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader: BufReader::new(reader),
        }
    }

    pub fn parse(&mut self) -> io::Result<Value> {
        let max_depth = 100;

        let Some(first_byte) = self.read_byte()? else {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "got no data"));
        };
        let Some(next_byte) = self.skip_whitespace(first_byte)? else {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "got no data"));
        };

        // an identifier can be true|false|null, which causes problems
        // this is a hack to avoid that :)
        let mut mistaken_identifier = None;

        // file might contain a value
        let parse_value_result = self.parse_value(next_byte, max_depth);
        match parse_value_result {
            Ok((value, next_byte)) => {
                if let Some(next_byte) = next_byte
                    && let Some(next_byte) = self.skip_whitespace(next_byte)?
                {
                    if next_byte != b':' {
                        return Err(io::Error::new(
                            ErrorKind::InvalidData,
                            "reader did not reach EOF after first value",
                        ));
                    } else {
                        // oops, that was an identifier, not a value, oopsie :p
                        mistaken_identifier = Some(value);
                    }
                } else {
                    return Ok(value);
                }
            }
            Err(err) => {
                // if next byte is not the start of an identifier, there was actually something wrong
                // with the value
                if !(utils::to_char(next_byte).is_ascii_alphabetic() || next_byte == b'_') {
                    return Err(err);
                }
            }
        }

        // file might also contain an object without curly brackets
        let mut object = HashMap::new();
        let mut first_byte = next_byte;

        // yay, hacks!
        if let Some(value) = mistaken_identifier {
            let key = match value {
                Value::Bool(true) => "true".to_owned(),
                Value::Bool(false) => "false".to_owned(),
                Value::Null => "null".to_owned(),
                _ => {
                    return Err(io::Error::new(
                        ErrorKind::InvalidData,
                        format!("got invalid mistaken identifier {value:?}"),
                    ));
                }
            };

            // skip whitespace after colon
            let Some(next_byte) = self.read_byte()? else {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "got EOF before value",
                ));
            };
            let Some(next_byte) = self.skip_whitespace(next_byte)? else {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "got EOF before value",
                ));
            };

            let (value, next_byte) = self.parse_value(next_byte, max_depth)?;
            object.insert(key, value);

            let Some(next_byte) = next_byte else {
                return Ok(Value::Object(object));
            };

            let (valid_sep, next_byte) = self.parse_sep(next_byte)?;
            let Some(next_byte) = next_byte else {
                return Ok(Value::Object(object));
            };
            let Some(next_byte) = self.skip_whitespace(next_byte)? else {
                return Ok(Value::Object(object));
            };

            if !valid_sep {
                return Err(io::Error::new(ErrorKind::InvalidData, "invalid sep"));
            } else {
                first_byte = next_byte;
            }
        }

        loop {
            let (key, value, next_byte) = self.parse_key_value_pair(first_byte, max_depth)?;
            object.insert(key, value);

            let Some(next_byte) = next_byte else {
                return Ok(Value::Object(object));
            };

            let (valid_sep, next_byte) = self.parse_sep(next_byte)?;
            let Some(next_byte) = next_byte else {
                return Ok(Value::Object(object));
            };
            let Some(next_byte) = self.skip_whitespace(next_byte)? else {
                return Ok(Value::Object(object));
            };

            if !valid_sep {
                return Err(io::Error::new(ErrorKind::InvalidData, "invalid sep"));
            } else {
                first_byte = next_byte;
            }
        }
    }

    fn parse_key_value_pair(
        &mut self,
        first_byte: u8,
        depth: u8,
    ) -> io::Result<(String, Value, Option<u8>)> {
        let (key, next_byte) = self.parse_key(first_byte)?;
        let Some(next_byte) = next_byte else {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "got EOF after key",
            ));
        };
        if !matches!(next_byte, b':' | b' ' | b'\r' | b'\n' | b'\t' | b'/') {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "got unexpected byte {:?} after key",
                    utils::to_char(next_byte)
                ),
            ));
        }

        // skip whitespace before colon
        let Some(next_byte) = self.skip_whitespace(next_byte)? else {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "got EOF after key",
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
        let Some(next_byte) = self.read_byte()? else {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "got EOF before value",
            ));
        };
        let Some(next_byte) = self.skip_whitespace(next_byte)? else {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "got EOF before value",
            ));
        };

        let (value, next_byte) = self.parse_value(next_byte, depth)?;

        Ok((key, value, next_byte))
    }

    fn parse_value(&mut self, first_byte: u8, depth: u8) -> io::Result<(Value, Option<u8>)> {
        if depth == 0 {
            return Err(io::Error::new(ErrorKind::InvalidData, "max depth exceeded"));
        }

        let eof_err = io::Error::new(io::ErrorKind::UnexpectedEof, "got EOF while parsing value");
        let unexpected_after_err = |byte, c: char| {
            io::Error::new(
                ErrorKind::InvalidData,
                format!(
                    "got unexpected character {:?} after {} when parsing value",
                    utils::to_char(byte),
                    c
                ),
            )
        };

        let mut next_byte = None;

        let value = match first_byte {
            b'{' => Value::Object(self.parse_object(depth - 1)?),
            b'[' => Value::Array(self.parse_array(depth - 1)?),
            b'"' => {
                let value_bytes = utils::read_until_unquote(&mut self.reader)?;
                let unescaped_bytes = unescape_string(&value_bytes)
                    .map_err(|err| io::Error::new(ErrorKind::InvalidData, err))?
                    .to_vec();
                let value = String::from_utf8(unescaped_bytes).map_err(|err| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "got non-utf8 string: {} (bytes: {:?})",
                            String::from_utf8_lossy(err.as_bytes()),
                            err.as_bytes(),
                        ),
                    )
                })?;
                Value::String(value)
            }
            b'r' => {
                let mut pattern = Vec::new();
                loop {
                    let Some(byte) = self.read_byte()? else {
                        return Err(eof_err);
                    };
                    match byte {
                        b'#' => pattern.push(byte),
                        b'"' => {
                            pattern.push(byte);
                            break;
                        }
                        _ => return Err(unexpected_after_err(byte, 'r')),
                    }
                }
                pattern.reverse();
                let value_bytes = utils::read_until_pattern(&mut self.reader, &pattern)?;
                let value = String::from_utf8(value_bytes).map_err(|err| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "got non-utf8 string: {} (bytes: {:?})",
                            String::from_utf8_lossy(err.as_bytes()),
                            err.as_bytes(),
                        ),
                    )
                })?;
                Value::String(value)
            }
            b'b' => {
                match self.read_byte()? {
                    None => return Err(eof_err),
                    Some(b'"') => {}
                    Some(byte) => return Err(unexpected_after_err(byte, 'b')),
                }

                let value_bytes = utils::read_until_unquote(&mut self.reader)?;
                if let Some(non_ascii) = value_bytes.iter().find(|byte| !byte.is_ascii()) {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "got non-ascii value in byte string: {:?} (bytes: {:?})",
                            utils::to_char(*non_ascii),
                            value_bytes,
                        ),
                    ));
                }
                let unescaped_bytes = unescape_string(&value_bytes)
                    .map_err(|err| io::Error::new(ErrorKind::InvalidData, err))?
                    .to_vec();

                Value::ByteString(unescaped_bytes)
            }
            b't' => {
                utils::read_pattern(&mut self.reader, b"rue")?;
                Value::Bool(true)
            }
            b'f' => {
                utils::read_pattern(&mut self.reader, b"alse")?;
                Value::Bool(true)
            }
            b'n' => {
                utils::read_pattern(&mut self.reader, b"ull")?;
                Value::Bool(true)
            }
            _ => {
                let (number, byte) = self.parse_number(first_byte)?;
                if let Some(byte) = byte {
                    next_byte = Some(byte);
                }
                Value::Number(number)
            }
        };

        if next_byte.is_none() {
            next_byte = self.read_byte()?;
        }
        Ok((value, next_byte))
    }

    fn parse_number(&mut self, first_byte: u8) -> io::Result<(f64, Option<u8>)> {
        let eof_err = io::Error::new(io::ErrorKind::UnexpectedEof, "got EOF while parsing number");

        let mut number_bytes = Vec::new();
        let mut next_byte = None;

        if first_byte == b'0' {
            let Some(byte) = self.read_byte()? else {
                return Err(eof_err);
            };

            let result = match byte {
                b'x' => {
                    let (mut values, exponent) = (Vec::new(), 16f64);
                    loop {
                        let Some(byte) = self.read_byte()? else {
                            return Err(eof_err);
                        };
                        if byte == b'\'' {
                            continue;
                        } else if byte.is_ascii_hexdigit() {
                            if byte <= b'9' {
                                values.push(f64::from(byte - b'0'));
                            } else if byte <= b'F' {
                                values.push(f64::from(byte - (b'A' - 9)));
                            } else {
                                values.push(f64::from(byte - (b'a' - 9)));
                            }
                            continue;
                        } else {
                            next_byte = Some(byte);
                            break;
                        }
                    }
                    Some((values, exponent))
                }
                b'o' => {
                    let (mut values, exponent) = (Vec::new(), 8f64);
                    loop {
                        let Some(byte) = self.read_byte()? else {
                            return Err(eof_err);
                        };
                        if byte == b'\'' {
                            continue;
                        } else if byte >= b'0' && byte - b'0' < 8 {
                            values.push(f64::from(byte - b'0'));
                            continue;
                        } else {
                            next_byte = Some(byte);
                            break;
                        }
                    }
                    Some((values, exponent))
                }
                b'b' => {
                    let (mut values, exponent) = (Vec::new(), 2f64);
                    loop {
                        let Some(byte) = self.read_byte()? else {
                            return Err(eof_err);
                        };
                        if byte == b'\'' {
                            continue;
                        } else if matches!(byte, b'0' | b'1') {
                            values.push(f64::from(byte - b'0'));
                            continue;
                        } else {
                            next_byte = Some(byte);
                            break;
                        }
                    }
                    Some((values, exponent))
                }
                _ => {
                    number_bytes.push(b'0');
                    next_byte = Some(byte);
                    None
                }
            };

            if let Some((values, exponent)) = result {
                let mut number = 0.0;
                for (i, value) in values.iter().rev().enumerate() {
                    number += value * exponent.powi(i as i32);
                }
                return Ok((number, next_byte));
            }
        }

        let mut byte = if let Some(byte) = next_byte {
            byte
        } else {
            first_byte
        };
        loop {
            if byte.is_ascii_digit() || matches!(byte, b'+' | b'-' | b'.' | b'\'' | b'e' | b'E') {
                number_bytes.push(byte);
            } else {
                next_byte = Some(byte);
                break;
            }

            let Some(new_byte) = self.read_byte()? else {
                break;
            };
            byte = new_byte;
        }

        // Safety: we know number_bytes contains valid utf8
        let number_str = unsafe { std::str::from_utf8_unchecked(&number_bytes) };

        // mason supports quotes in numbers, but rust can't parse them
        let split: Vec<_> = number_str.split('.').collect();
        for string in &split {
            if string.starts_with('\'') {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    format!(
                        "failed to convert {number_str} to number: number can't start with \"'\"."
                    ),
                ));
            }
        }
        let number_str: String = split.join(".").replace('\'', "");

        // a number ending with '.' will be parsed correctly, but is invalid mason >:(
        if number_str.ends_with(".") || number_str.contains(".e") || number_str.contains(".E") {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                format!("failed to convert {number_str} to number: numbers can't end with ."),
            ));
        }

        let number = number_str.parse().map_err(|err| {
            io::Error::new(
                ErrorKind::InvalidData,
                format!("Failed to parse number {number_str:?}: {err}"),
            )
        })?;
        Ok((number, next_byte))
    }

    fn parse_key(&mut self, first_byte: u8) -> io::Result<(String, Option<u8>)> {
        if first_byte == b'"' {
            let byte_key = utils::read_until_unquote(&mut self.reader)?;
            let key = String::from_utf8(byte_key).map_err(|err| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "got non-utf8 key: {}",
                        String::from_utf8_lossy(err.as_bytes())
                    ),
                )
            })?;

            Ok((key, self.read_byte()?))
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
                let byte = self.read_byte()?;
                if let Some(byte) = byte
                    && (utils::to_char(byte).is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
                {
                    key.push(utils::to_char(byte));
                    continue;
                }
                return Ok((key.into_iter().collect(), byte));
            }
        }
    }

    fn parse_object(&mut self, depth: u8) -> io::Result<HashMap<String, Value>> {
        let eof_err = io::Error::new(io::ErrorKind::UnexpectedEof, "got EOF while parsing object");

        let Some(next_byte) = self.read_byte()? else {
            return Err(eof_err);
        };
        let Some(next_byte) = self.skip_whitespace(next_byte)? else {
            return Err(eof_err);
        };

        if next_byte == b'}' {
            return Ok(HashMap::new());
        }

        let mut hash_map = HashMap::new();
        let mut first_byte = next_byte;
        loop {
            let (key, value, next_byte) = self.parse_key_value_pair(first_byte, depth)?;
            let Some(next_byte) = next_byte else {
                return Err(eof_err);
            };
            hash_map.insert(key, value);

            let (valid_sep, next_byte) = self.parse_sep(next_byte)?;
            let Some(next_byte) = next_byte else {
                return Err(eof_err);
            };
            let Some(next_byte) = self.skip_whitespace(next_byte)? else {
                return Err(eof_err);
            };

            if !valid_sep && !next_byte == b'}' {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("invalid separator {}", utils::to_char(next_byte)),
                ));
            }

            if next_byte == b'}' {
                break;
            } else {
                first_byte = next_byte;
            }
        }

        Ok(hash_map)
    }

    fn parse_array(&mut self, depth: u8) -> io::Result<Vec<Value>> {
        let eof_err = io::Error::new(io::ErrorKind::UnexpectedEof, "got EOF while parsing array");

        let Some(next_byte) = self.read_byte()? else {
            return Err(eof_err);
        };
        let Some(next_byte) = self.skip_whitespace(next_byte)? else {
            return Err(eof_err);
        };

        if next_byte == b']' {
            return Ok(Vec::new());
        }

        let mut array = Vec::new();
        let mut first_byte = next_byte;
        loop {
            let (value, next_byte) = self.parse_value(first_byte, depth)?;
            let Some(next_byte) = next_byte else {
                return Err(eof_err);
            };
            array.push(value);

            let (valid_sep, next_byte) = self.parse_sep(next_byte)?;
            let Some(next_byte) = next_byte else {
                return Err(eof_err);
            };
            let Some(next_byte) = self.skip_whitespace(next_byte)? else {
                return Err(eof_err);
            };

            if !valid_sep && next_byte != b']' {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("invalid separator {}", utils::to_char(next_byte)),
                ));
            }

            if next_byte == b']' {
                break;
            } else {
                first_byte = next_byte;
            }
        }

        Ok(array)
    }

    fn parse_sep(&mut self, first_byte: u8) -> io::Result<(bool, Option<u8>)> {
        let eof_err = io::Error::new(io::ErrorKind::UnexpectedEof, "got EOF while parsing sep");
        let comment_err = |byte| {
            io::Error::new(
                ErrorKind::InvalidData,
                format!("invalid sep: '/{0}' (char={0:?})", utils::to_char(byte)),
            )
        };

        // parse space
        let mut next_byte = first_byte;
        loop {
            match next_byte {
                b' ' | b'\t' => {}
                b'/' => {
                    let Some(byte) = self.read_byte()? else {
                        return Err(eof_err);
                    };
                    match byte {
                        b'/' => {
                            // a line commend contains a newline,
                            // and is therefore a valid sep
                            self.reader.skip_until(b'\n')?;
                            return Ok((true, self.read_byte()?));
                        }
                        b'*' => {}
                        _ => return Err(comment_err(byte)),
                    }
                    loop {
                        self.reader.skip_until(b'*')?;
                        let Some(byte) = self.read_byte()? else {
                            return Err(eof_err);
                        };
                        if byte == b'/' {
                            break;
                        }
                    }
                }
                _ => break,
            }

            match self.read_byte()? {
                Some(byte) => next_byte = byte,
                None => return Ok((false, None)),
            }
        }

        match next_byte {
            b'\r' => {
                let Some(next_byte) = self.read_byte()? else {
                    return Err(eof_err);
                };
                if next_byte != b'\n' {
                    return Ok((false, Some(next_byte)));
                }
            }
            b'\n' | b',' => {}
            b'/' => {
                let Some(byte) = self.read_byte()? else {
                    return Err(eof_err);
                };
                if byte != b'/' {
                    return Err(comment_err(byte));
                }
                self.reader.skip_until(b'\n')?;
            }
            _ => {
                return Ok((false, Some(next_byte)));
            }
        }

        Ok((true, self.read_byte()?))
    }

    fn skip_whitespace(&mut self, first_byte: u8) -> io::Result<Option<u8>> {
        let mut next_byte = first_byte;
        loop {
            match next_byte {
                b' ' | b'\r' | b'\n' | b'\t' => {
                    match self.read_byte()? {
                        Some(byte) => next_byte = byte,
                        None => return Ok(None),
                    };
                    continue;
                }
                b'/' => {}
                _ => return Ok(Some(next_byte)),
            };

            let Some(byte) = self.read_byte()? else {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "invalid whitespace: '/'",
                ));
            };
            match byte {
                b'/' => {
                    self.reader.skip_until(b'\n')?;
                }
                b'*' => loop {
                    self.reader.skip_until(b'*')?;
                    let Some(next_byte) = self.read_byte()? else {
                        return Err(io::Error::new(
                            ErrorKind::InvalidData,
                            "unclosed block comment",
                        ));
                    };
                    if next_byte == b'/' {
                        break;
                    }
                },
                _ => {
                    let c = utils::to_char(byte);
                    return Err(io::Error::new(
                        ErrorKind::InvalidData,
                        format!("invalid whitespace: '/{c}' (char={c:?})"),
                    ));
                }
            };

            match self.read_byte()? {
                Some(byte) => next_byte = byte,
                None => return Ok(None),
            }
        }
    }

    fn read_byte(&mut self) -> io::Result<Option<u8>> {
        utils::read_byte(&mut self.reader)
    }
}

fn main() {
    let mut parser = Parser::new(File::open("test.mason").unwrap());
    let value = parser.parse().unwrap();
    eprintln!("{value:#?}");
}
