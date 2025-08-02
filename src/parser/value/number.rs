use std::io::{self, BufRead, Read};

use crate::{peek_reader::PeekReader, utils};

pub fn parse_number<R: Read>(reader: &mut PeekReader<R>) -> io::Result<f64> {
    let eof_err = io::Error::new(io::ErrorKind::UnexpectedEof, "got EOF while parsing number");

    let mut sign = 1.0;
    match reader.peek()? {
        Some(b'+') => {
            reader.consume(1);
        }
        Some(b'-') => {
            reader.consume(1);
            sign = -1.0;
        }
        None => return Err(eof_err),
        _ => {}
    }

    let Some(first_byte) = reader.peek()? else {
        return Err(eof_err);
    };

    let mut base_data: Option<(f64, Box<dyn Fn(_) -> _>)> = None;
    if first_byte == b'0' {
        let Some([_, second_byte]) = reader.peek2()? else {
            return Ok(0.0);
        };

        base_data = match second_byte {
            b'x' => {
                reader.consume(2);
                let to_number = |byte: u8| {
                    if byte.is_ascii_hexdigit() {
                        if byte <= b'9' {
                            Some(f64::from(byte - b'0'))
                        } else if byte <= b'F' {
                            Some(f64::from(byte - (b'A' - 10)))
                        } else {
                            Some(f64::from(byte - (b'a' - 10)))
                        }
                    } else {
                        None
                    }
                };
                Some((16.0, Box::new(to_number)))
            }
            b'o' => {
                reader.consume(2);
                let to_number = |byte: u8| {
                    if byte >= b'0' && byte - b'0' < 8 {
                        Some(f64::from(byte - b'0'))
                    } else {
                        None
                    }
                };
                Some((8.0, Box::new(to_number)))
            }
            b'b' => {
                reader.consume(2);
                let to_number = |byte: u8| {
                    if matches!(byte, b'0' | b'1') {
                        Some(f64::from(byte - b'0'))
                    } else {
                        None
                    }
                };
                Some((2.0, Box::new(to_number)))
            }
            _ => None,
        };
    } else if matches!(first_byte, b'+' | b'-') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid start to number: two +/- signs",
        ));
    }

    if let Some((base, to_number)) = base_data {
        let mut number_digits = Vec::new();
        {
            let Some(first_byte) = reader.read_byte()? else {
                return Err(eof_err);
            };
            let Some(first_number) = to_number(first_byte) else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("invalid start to number: {:?}", utils::to_char(first_byte)),
                ));
            };
            number_digits.push(first_number);
        }

        loop {
            match reader.peek()? {
                Some(b'\'') => {
                    reader.consume(1);
                    continue;
                }
                Some(other) => {
                    if let Some(number) = to_number(other) {
                        reader.consume(1);
                        number_digits.push(number);
                        continue;
                    } else {
                        break;
                    }
                }
                None => break,
            }
        }

        let mut number = 0.0;
        for (i, value) in number_digits.iter().rev().enumerate() {
            number += value * base.powi(i as i32);
        }
        Ok(sign * number)
    } else {
        let mut number_bytes = Vec::new();
        let mut current_byte = first_byte;
        loop {
            if !current_byte.is_ascii_digit()
                && !matches!(current_byte, b'+' | b'-' | b'.' | b'\'' | b'e' | b'E')
            {
                break;
            }

            reader.consume(1);
            if current_byte == b'\'' {
                if number_bytes.last().is_none_or(|byte| *byte == b'.') {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "number can't start with \"'\"",
                    ));
                }
            } else {
                number_bytes.push(current_byte);
            }

            match reader.peek()? {
                Some(byte) => current_byte = byte,
                None => break,
            }
        }

        // Safety: we know number_bytes contains valid utf8
        let number_str = unsafe { std::str::from_utf8_unchecked(&number_bytes) };

        // a number ending with '.' will be parsed correctly, but is invalid mason >:(
        if number_str.ends_with(".") || number_str.contains(".e") || number_str.contains(".E") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("failed to convert {number_str} to number: numbers can't end with '.'"),
            ));
        }

        let number: f64 = number_str.parse().map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to parse number {number_str:?}: {err}"),
            )
        })?;
        Ok(sign * number)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_number() {
        let data = "1";
        let mut reader = PeekReader::new(data.as_bytes());
        assert_eq!(parse_number(&mut reader).unwrap(), 1.0);

        let data = "0";
        let mut reader = PeekReader::new(data.as_bytes());
        assert_eq!(parse_number(&mut reader).unwrap(), 0.0);

        let data = "++0";
        let mut reader = PeekReader::new(data.as_bytes());
        assert!(parse_number(&mut reader).is_err());

        let data = "-0'6.1'2'45";
        let mut reader = PeekReader::new(data.as_bytes());
        assert_eq!(parse_number(&mut reader).unwrap(), -6.1245);

        let data = "06.'1245";
        let mut reader = PeekReader::new(data.as_bytes());
        assert!(parse_number(&mut reader).is_err());

        let data = "+1.0'12e-2";
        let mut reader = PeekReader::new(data.as_bytes());
        assert_eq!(parse_number(&mut reader).unwrap(), 0.01012);

        let data = "-.2E2";
        let mut reader = PeekReader::new(data.as_bytes());
        assert_eq!(parse_number(&mut reader).unwrap(), -20.0);

        let data = "1.23And then";
        let mut reader = PeekReader::new(data.as_bytes());
        assert_eq!(parse_number(&mut reader).unwrap(), 1.23);
        let mut buf = [0; 8];
        reader.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"And then");
    }

    #[test]
    fn test_parse_base() {
        let data = "-0xa'bc''76";
        let mut reader = PeekReader::new(data.as_bytes());
        assert_eq!(parse_number(&mut reader).unwrap(), -703606.0);

        let data = "0o'110";
        let mut reader = PeekReader::new(data.as_bytes());
        assert!(parse_number(&mut reader).is_err());

        let data = "+0o712";
        let mut reader = PeekReader::new(data.as_bytes());
        assert_eq!(parse_number(&mut reader).unwrap(), 458.0);

        let data = "0b11'00'11'00";
        let mut reader = PeekReader::new(data.as_bytes());
        assert_eq!(parse_number(&mut reader).unwrap(), 204.0);

        let data = "0xff, ...";
        let mut reader = PeekReader::new(data.as_bytes());
        assert_eq!(parse_number(&mut reader).unwrap(), 255.0);
        let mut buf = [0; 5];
        reader.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b", ...");
    }
}
