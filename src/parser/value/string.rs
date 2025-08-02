use std::io::{self, Read};

use crate::{peek_reader::PeekReader, unescape_string::unescape_string, utils};

pub fn parse_string<R: Read>(reader: &mut PeekReader<R>) -> io::Result<String> {
    if reader.read_byte()? != Some(b'"') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "string did not start with '\"'",
        ));
    }

    let value_bytes = utils::read_until_unquote(reader)?;
    let unescaped_bytes = unescape_string(&value_bytes)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?
        .to_vec();

    String::from_utf8(unescaped_bytes).map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "got non-utf8 string: {} (bytes: {:?})",
                String::from_utf8_lossy(err.as_bytes()),
                err.as_bytes(),
            ),
        )
    })
}

pub fn parse_raw_string<R: Read>(reader: &mut PeekReader<R>) -> io::Result<String> {
    if reader.read_byte()? != Some(b'r') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "string did not start with 'r'",
        ));
    }

    let mut pattern = Vec::new();
    loop {
        let Some(byte) = reader.read_byte()? else {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "got EOF while parsing raw string",
            ));
        };
        match byte {
            b'#' => pattern.push(byte),
            b'"' => {
                pattern.push(byte);
                break;
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "got invalid char {:?} in beginning of raw string",
                        utils::to_char(byte)
                    ),
                ));
            }
        }
    }
    pattern.reverse();
    let value_bytes = utils::read_until_pattern(reader, &pattern)?;

    String::from_utf8(value_bytes).map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "got non-utf8 string: {} (bytes: {:?})",
                String::from_utf8_lossy(err.as_bytes()),
                err.as_bytes(),
            ),
        )
    })
}

pub fn parse_byte_string<R: Read>(reader: &mut PeekReader<R>) -> io::Result<Vec<u8>> {
    if (reader.read_byte()?, reader.read_byte()?) != (Some(b'b'), Some(b'"')) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "string did not start with 'b\"'",
        ));
    }

    let value_bytes = utils::read_until_unquote(reader)?;
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
    unescape_string(&value_bytes)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
        .map(|bytes| bytes.into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_string() {
        let data = r#""This \" string \n is \"\" a string""#;
        let mut reader = PeekReader::new(data.as_bytes());
        assert_eq!(
            parse_string(&mut reader).unwrap(),
            "This \" string \n is \"\" a string"
        );

        let data = r#""I am missing an end quote :("#;
        let mut reader = PeekReader::new(data.as_bytes());
        assert!(parse_string(&mut reader).is_err());
    }

    #[test]
    fn test_parse_byte_string() {
        let data = r#"b"This \" string \n is \"\" a string""#;
        let mut reader = PeekReader::new(data.as_bytes());
        assert_eq!(
            parse_byte_string(&mut reader).unwrap(),
            b"This \" string \n is \"\" a string"
        );

        let data = r#"b"I contain an emoji 😮""#;
        let mut reader = PeekReader::new(data.as_bytes());
        assert!(parse_byte_string(&mut reader).is_err());

        let data = r#"b"I am missing an end quote :("#;
        let mut reader = PeekReader::new(data.as_bytes());
        assert!(parse_string(&mut reader).is_err());
    }

    #[test]
    fn test_parse_raw_string() {
        let data = r###"r##"This "string" can fit so many #"quotes"# :)"##"###;
        let mut reader = PeekReader::new(data.as_bytes());
        assert_eq!(
            parse_raw_string(&mut reader).unwrap(),
            "This \"string\" can fit so many #\"quotes\"# :)"
        );

        let data = r##"r#"I am not closed properly ""##;
        let mut reader = PeekReader::new(data.as_bytes());
        assert!(parse_raw_string(&mut reader).is_err());
    }
}
