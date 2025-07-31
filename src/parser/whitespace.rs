use std::io::{self, BufRead, Read};

use crate::{buf_buf_reader::BufBufReader, utils};

pub fn skip_whitespace<R: Read>(reader: &mut BufBufReader<R>) -> io::Result<()> {
    loop {
        let Some(next_byte) = reader.peak()? else {
            // We reached EOF, which means there is no more whitespace to skip
            return Ok(());
        };

        match next_byte {
            b' ' | b'\r' | b'\n' | b'\t' => {
                reader.consume(1);
                continue;
            }
            b'/' => reader.consume(1),
            _ => return Ok(()),
        };

        let Some(next_byte) = reader.peak()? else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid whitespace: '/'",
            ));
        };

        match next_byte {
            b'/' => {
                reader.consume(1);
                reader.skip_until(b'\n')?;
            }
            b'*' => {
                reader.consume(1);
                loop {
                    reader.skip_until(b'*')?;
                    let Some(next_byte) = reader.read_byte()? else {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "unclosed block comment",
                        ));
                    };
                    if next_byte == b'/' {
                        break;
                    }
                }
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "invalid whitespace: '/{0}' (char={0:?})",
                        utils::to_char(next_byte)
                    ),
                ));
            }
        };
    }
}

pub fn parse_sep<R: Read>(reader: &mut BufBufReader<R>) -> io::Result<bool> {
    let eof_err = io::Error::new(io::ErrorKind::UnexpectedEof, "got EOF while parsing sep");
    let sep_err = |byte| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid sep: '/{0}' (char={0:?})", utils::to_char(byte)),
        )
    };

    // parse space
    loop {
        let Some(next_byte) = reader.peak()? else {
            return Ok(false);
        };

        match next_byte {
            b' ' | b'\t' => {
                reader.consume(1);
            }
            b'/' => {
                reader.consume(1);
                let Some(byte) = reader.read_byte()? else {
                    return Err(eof_err);
                };
                match byte {
                    b'/' => {
                        // a line comment contains a newline,
                        // and is therefore a valid sep
                        reader.skip_until(b'\n')?;
                        return Ok(true);
                    }
                    b'*' => {}
                    _ => return Err(sep_err(byte)),
                }
                loop {
                    reader.skip_until(b'*')?;
                    let Some(byte) = reader.read_byte()? else {
                        return Err(eof_err);
                    };
                    if byte == b'/' {
                        break;
                    }
                }
            }
            _ => break,
        }
    }

    let Some(next_bytes) = reader.peak2()? else {
        return Ok(false);
    };
    match &next_bytes {
        b"\r\n" => {
            reader.consume(2);
            Ok(true)
        }
        &[b'\n', _] | &[b',', _] => {
            reader.consume(1);
            Ok(true)
        }
        b"//" => {
            reader.skip_until(b'\n')?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skip_whitespace() {
        let data = "\
        Hello!   \r\n\t\r // This is a comment, I can contain anything!\n \
            /* this is a block * comment / / ***  */ !olleH
        ";

        let mut reader = BufBufReader::new(data.as_bytes());
        skip_whitespace(&mut reader).unwrap();

        let mut buf = [0; 6];
        reader.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"Hello!");

        skip_whitespace(&mut reader).unwrap();
        reader.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"!olleH");

        skip_whitespace(&mut reader).unwrap();
    }

    #[test]
    fn test_parse_sep() {
        let data = "\
        First/* */ \t  , \
        Secon // the second number
        Third /* */ \t
        Fourt Fift? \t
        ";

        let mut reader = BufBufReader::new(data.as_bytes());
        skip_whitespace(&mut reader).unwrap();

        let mut buf = [0; 5];
        reader.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"First");

        assert!(parse_sep(&mut reader).unwrap());
        skip_whitespace(&mut reader).unwrap();

        reader.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"Secon");

        assert!(parse_sep(&mut reader).unwrap());
        skip_whitespace(&mut reader).unwrap();

        reader.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"Third");

        assert!(parse_sep(&mut reader).unwrap());
        skip_whitespace(&mut reader).unwrap();

        reader.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"Fourt");

        assert!(!parse_sep(&mut reader).unwrap());

        reader.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"Fift?");

        assert!(parse_sep(&mut reader).unwrap());
    }
}
