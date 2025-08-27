//! Serialize a Rust data structure into MASON data.

use core::fmt;
use std::fmt::{Display, Write};

use pastey::paste;
use serde::{
    Serialize,
    ser::{self, Error as _, Impossible},
};

use crate::serialize;

use super::error::{Error, Result};

/// A structure for serializing Rust values into MASON.
pub struct Serializer<W: Write> {
    writer: W,
    depth: usize,
}

impl<W: Write> Serializer<W> {
    /// Creates a new MASON serializer.
    pub fn new(writer: W) -> Self {
        Self { writer, depth: 0 }
    }
}

/// Serialize the given data structure as MASON into the I/O stream.
///
/// Serialization guarantees it only feeds valid UTF-8 sequences to the writer.
///
/// # Errors
///
/// Serialization can fail if `T`'s implementation of `Serialize` decides to
/// fail, or if `T` contains a map with non-string keys.
pub fn to_writer<T: Serialize, W: Write>(value: &T, writer: &mut W) -> Result<()> {
    let mut serializer = Serializer::new(writer);
    value.serialize(&mut serializer)?;
    Ok(())
}

/// Serialize the given data structure as a String of MASON.
///
/// # Errors
///
/// Serialization can fail if `T`'s implementation of `Serialize` decides to
/// fail, or if `T` contains a map with non-string keys.
pub fn to_string<T: Serialize>(value: &T) -> Result<String> {
    let mut string = String::new();
    to_writer(value, &mut string)?;
    Ok(string)
}

impl<W: Write> Serializer<W> {
    fn as_compound(&mut self) -> Compound<'_, W> {
        Compound {
            serializer: self,
            first_item: true,
        }
    }

    fn write_whitespace(&mut self, depth: usize) -> fmt::Result {
        if depth == 0 {
            return Ok(());
        }
        write!(self.writer, "{}", "    ".repeat(depth))
    }
}

macro_rules! write_displayed {
    ($type:ty) => {
        paste! {
            fn [<serialize_ $type>](self, v: $type) -> Result<()> {
                Ok(write!(self.writer, "{v}")?)
            }
        }
    };
}

impl<'s, W: Write> ser::Serializer for &'s mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Compound<'s, W>;
    type SerializeTuple = Compound<'s, W>;
    type SerializeTupleStruct = Compound<'s, W>;
    type SerializeTupleVariant = Compound<'s, W>;
    type SerializeMap = Compound<'s, W>;
    type SerializeStruct = Compound<'s, W>;
    type SerializeStructVariant = Compound<'s, W>;

    write_displayed!(bool);

    // MASON does not distinguish between number types.
    write_displayed!(i8);
    write_displayed!(i16);
    write_displayed!(i32);
    // It is possible for an i64 to not be representable as f64. It is not invalid
    // MASON to have a non-f64 number, but most parsers will raise an error when
    // deserializing such a number. It might be better to raise an error when
    // serializing instead, but I will leave it like this for now
    write_displayed!(i64);
    write_displayed!(u8);
    write_displayed!(u16);
    write_displayed!(u32);
    // This has the same issue as serializing i64.
    write_displayed!(u64);
    write_displayed!(f32);
    write_displayed!(f64);

    fn serialize_char(self, v: char) -> Result<()> {
        // just serialize the char as a string
        Ok(serialize::serialize_string(
            &mut self.writer,
            v.encode_utf8(&mut [0; 4]),
        )?)
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        Ok(serialize::serialize_string(&mut self.writer, v)?)
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        Ok(serialize::serialize_bytes(&mut self.writer, v)?)
    }

    // An absent optional is represented as the MASON `null`.
    fn serialize_none(self) -> Result<()> {
        Ok(write!(self.writer, "null")?)
    }

    // A present optional is represented as just the contained value.
    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    // In Serde, unit means an anonymous value containing no data. Map this to
    // MASON as `null`.
    fn serialize_unit(self) -> Result<()> {
        self.serialize_none()
    }

    // Unit struct means a named value containing no data. Again, since there is
    // no data, map this to MASON as `null`.
    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        self.serialize_none()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<()> {
        Ok(serialize::serialize_key(&mut self.writer, variant)?)
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        if self.depth != 0 {
            writeln!(self.writer, "{{\n")?;
        }
        serialize::serialize_key(&mut self.writer, variant)?;
        write!(self.writer, ": ")?;
        value.serialize(&mut *self)?;
        if self.depth != 0 {
            writeln!(self.writer, "\n}}")?;
        }
        Ok(())
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        write!(self.writer, "[")?;
        Ok(self.as_compound())
    }

    // Tuples look just like sequences in MASON.
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(len))
    }

    // Tuple structs look just like sequences in MASON.
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.serialize_seq(Some(len))
    }

    // Tuple variants are represented in MASON as `{ NAME: [DATA...] }`.
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        if self.depth != 0 {
            writeln!(self.writer, "{{")?;
        }
        serialize::serialize_key(&mut self.writer, variant)?;
        write!(self.writer, ": [")?;
        Ok(self.as_compound())
    }

    // Maps are represented in MASON as `{ K: V, K: V, ... }`.
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        if self.depth != 0 {
            writeln!(self.writer, "{{")?;
        }
        Ok(self.as_compound())
    }

    // Structs look just like maps in MASON.
    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        self.serialize_map(Some(len))
    }

    // Struct variants are represented in MASON as `NAME: { K: V, ... }`.
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        if self.depth != 0 {
            write!(self.writer, "{{")?;
        };
        serialize::serialize_key(&mut self.writer, variant)?;
        writeln!(self.writer, ": {{")?;
        self.depth += 1;
        Ok(self.as_compound())
    }
}

// Not public API. Should be pub(crate).
#[doc(hidden)]
pub struct Compound<'s, W: Write> {
    serializer: &'s mut Serializer<W>,
    first_item: bool,
}

impl<W: Write> Compound<'_, W> {
    fn write_unless_first_item(&mut self, string: &'static str) -> fmt::Result {
        if !self.first_item {
            write!(self.serializer.writer, "{}", string)
        } else {
            self.first_item = false;
            Ok(())
        }
    }
}

impl<W: Write> ser::SerializeSeq for Compound<'_, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.write_unless_first_item(", ")?;
        value.serialize(&mut *self.serializer)
    }

    fn end(self) -> Result<()> {
        write!(self.serializer.writer, "]")?;
        Ok(())
    }
}

impl<W: Write> ser::SerializeTuple for Compound<'_, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        <Self as ser::SerializeSeq>::serialize_element(self, value)
    }

    fn end(self) -> Result<()> {
        <Self as ser::SerializeSeq>::end(self)
    }
}

impl<W: Write> ser::SerializeTupleStruct for Compound<'_, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        <Self as ser::SerializeSeq>::serialize_element(self, value)
    }

    fn end(self) -> Result<()> {
        <Self as ser::SerializeSeq>::end(self)
    }
}

impl<W: Write> ser::SerializeTupleVariant for Compound<'_, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        <Self as ser::SerializeSeq>::serialize_element(self, value)
    }

    fn end(self) -> Result<()> {
        if self.serializer.depth > 0 {
            // Here we must close the object in addition to the array
            write!(self.serializer.writer, "]\n}}")?;
        } else {
            write!(self.serializer.writer, "]")?;
        }
        Ok(())
    }
}

impl<W: Write> ser::SerializeMap for Compound<'_, W> {
    type Ok = ();
    type Error = Error;

    // MASON only allows string keys so the implementation below will produce invalid
    // MASON if the key serializes as something other than a string.
    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.write_unless_first_item("\n")?;
        self.serializer.write_whitespace(self.serializer.depth)?;
        key.serialize(KeySerializer {
            ser: self.serializer,
        })
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        write!(self.serializer.writer, ": ")?;
        self.serializer.depth += 1;
        value.serialize(&mut *self.serializer)?;
        self.serializer.depth -= 1;
        Ok(())
    }

    fn end(self) -> Result<()> {
        if self.serializer.depth > 0 {
            write!(self.serializer.writer, "\n}}")?;
        }
        Ok(())
    }
}

impl<W: Write> ser::SerializeStruct for Compound<'_, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        <Self as ser::SerializeMap>::serialize_key(self, key)?;
        <Self as ser::SerializeMap>::serialize_value(self, value)
    }

    fn end(self) -> Result<()> {
        <Self as ser::SerializeMap>::end(self)
    }
}

impl<W: Write> ser::SerializeStructVariant for Compound<'_, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        <Self as ser::SerializeMap>::serialize_key(self, key)?;
        <Self as ser::SerializeMap>::serialize_value(self, value)
    }

    fn end(self) -> Result<()> {
        if self.serializer.depth > 1 {
            // here we must close both the inner and outer object
            write!(self.serializer.writer, "\n}}\n}}")?;
        } else {
            write!(self.serializer.writer, "\n}}")?;
        }
        self.serializer.depth -= 1;
        Ok(())
    }
}

// A serializer which can only serialize valid keys
struct KeySerializer<'s, W: Write> {
    ser: &'s mut Serializer<W>,
}

impl<W: Write> KeySerializer<'_, W> {
    // this function does not enforce that value is not a string, but it is only
    // used for numbers, which are never valid identifiers.
    fn serialize_non_str_displayable(self, value: impl Display) -> fmt::Result {
        write!(self.ser.writer, "\"{value}\"")
    }
}

impl<W: Write> ser::Serializer for KeySerializer<'_, W> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_str(self, value: &str) -> Result<()> {
        Ok(serialize::serialize_key(&mut self.ser.writer, value)?)
    }

    #[inline]
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<()> {
        Ok(serialize::serialize_key(&mut self.ser.writer, variant)?)
    }

    #[inline]
    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    type SerializeSeq = Impossible<(), Error>;
    type SerializeTuple = Impossible<(), Error>;
    type SerializeTupleStruct = Impossible<(), Error>;
    type SerializeTupleVariant = Impossible<(), Error>;
    type SerializeMap = Impossible<(), Error>;
    type SerializeStruct = Impossible<(), Error>;
    type SerializeStructVariant = Impossible<(), Error>;

    // a bool is always a valid key
    fn serialize_bool(self, value: bool) -> Result<()> {
        self.ser.serialize_bool(value)
    }

    fn serialize_i8(self, value: i8) -> Result<()> {
        Ok(self.serialize_non_str_displayable(value)?)
    }

    fn serialize_i16(self, value: i16) -> Result<()> {
        Ok(self.serialize_non_str_displayable(value)?)
    }

    fn serialize_i32(self, value: i32) -> Result<()> {
        Ok(self.serialize_non_str_displayable(value)?)
    }

    fn serialize_i64(self, value: i64) -> Result<()> {
        Ok(self.serialize_non_str_displayable(value)?)
    }

    fn serialize_i128(self, value: i128) -> Result<()> {
        Ok(self.serialize_non_str_displayable(value)?)
    }

    fn serialize_u8(self, value: u8) -> Result<()> {
        Ok(self.serialize_non_str_displayable(value)?)
    }

    fn serialize_u16(self, value: u16) -> Result<()> {
        Ok(self.serialize_non_str_displayable(value)?)
    }

    fn serialize_u32(self, value: u32) -> Result<()> {
        Ok(self.serialize_non_str_displayable(value)?)
    }

    fn serialize_u64(self, value: u64) -> Result<()> {
        Ok(self.serialize_non_str_displayable(value)?)
    }

    fn serialize_u128(self, value: u128) -> Result<()> {
        Ok(self.serialize_non_str_displayable(value)?)
    }

    fn serialize_f32(self, value: f32) -> Result<()> {
        Ok(self.serialize_non_str_displayable(value)?)
    }

    fn serialize_f64(self, value: f64) -> Result<()> {
        Ok(self.serialize_non_str_displayable(value)?)
    }

    fn serialize_char(self, value: char) -> Result<()> {
        self.serialize_str(value.encode_utf8(&mut [0u8; 4]))
    }

    fn serialize_bytes(self, _value: &[u8]) -> Result<()> {
        Err(Error::custom("invalid map key: bytes"))
    }

    fn serialize_unit(self) -> Result<()> {
        self.serialize_none()
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        self.serialize_none()
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::custom("invalid     { key: seq"))
    }

    // null is a valid key
    fn serialize_none(self) -> Result<()> {
        self.ser.serialize_none()
    }

    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Err(Error::custom("invalid map key: seq"))
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Err(Error::custom("invalid map key: tuple"))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Err(Error::custom("invalid map key: tuple struct"))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Err(Error::custom("invalid map key: tuple variant"))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Err(Error::custom("invalid map key: map"))
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        Err(Error::custom("invalid map key: struct"))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Err(Error::custom("invalid map key: struct_variant"))
    }

    fn collect_str<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + Display,
    {
        self.ser.collect_str(value)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn test_struct() {
        #[derive(Serialize)]
        struct Test {
            int: u32,
            seq: Vec<&'static str>,
        }

        let test = Test {
            int: 1,
            seq: vec!["a", "b"],
        };
        let expected = "\
int: 1
seq: [\"a\", \"b\"]";
        assert_eq!(to_string(&test).unwrap(), expected);
    }

    #[test]
    fn test_enum() {
        #[derive(Serialize)]
        enum E {
            Unit,
            Newtype(u32),
            Tuple(u32, u32),
            Struct { a: u32 },
        }

        let u = E::Unit;
        let expected = r#"Unit"#;
        assert_eq!(to_string(&u).unwrap(), expected);

        let n = E::Newtype(1);
        let expected = r#"Newtype: 1"#;
        assert_eq!(to_string(&n).unwrap(), expected);

        let t = E::Tuple(1, 2);
        let expected = r#"Tuple: [1, 2]"#;
        assert_eq!(to_string(&t).unwrap(), expected);

        let s = E::Struct { a: 1 };
        let expected = "\
Struct: {
    a: 1
}";
        assert_eq!(to_string(&s).unwrap(), expected);
    }

    #[test]
    fn test_complicated() {
        #[derive(Serialize)]
        struct Complicated {
            map: HashMap<String, Vec<f32>>,
            bytes: &'static [u8],
            option: Option<String>,
            nothing: (),
        }

        let complicated = Complicated {
            map: HashMap::from([
                ("simple-key".into(), vec![1.0, 999.0, 1.2345]),
                (
                    "a \" \\ \\\" difficult key 🏳️‍⚧️".into(),
                    vec![-1e9, 1.23e3, 3.21e-10],
                ),
            ]),
            bytes: b"Bytes!",
            option: None,
            nothing: (),
        };

        let simple_key = "simple-key: [1, 999, 1.2345]";
        let difficult_key =
            r#""a \" \\ \\\" difficult key 🏳️‍⚧️": [-1000000000, 1230, 0.000000000321]"#;

        // the order of hash map items is random
        let first_key = complicated.map.keys().next().unwrap();
        let map_str = if first_key == "simple-key" {
            format!("{{\n    {}\n    {}\n}}", simple_key, difficult_key)
        } else {
            format!("{{\n    {}\n    {}\n}}", difficult_key, simple_key)
        };

        let expected = "\
map: <map>
bytes: [66, 121, 116, 101, 115, 33]
option: null
nothing: null"
            .replace("<map>", &map_str);
        let got = to_string(&complicated).unwrap();
        if expected != got {
            panic!(
                "assertion `left == right` failed\n left:\n{}\n\nright:\n{}",
                expected, got
            )
        }
    }
}
