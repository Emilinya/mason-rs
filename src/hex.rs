use crate::utils;

/// Decode a pair of hex digits into a number.
pub fn decode_hex(hex: [u8; 2]) -> Result<u8, ()> {
    let (high, low) = (hex_to_num(hex[0])?, hex_to_num(hex[1])?);
    Ok(low | (high << 4))
}

/// Encode a number into a pair of hex digits.
pub fn encode_hex(num: u8) -> [u8; 2] {
    let (high, low) = (num >> 4, num & 0b0000_1111);
    [
        num_to_hex(high).expect("high should be >16"),
        num_to_hex(low).expect("low should be >16"),
    ]
}

/// Convert a hex digit into a number.
fn hex_to_num(hex: u8) -> Result<u8, ()> {
    match utils::to_char(hex) {
        '0'..='9' => Ok(hex - b'0'),
        'A'..='F' => Ok(hex - (b'A' - 10)),
        'a'..='f' => Ok(hex - (b'a' - 10)),
        _ => Err(()),
    }
}

/// Convert number into a hex digit.
fn num_to_hex(num: u8) -> Result<u8, ()> {
    match num {
        0..=9 => Ok(b'0' + num),
        10..=15 => Ok(b'a' + (num - 10)),
        _ => Err(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_hex() {
        assert_eq!(decode_hex([b'f', b'F']).unwrap(), 255);
        assert_eq!(decode_hex([b'1', b'2']).unwrap(), 18);
        assert_eq!(decode_hex([b'c', b'3']).unwrap(), 195);
        assert!(decode_hex([b'!', b'?']).is_err());
    }

    #[test]
    fn test_encode_hex() {
        assert_eq!(encode_hex(255), [b'f', b'f']);
        assert_eq!(encode_hex(18), [b'1', b'2']);
        assert_eq!(encode_hex(195), [b'c', b'3']);
    }

    #[test]
    fn test_hex_to_num() {
        assert_eq!(hex_to_num(b'C').unwrap(), 12);
        assert_eq!(hex_to_num(b'a').unwrap(), 10);
        assert_eq!(hex_to_num(b'7').unwrap(), 7);
        assert!(hex_to_num(b'!').is_err());
    }

    #[test]
    fn test_num_to_hex() {
        assert_eq!(num_to_hex(12).unwrap(), b'c');
        assert_eq!(num_to_hex(10).unwrap(), b'a');
        assert_eq!(num_to_hex(7).unwrap(), b'7');
        assert!(num_to_hex(100).is_err());
    }
}
