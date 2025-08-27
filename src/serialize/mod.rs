use std::fmt::{self, Write};

use crate::{Value, hex::encode_hex, utils};

pub fn write_indented_value<W: Write>(
    value: &Value,
    w: &mut W,
    indentation: &str,
    indentation_level: usize,
) -> fmt::Result {
    match value {
        Value::Object(hash_map) => {
            if indentation_level != 0 {
                writeln!(w, "{{\n")?;
            }
            for (i, (key, value)) in hash_map.iter().enumerate() {
                write!(w, "{}", indentation.repeat(indentation_level))?;
                serialize_key(w, key)?;
                write!(w, ": ")?;
                write_indented_value(value, w, indentation, indentation_level + 1)?;
                if i != hash_map.len() - 1 {
                    writeln!(w)?;
                }
            }
            if indentation_level != 0 {
                write!(w, "\n{}}}", indentation.repeat(indentation_level - 1))
            } else {
                Ok(())
            }
        }
        Value::Array(vec) => {
            write!(w, "[")?;
            for (i, value) in vec.iter().enumerate() {
                write_indented_value(value, w, indentation, indentation_level)?;
                if i != vec.len() - 1 {
                    write!(w, ", ")?;
                }
            }
            write!(w, "]")
        }
        Value::ByteString(vec) => serialize_bytes(w, vec),
        Value::String(string) => serialize_string(w, string),
        Value::Number(num) => write!(w, "{num}"),
        Value::Bool(b) => write!(w, "{b}"),
        Value::Null => write!(w, "null"),
    }
}

pub(crate) fn serialize_bytes<W: Write>(w: &mut W, bytes: &[u8]) -> fmt::Result {
    write!(w, "b\"")?;
    for byte in bytes {
        if *byte > 31 && *byte < 127 {
            // byte is normal, add it as char
            write!(w, "{}", utils::to_char(*byte))?;
        } else {
            match byte {
                b'\t' => write!(w, "\\t")?,
                b'\r' => write!(w, "\\r")?,
                b'\n' => write!(w, "\\n")?,
                0x8 => write!(w, "\\b")?,
                0xC => write!(w, "\\f")?,
                _ => {
                    let [first, second] = encode_hex(*byte);
                    write!(w, "\\x{}{}", utils::to_char(first), utils::to_char(second))?;
                }
            }
        }
    }
    write!(w, "\"")
}

// We must escape quotes and backslashes
pub(crate) fn serialize_string<W: Write>(w: &mut W, string: &str) -> fmt::Result {
    if !string.contains(['"', '\\']) {
        Ok(write!(w, "\"{string}\"")?)
    } else {
        let mut v = string;
        write!(w, "\"")?;
        while let Some(index) = v.find(['"', '\\']) {
            write!(w, "{}\\{}", &v[..index], &v[index..=index])?;
            v = &v[(index + 1)..];
        }
        write!(w, "{v}\"")?;
        Ok(())
    }
}

pub(crate) fn serialize_key<W: Write>(w: &mut W, key: &str) -> fmt::Result {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return write!(w, "\"\"");
    };

    if (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
    {
        write!(w, "{key}")
    } else {
        serialize_string(w, key)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::Value;

    #[test]
    fn test_to_string() {
        let string = r#"vec: [1, true, false, null]"#;
        assert_eq!(Value::from_str(string).unwrap().to_string(), string);

        let string = r#""nice bytes :)": b"This \b \x0e\t is \x7f bytes!""#;
        assert_eq!(Value::from_str(string).unwrap().to_string(), string);

        let value = Value::from_str(
            r#"{
    thing: [1, true, false, null]
    thang: {
        a: "hey",
        "a difficult key 😮": "hoy"
    }
    _other: {a: {b: {c: {d: 3.1415}, o: false}, o: true}, o: null},
    some-bytes_: b"This \b \x0e\t is \x7F bytes!"
}"#,
        )
        .unwrap();
        let same_value = Value::from_str(&value.to_string()).unwrap();
        assert_eq!(value, same_value);
    }
}
