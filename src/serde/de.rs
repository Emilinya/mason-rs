//! Deserialize MASON data to a Rust data structure.

use std::io::{self, BufRead, Read};

use pastey::paste;
use serde::Deserialize;
use serde::de::value::StringDeserializer;
use serde::de::{
    self, DeserializeSeed, EnumAccess, Error as _, IntoDeserializer, MapAccess, SeqAccess,
    Unexpected, VariantAccess, Visitor,
};

use crate::peek_reader::PeekReader;
use crate::{deserialize, utils};

use super::error::{Error, Result};

/// A structure that deserializes MASON into Rust values.
pub struct Deserializer<R: Read> {
    reader: PeekReader<R>,
    depth: usize,
}

impl<R: Read> Deserializer<R> {
    /// Creates a MASON deserializer from an `io::Read`.
    ///
    /// Reader-based deserializers do not support deserializing borrowed types
    /// like `&str`, since the `std::io::Read` trait has no non-copying methods
    /// -- everything it does involves copying bytes out of the data source.
    pub fn from_reader(reader: R) -> Self {
        Self {
            reader: PeekReader::new(reader),
            depth: 0,
        }
    }
}

impl<'de> Deserializer<&'de [u8]> {
    /// Creates a MASON deserializer from a `&[u8]`.
    pub fn from_slice(input: &'de [u8]) -> Self {
        Self::from_reader(input)
    }

    #[allow(clippy::should_implement_trait)]
    /// Creates a MASON deserializer from a `&str`.
    pub fn from_str(input: &'de str) -> Self {
        Self::from_reader(input.as_bytes())
    }
}

/// Deserialize an instance of type `T` from an I/O stream of MASON.
///
/// The content of the I/O stream is buffered in memory using a [`std::io::BufReader`].
///
/// It is expected that the input stream ends after the deserialized value.
/// If the stream does not end, such as in the case of a persistent socket connection,
/// this function will not return.
///
/// # Example
///
/// Reading the contents of a file.
///
/// ```
/// use serde::Deserialize;
///
/// use std::error::Error;
/// use std::fs::File;
/// use std::io::BufReader;
/// use std::path::Path;
///
/// #[derive(Deserialize, Debug)]
/// struct User {
///     fingerprint: String,
///     location: String,
/// }
///
/// fn read_user_from_file<P: AsRef<Path>>(path: P) -> Result<User, Box<dyn Error>> {
///     // Open the file in read-only mode with buffer.
///     let file = File::open(path)?;
///     let reader = BufReader::new(file);
///
///     // Read the MASON contents of the file as an instance of `User`.
///     let u = mason_rs::from_reader(reader)?;
///
///     // Return the `User`.
///     Ok(u)
/// }
///
/// fn main() {
/// # }
/// # fn fake_main() {
///     let u = read_user_from_file("test.mason").unwrap();
///     println!("{:#?}", u);
/// }
/// ```
///
/// Reading from a persistent socket connection.
///
/// ```
/// use serde::Deserialize;
///
/// use std::error::Error;
/// use std::io::BufReader;
/// use std::net::{TcpListener, TcpStream};
///
/// #[derive(Deserialize, Debug)]
/// struct User {
///     fingerprint: String,
///     location: String,
/// }
///
/// fn read_user_from_stream(stream: &mut BufReader<TcpStream>) -> Result<User, Box<dyn Error>> {
///     let mut de = mason_rs::serde::de::Deserializer::from_reader(stream);
///     let u = User::deserialize(&mut de)?;
///
///     Ok(u)
/// }
///
/// fn main() {
/// # }
/// # fn fake_main() {
///     let listener = TcpListener::bind("127.0.0.1:4000").unwrap();
///
///     for tcp_stream in listener.incoming() {
///         let mut buffered = BufReader::new(tcp_stream.unwrap());
///         println!("{:#?}", read_user_from_stream(&mut buffered));
///     }
/// }
/// ```
///
/// # Errors
///
/// This conversion can fail if the structure of the input does not match the
/// structure expected by `T`, for example if `T` is a struct type but the input
/// contains something other than a MASON map. It can also fail if the structure
/// is correct but `T`'s implementation of `Deserialize` decides that something
/// is wrong with the data, for example required struct fields are missing from
/// the MASON map or some number is too big to fit in the expected primitive
/// type.
pub fn from_reader<'de, T, R>(reader: R) -> Result<T>
where
    T: Deserialize<'de>,
    R: Read + 'de,
{
    let mut deserializer = Deserializer::from_reader(reader);
    let t = T::deserialize(&mut deserializer)?;
    deserialize::skip_whitespace(&mut deserializer.reader)?;
    if let Some(garbage) = deserializer.reader.peek()? {
        Err(Error::from(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Trailing garbage after document: {:?}",
                utils::to_char(garbage)
            ),
        )))
    } else {
        Ok(t)
    }
}

/// Deserialize an instance of type `T` from bytes of MASON text.
///
/// # Example
///
/// ```
/// use serde::Deserialize;
///
/// #[derive(Deserialize, Debug)]
/// struct User {
///     fingerprint: String,
///     location: String,
/// }
///
/// // The type of `j` is `&[u8]`
/// let j = b"
///     fingerprint: \"0xF9BA143B95FF6D82\",
///     location: \"Menlo Park, CA\"
/// ";
///
/// let u: User = mason_rs::from_slice(j).unwrap();
/// println!("{:#?}", u);
/// ```
///
/// # Errors
///
/// This conversion can fail if the structure of the input does not match the
/// structure expected by `T`, for example if `T` is a struct type but the input
/// contains something other than a MASON map. It can also fail if the structure
/// is correct but `T`'s implementation of `Deserialize` decides that something
/// is wrong with the data, for example required struct fields are missing from
/// the MASON map or some number is too big to fit in the expected primitive
/// type.
pub fn from_slice<'de, T>(bytes: &'de [u8]) -> Result<T>
where
    T: Deserialize<'de>,
{
    from_reader(bytes)
}

/// Deserialize an instance of type `T` from a string of MASON text.
///
/// # Example
///
/// ```
/// use serde::Deserialize;
///
/// #[derive(Deserialize, Debug)]
/// struct User {
///     fingerprint: String,
///     location: String,
/// }
///
/// // The type of `j` is `&str`
/// let j = "
///     fingerprint: \"0xF9BA143B95FF6D82\"
///     location: \"Menlo Park, CA\"
/// ";
///
/// let u: User = mason_rs::from_str(j).unwrap();
/// println!("{:#?}", u);
/// ```
///
/// # Errors
///
/// This conversion can fail if the structure of the input does not match the
/// structure expected by `T`, for example if `T` is a struct type but the input
/// contains something other than a MASON map. It can also fail if the structure
/// is correct but `T`'s implementation of `Deserialize` decides that something
/// is wrong with the data, for example required struct fields are missing from
/// the MASON map or some number is too big to fit in the expected primitive
/// type.
pub fn from_str<'de, T>(string: &'de str) -> Result<T>
where
    T: Deserialize<'de>,
{
    from_reader(string.as_bytes())
}

impl<R: Read> Deserializer<R> {
    // read_byte, but return Error::Eof on EOF
    fn expect_read_byte(&mut self) -> Result<u8> {
        match self.reader.read_byte() {
            Ok(Some(byte)) => Ok(byte),
            Ok(None) => Err(Error::eof()),
            Err(err) => Err(Error::from(err)),
        }
    }

    // peek, but return Error::Eof on EOF
    fn expect_peek(&mut self) -> Result<u8> {
        match self.reader.peek() {
            Ok(Some(byte)) => Ok(byte),
            Ok(None) => Err(Error::eof()),
            Err(err) => Err(Error::from(err)),
        }
    }

    // peek2, but return Error::Eof on EOF
    fn expect_peek2(&mut self) -> Result<[u8; 2]> {
        match self.reader.peek2() {
            Ok(Some(bytes)) => Ok(bytes),
            Ok(None) => Err(Error::eof()),
            Err(err) => Err(Error::from(err)),
        }
    }
}

/// Deserialize an f64, and see if it can be converted into the given type
macro_rules! deserialize_integer {
    ($type:ty) => {
        paste! {
            fn [<deserialize_ $type>]<V>(self, visitor: V) -> Result<V::Value>
            where
                V: Visitor<'de>,
            {
                let num = $crate::deserialize::parse_number(&mut self.reader)?;
                if num.fract() != 0.0 || num >  $type::MAX as f64 || num <  $type::MIN as f64 {
                    Err(Error::invalid_type(
                        Unexpected::Float(num),
                        &stringify!($type),
                    ))
                } else {
                    visitor.[<visit_ $type>](num as  $type)
                }
            }
        }
    };
}

impl<'de, R: Read + 'de> de::Deserializer<'de> for &mut Deserializer<R> {
    type Error = Error;

    // Look at the input data to decide what Serde data model type to
    // deserialize as. Not all data formats are able to support this operation.
    // Formats that support `deserialize_any` are known as self-describing.
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.depth == 100 {
            return Err(Error::custom("reached maximum depth"));
        }

        deserialize::skip_whitespace(&mut self.reader)?;

        let first_byte = self.expect_peek()?;
        match first_byte {
            b'{' => return self.deserialize_map(visitor),
            b'[' => return self.deserialize_seq(visitor),
            b'"' => {
                let string = deserialize::parse_string(&mut self.reader)?;
                if self.depth == 0 {
                    deserialize::skip_whitespace(&mut self.reader)?;
                    if self.reader.peek()? == Some(b':') {
                        return visitor
                            .visit_map(SepSeparated::with_initial_key(self, false, string));
                    }
                }
                return string.into_deserializer().deserialize_string(visitor);
            }
            b'r' => {
                if let Some([_, second_byte]) = self.reader.peek2()? {
                    if matches!(second_byte, b'"' | b'#') {
                        return self.deserialize_string(visitor);
                    }
                }
            }
            b'|' => return self.deserialize_string(visitor),
            b'b' => {
                if let Some([_, second_byte]) = self.reader.peek2()? {
                    if matches!(second_byte, b'"') {
                        return self.deserialize_bytes(visitor);
                    }
                }
            }
            _ => {}
        }

        if first_byte.is_ascii_digit() || matches!(first_byte, b'+' | b'-' | b'.') {
            self.deserialize_f64(visitor)
        } else {
            let identifier = deserialize::parse_identifier(&mut self.reader)?;
            if self.depth == 0 {
                deserialize::skip_whitespace(&mut self.reader)?;
                if self.reader.peek()? == Some(b':') {
                    return visitor
                        .visit_map(SepSeparated::with_initial_key(self, false, identifier));
                }
            }
            match identifier.as_str() {
                "true" => visitor.visit_bool(true),
                "false" => visitor.visit_bool(false),
                "null" => visitor.visit_unit(),
                _ => Err(Error::custom(format!("malformed value: {identifier}"))),
            }
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let expected: &[u8] = if self.expect_peek()? == b't' {
            b"true"
        } else {
            b"false"
        };
        for b in expected {
            let byte = self.expect_read_byte()?;
            if byte != *b {
                return Err(Error::invalid_type(
                    Unexpected::Char(utils::to_char(*b)),
                    &"bool",
                ));
            }
        }
        visitor.visit_bool(expected == b"true")
    }

    deserialize_integer!(i8);
    deserialize_integer!(i16);
    deserialize_integer!(i32);
    deserialize_integer!(i64);

    deserialize_integer!(u8);
    deserialize_integer!(u16);
    deserialize_integer!(u32);
    deserialize_integer!(u64);

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let num = deserialize::parse_number(&mut self.reader)?;
        let num_f32 = num as f32;

        // se if num is representable as an f32
        if (num - f64::from(num_f32)).abs() > 10.0 * f64::from(f32::EPSILON) {
            Err(Error::invalid_type(Unexpected::Float(num), &"f32"))
        } else {
            visitor.visit_f32(num_f32)
        }
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f64(deserialize::parse_number(&mut self.reader)?)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let string = deserialize::parse_string(&mut self.reader)?;
        let mut chars = string.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => visitor.visit_char(c),
            _ => Err(Error::invalid_type(Unexpected::Str(&string), &"char")),
        }
    }

    // Can we get a 'de str here? I don't think so, right?
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let byte = self.expect_peek()?;
        match byte {
            b'"' => visitor.visit_string(deserialize::parse_string(&mut self.reader)?),
            b'r' => visitor.visit_string(deserialize::parse_raw_string(&mut self.reader)?),
            b'|' => visitor.visit_string(deserialize::parse_multi_line_string(&mut self.reader)?),
            _ => Err(Error::invalid_type(
                Unexpected::Char(utils::to_char(byte)),
                &"string",
            )),
        }
    }

    // Can we get a 'de [u8] here? I don't think so, right?
    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_byte_buf(visitor)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_byte_buf(deserialize::parse_byte_string(&mut self.reader)?)
    }

    // An absent optional is represented as the MASON `null` and a present
    // optional is represented as just the contained value.
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // TODO: the first two bytes being 'nu' does not actually guarantee
        // that the value is 'null'
        if &self.expect_peek2()? == b"nu" {
            self.reader.consume(4);
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    // In Serde, unit means an anonymous value containing no data.
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        for b in b"null" {
            let byte = self.expect_read_byte()?;
            if byte != *b {
                return Err(Error::invalid_type(
                    Unexpected::Char(utils::to_char(byte)),
                    &"unit",
                ));
            }
        }
        visitor.visit_unit()
    }

    // Unit struct means a named value containing no data.
    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    // As is done here, serializers are encouraged to treat newtype structs as
    // insignificant wrappers around the data they contain. That means not
    // parsing anything other than the contained value.
    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    // Deserialization of compound types like sequences and maps happens by
    // passing the visitor an "Access" object that gives it the ability to
    // iterate through the data contained in the sequence.
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // Parse the opening bracket of the sequence.
        let byte = self.expect_read_byte()?;
        if byte == b'[' {
            // Give the visitor access to each element of the sequence.
            let value = visitor.visit_seq(SepSeparated::new(self, true))?;
            // Parse the closing bracket of the sequence.
            let byte = self.expect_read_byte()?;
            if byte == b']' {
                Ok(value)
            } else {
                Err(Error::invalid_type(
                    Unexpected::Char(utils::to_char(byte)),
                    &"array end",
                ))
            }
        } else {
            Err(Error::invalid_type(
                Unexpected::Char(utils::to_char(byte)),
                &"seq",
            ))
        }
    }

    // Tuples look just like sequences in MASON.
    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    // Tuple structs look just like sequences in MASON.
    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.depth == 0 {
            // If depth is 0, struct does not need to be surrounded by braces
            let mut has_opening_brace = false;
            if self.expect_peek()? == b'{' {
                self.reader.read_byte()?;
                has_opening_brace = true;
            }

            self.depth += 1;
            let value = visitor.visit_map(SepSeparated::new(self, has_opening_brace))?;
            self.depth -= 1;

            match (has_opening_brace, self.reader.peek()? == Some(b'}')) {
                (true, true) => {
                    self.reader.read_byte()?;
                }
                (false, true) => {
                    return Err(Error::custom(
                        "got closing bracket without an opening bracket",
                    ));
                }
                (true, false) => return Err(Error::custom("unclosed bracket")),
                (false, false) => {}
            }
            return Ok(value);
        }

        let byte = self.expect_read_byte()?;
        if byte == b'{' {
            self.depth += 1;
            let value = visitor.visit_map(SepSeparated::new(self, true))?;
            self.depth -= 1;

            let byte = self.expect_read_byte()?;
            if byte == b'}' {
                Ok(value)
            } else {
                Err(Error::invalid_type(
                    Unexpected::Char(utils::to_char(byte)),
                    &"map end",
                ))
            }
        } else {
            Err(Error::invalid_type(
                Unexpected::Char(utils::to_char(byte)),
                &"map",
            ))
        }
    }

    // Structs look just like maps in MASON.
    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let variant = deserialize::parse_identifier(&mut self.reader)?;
        deserialize::skip_whitespace(&mut self.reader)?;

        if self.reader.peek()? != Some(b':') {
            // Visit a unit variant.
            visitor.visit_enum(variant.into_deserializer())
        } else if self.depth == 0 {
            // skip colon
            self.reader.read_byte()?;
            deserialize::skip_whitespace(&mut self.reader)?;

            Ok(visitor.visit_enum(Enum::new(self, variant))?)
        } else {
            todo!()
        }
    }

    // An identifier in Serde is the type that identifies a field of a struct or
    // the variant of an enum. In MASON, struct fields and enum variants are
    // represented as strings. In other formats they may be represented as
    // numeric indices.
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(deserialize::parse_identifier(&mut self.reader)?)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

// In order to handle seps correctly when deserializing a MASON array or map,
// we need to track whether we are on the first element or past the first
// element.
struct SepSeparated<'a, R: Read> {
    de: &'a mut Deserializer<R>,
    first: bool,
    // should we expect a closing bracket?
    expect_closing: bool,
    first_key: Option<String>,
    // a multi line string is always a valid sep
    previously_parsed_multi_line_string: bool,
}

impl<'a, R: Read> SepSeparated<'a, R> {
    fn new(de: &'a mut Deserializer<R>, expect_closing: bool) -> Self {
        SepSeparated {
            de,
            first: true,
            expect_closing,
            first_key: None,
            previously_parsed_multi_line_string: false,
        }
    }

    fn with_initial_key(
        de: &'a mut Deserializer<R>,
        expect_closing: bool,
        first_key: String,
    ) -> Self {
        SepSeparated {
            de,
            first: true,
            expect_closing,
            first_key: Some(first_key),
            previously_parsed_multi_line_string: false,
        }
    }
}

// `SeqAccess` is provided to the `Visitor` to give it the ability to iterate
// through elements of the sequence.
impl<'de, R: Read + 'de> SeqAccess<'de> for SepSeparated<'_, R> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        if !self.first {
            let valid_sep = self.previously_parsed_multi_line_string
                || deserialize::parse_sep(&mut self.de.reader)?;
            deserialize::skip_whitespace(&mut self.de.reader)?;

            if !valid_sep {
                if self.de.expect_peek()? == b']' {
                    return Ok(None);
                } else {
                    return Err(Error::custom("array missing sep"));
                }
            }
        }
        self.first = false;

        deserialize::skip_whitespace(&mut self.de.reader)?;

        // Check if there are no more elements.
        if self.de.expect_peek()? == b']' {
            return Ok(None);
        }

        // Deserialize an array element.
        self.de.depth += 1;
        self.previously_parsed_multi_line_string = self.de.reader.peek()? == Some(b'|');
        let result = seed.deserialize(&mut *self.de).map(Some);
        self.de.depth -= 1;

        result
    }
}

// `MapAccess` is provided to the `Visitor` to give it the ability to iterate
// through entries of the map.
impl<'de, R: Read + 'de> MapAccess<'de> for SepSeparated<'_, R> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        if let Some(key) = self.first_key.take() {
            self.first = false;
            return seed.deserialize(key.into_deserializer()).map(Some);
        }

        let valid_sep = if !self.first {
            self.previously_parsed_multi_line_string || deserialize::parse_sep(&mut self.de.reader)?
        } else {
            true
        };
        deserialize::skip_whitespace(&mut self.de.reader)?;

        match (self.de.reader.peek()?, self.expect_closing) {
            (Some(b'}'), true) | (None, false) => return Ok(None),
            (Some(b'}'), false) => {
                return Err(Error::custom(
                    "got closing bracket without an opening bracket",
                ));
            }
            (None, true) => return Err(Error::custom("unclosed bracket")),
            _ => {}
        }

        if !valid_sep {
            return Err(Error::custom("map missing sep"));
        }
        self.first = false;

        let key = if self.de.expect_peek()? == b'"' {
            deserialize::parse_string(&mut self.de.reader)?
        } else {
            deserialize::parse_identifier(&mut self.de.reader)?
        };

        seed.deserialize(key.into_deserializer()).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        deserialize::skip_whitespace(&mut self.de.reader)?;
        let byte = self.de.expect_read_byte()?;
        if byte != b':' {
            return Err(Error::invalid_type(
                Unexpected::Char(utils::to_char(byte)),
                &"map colon",
            ));
        }
        deserialize::skip_whitespace(&mut self.de.reader)?;

        // Deserialize a map value.
        self.de.depth += 1;
        self.previously_parsed_multi_line_string = self.de.reader.peek()? == Some(b'|');
        let result = seed.deserialize(&mut *self.de);
        self.de.depth -= 1;

        result
    }
}

struct Enum<'a, R: Read> {
    de: &'a mut Deserializer<R>,
    variant: Option<String>,
}

impl<'a, R: Read> Enum<'a, R> {
    fn new(de: &'a mut Deserializer<R>, variant: String) -> Self {
        Enum {
            de,
            variant: Some(variant),
        }
    }
}

// `EnumAccess` is provided to the `Visitor` to give it the ability to determine
// which variant of the enum is supposed to be deserialized.
//
// Note that all enum deserialization methods in Serde refer exclusively to the
// "externally tagged" enum representation.
impl<'de, R: Read + 'de> EnumAccess<'de> for Enum<'_, R> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(mut self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        // We have already parsed the enum varian, so we don't need to do anything here
        let Some(variant) = self.variant.take() else {
            return Err(Error::custom("variant_seed got called more than once?"));
        };
        let string_deserializer: StringDeserializer<Error> = variant.into_deserializer();
        Ok((seed.deserialize(string_deserializer)?, self))
    }
}

// `VariantAccess` is provided to the `Visitor` to give it the ability to see
// the content of the single variant that it decided to deserialize.
impl<'de, R: Read + 'de> VariantAccess<'de> for Enum<'_, R> {
    type Error = Error;

    // If the `Visitor` expected this variant to be a unit variant, the input
    // should have been the plain string case handled in `deserialize_enum`.
    fn unit_variant(self) -> Result<()> {
        Err(Error::invalid_type(
            Unexpected::UnitVariant,
            &"visitor should have handled unit variant case",
        ))
    }

    // Newtype variants are represented in MASON as `{ NAME: VALUE }` so
    // deserialize the value here.
    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(self.de)
    }

    // Tuple variants are represented in MASON as `{ NAME: [DATA...] }` so
    // deserialize the sequence of data here.
    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_seq(self.de, visitor)
    }

    // Struct variants are represented in MASON as `{ NAME: { K: V, ... } }` so
    // deserialize the inner map here.
    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_map(self.de, visitor)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn test_struct() {
        #[derive(Deserialize, PartialEq, Debug)]
        struct Test {
            int: u32,
            seq: Vec<String>,
        }

        let j = "\
int: 1
seq: [\"a\", \"b\"]";
        let expected = Test {
            int: 1,
            seq: vec!["a".to_owned(), "b".to_owned()],
        };
        assert_eq!(expected, from_str(j).unwrap());
    }

    #[test]
    fn test_enum() {
        #[derive(Deserialize, PartialEq, Debug)]
        enum E {
            Unit,
            Newtype(u32),
            Tuple(u32, u32),
            Struct { a: u32 },
        }

        let j = "Unit";
        let expected = E::Unit;
        assert_eq!(expected, from_str(j).unwrap());

        let j = "Newtype: 1";
        let expected = E::Newtype(1);
        assert_eq!(expected, from_str(j).unwrap());

        let j = "Tuple: [1, 2]";
        let expected = E::Tuple(1, 2);
        assert_eq!(expected, from_str(j).unwrap());

        let j = "\
Struct: {
    \"a\": 1
}";
        let expected = E::Struct { a: 1 };
        assert_eq!(expected, from_str(j).unwrap());
    }

    #[test]
    fn test_complicated() {
        #[derive(Deserialize, PartialEq, Debug)]
        struct Complicated {
            map: HashMap<String, Vec<f32>>,
            bytes: Vec<u8>,
            option: Option<String>,
            nothing: (),
        }

        let j = "\
map: {
    \"a \\\" \\\\ \\\\\\\" difficult key 🏳️‍⚧️\": [-1000000000, 1230, 0.000000000321]
}
bytes: [66, 121, 116, 101, 115, 33]
option: null
nothing: null";
        let expected = Complicated {
            map: HashMap::from([(
                "a \" \\ \\\" difficult key 🏳️‍⚧️".into(),
                vec![-1e9, 1.23e3, 3.21e-10],
            )]),
            bytes: b"Bytes!".to_vec(),
            option: None,
            nothing: (),
        };
        assert_eq!(expected, from_str(j).unwrap());
    }
}
