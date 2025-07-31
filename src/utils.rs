use std::io::{self, BufRead, Read};

use crate::buf_buf_reader::BufBufReader;

pub fn to_char(byte: u8) -> char {
    // Safety: all u8's are valid chars
    unsafe { char::from_u32_unchecked(byte.into()) }
}

/// Read from `reader` until a not-escaped quote is reached. The final quote is read
/// but not returned.
pub fn read_until_unquote<R: Read>(reader: &mut BufBufReader<R>) -> io::Result<Vec<u8>> {
    let mut value = Vec::new();
    let mut buff = Vec::new();
    loop {
        reader.read_until(b'"', &mut buff)?;
        if buff.len() >= 2 && buff[buff.len() - 2] == b'\\' {
            // quote is escaped, continue
            value.append(&mut buff);
        } else {
            // quote is not escaped, remove it from buff and break
            if buff.pop().is_none_or(|end| end != b'"') {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "found no unquote",
                ));
            }
            value.append(&mut buff);
            break;
        }
    }

    Ok(value)
}

/// Read from `reader` until a specified pattern (string of bytes) is reached. The pattern is read
/// but not returned.
pub fn read_until_pattern<R: Read>(
    reader: &mut BufBufReader<R>,
    pattern: &[u8],
) -> io::Result<Vec<u8>> {
    if pattern.is_empty() {
        return Ok(Vec::new());
    }

    let mut value = Vec::new();
    let mut buff = Vec::new();
    loop {
        reader.read_until(pattern[0], &mut buff)?;
        if buff.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "pattern not found",
            ));
        }
        value.append(&mut buff);

        let mut correct_chars = 1;
        while correct_chars < pattern.len() {
            let Some(next) = reader.read_byte()? else {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "pattern not found",
                ));
            };
            value.push(next);

            if next == pattern[correct_chars] {
                correct_chars += 1;
            } else {
                break;
            }
        }

        if correct_chars == pattern.len() {
            for _ in 0..pattern.len() {
                value.pop();
            }
            return Ok(value);
        }
    }
}
