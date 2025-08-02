use std::fmt::{self, Write};

use crate::{Value, hex::encode_hex, utils};

/// Serialize a [`Value`] using the given writer.
///
/// # Example
///
/// ```
/// let value_string = r#"vec: [1, true, false, null]"#;
/// let value = mason_rs::from_string(value_string).unwrap();
///
/// let mut writer = String::new();
/// mason_rs::write_value(&value, &mut writer);
/// assert_eq!(writer, value_string);
/// ```
///
/// This is also the function used by `Value`'s display implementation:
///
/// ```
/// let value_string = r#""some bytes": b"This \b \x0e\t is \x7f bytes!""#;
/// let value = mason_rs::from_string(value_string).unwrap();
///
/// assert_eq!(value.to_string(), value_string);
/// ```
pub fn write_value<W: Write>(value: &Value, writer: &mut W) -> fmt::Result {
    write_indented_value(value, writer, "    ", 0)
}

fn write_indented_value<W: Write>(
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
        Value::ByteString(vec) => {
            write!(w, "b\"")?;
            for byte in vec {
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
        Value::String(string) => write!(w, "\"{string}\""),
        Value::Number(num) => write!(w, "{num}"),
        Value::Bool(b) => write!(w, "{b}"),
        Value::Null => write!(w, "null"),
    }
}

fn serialize_key<W: Write>(w: &mut W, key: &str) -> fmt::Result {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return write!(w, "\"\"");
    };

    if (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
    {
        write!(w, "{key}")
    } else {
        write!(w, "\"{key}\"")
    }
}

#[cfg(test)]
mod tests {
    use crate::from_string;

    #[test]
    fn test_to_string() {
        let string = r#"vec: [1, true, false, null]"#;
        assert_eq!(from_string(string).unwrap().to_string(), string);

        let string = r#""nice bytes :)": b"This \b \x0e\t is \x7f bytes!""#;
        assert_eq!(from_string(string).unwrap().to_string(), string);

        let value = from_string(
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
        let same_value = from_string(&value.to_string()).unwrap();
        assert_eq!(value, same_value);
    }
}
