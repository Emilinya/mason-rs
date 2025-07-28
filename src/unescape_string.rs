use std::borrow::Cow;

use crate::utils;

/// Returns a byte string where all escaped characters in the input byte string
/// are unescaped.
pub fn unescape_string(bytes: &[u8]) -> Result<Cow<[u8]>, String> {
    if !bytes.contains(&b'\\') {
        return Ok(Cow::Borrowed(bytes));
    }
    let mut new_bytes = Vec::with_capacity(bytes.len());

    let mut i = 0;
    while i < bytes.len() {
        let byte = bytes[i];
        if byte == b'\\' && i + 1 < bytes.len() {
            match bytes[i + 1] {
                b'n' => {
                    new_bytes.push(b'\n');
                    i += 2;
                }
                b'r' => {
                    new_bytes.push(b'\r');
                    i += 2;
                }
                b't' => {
                    new_bytes.push(b'\t');
                    i += 2;
                }
                b'b' => {
                    // backspace
                    new_bytes.push(0x8);
                    i += 2;
                }
                b'f' => {
                    // form feed
                    new_bytes.push(0xC);
                    i += 2;
                }
                b'\'' => {
                    new_bytes.push(b'\'');
                    i += 2;
                }
                b'\"' => {
                    new_bytes.push(b'\"');
                    i += 2;
                }
                b'\\' => {
                    new_bytes.push(b'\\');
                    i += 2;
                }
                b'/' => {
                    new_bytes.push(b'/');
                    i += 2;
                }
                b'x' => {
                    if i + 3 >= bytes.len() {
                        return Err("Got incomplete hex escape sequence".to_owned());
                    }

                    let mut byte = [0; 1];
                    match hex::decode_to_slice(&bytes[(i + 2)..=(i + 3)], &mut byte) {
                        Ok(()) => {
                            new_bytes.push(byte[0]);
                            i += 4;
                        }
                        Err(err) => {
                            return Err(format!("Failed to decode \\x hex: {err}"));
                        }
                    }
                }
                b'u' => {
                    if i + 5 >= bytes.len() {
                        return Err("Got incomplete unicode escape sequence".to_owned());
                    }

                    let mut decoded_bytes = [0; 2];
                    match hex::decode_to_slice(&bytes[(i + 2)..=(i + 5)], &mut decoded_bytes) {
                        Ok(()) => {
                            let num = u16::from_be_bytes(decoded_bytes);
                            let Some(c) = char::from_u32(num.into()) else {
                                return Err(format!(
                                    "Got invalid unicode code point \\u{} = {num}",
                                    unsafe {
                                        std::str::from_utf8_unchecked(&bytes[(i + 2)..=(i + 5)])
                                    }
                                ));
                            };

                            let mut c_utf8 = vec![0; c.len_utf8()];
                            c.encode_utf8(&mut c_utf8);
                            new_bytes.append(&mut c_utf8);
                            i += 6;
                        }
                        Err(err) => {
                            return Err(format!("Failed to decode \\u hex: {err}"));
                        }
                    }
                }
                b'U' => {
                    if i + 7 >= bytes.len() {
                        return Err("Got incomplete non-BMP unicode escape sequence".to_owned());
                    }

                    let mut decoded_bytes = [0; 4];
                    match hex::decode_to_slice(&bytes[(i + 2)..=(i + 7)], &mut decoded_bytes[1..4])
                    {
                        Ok(()) => {
                            let num = u32::from_be_bytes(decoded_bytes);
                            let Some(c) = char::from_u32(num) else {
                                return Err(format!(
                                    "Got invalid unicode code point \\U{} = {num}",
                                    unsafe {
                                        std::str::from_utf8_unchecked(&bytes[(i + 2)..=(i + 7)])
                                    }
                                ));
                            };

                            let mut c_utf8 = vec![0; c.len_utf8()];
                            c.encode_utf8(&mut c_utf8);
                            new_bytes.append(&mut c_utf8);
                            i += 8;
                        }
                        Err(err) => {
                            return Err(format!("Failed to decode \\U hex: {err}"));
                        }
                    }
                }
                x => {
                    return Err(format!(
                        "Unexpected escape sequence: \\{}",
                        utils::to_char(x)
                    ));
                }
            }
        } else {
            new_bytes.push(byte);
            i += 1;
        }
    }

    Ok(Cow::Owned(new_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unescape_string() {
        let escaped_string = "this\\t is \\n a string \\x00 with \\\" special \
        \\xf0\\x9f\\x8f\\xb3\\xef\\xb8\\x8f\\xe2\\x80\\x8d\\xe2\\x9a\\xa7\\xef\\xb8\\x8f \
        characters! \\u3061\\U003053";
        let unescaped_string = "this\t is \n a string \0 with \" special 🏳️‍⚧️ characters! ちこ";
        match unescape_string(escaped_string.as_bytes()) {
            Ok(string) => assert_eq!(
                String::from_utf8(string.to_vec()).unwrap(),
                unescaped_string
            ),
            Err(err) => panic!("unescape_string failed: {err}"),
        }

        let simple_string = "this is a string with normal characters!";
        match unescape_string(simple_string.as_bytes()) {
            Ok(string) => assert_eq!(String::from_utf8(string.to_vec()).unwrap(), simple_string),
            Err(err) => panic!("unescape_string failed: {err}"),
        }
    }
}
