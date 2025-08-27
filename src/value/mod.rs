#[cfg(feature = "serde")]
pub mod serde;

use std::{
    collections::HashMap,
    fmt::{self, Display, Write},
    io::{self, Read},
    mem,
    str::FromStr,
};

use crate::{deserialize, index::Index, peek_reader::PeekReader, serialize::write_indented_value};

/// Represents any valid MASON value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Object(HashMap<String, Value>),
    Array(Vec<Value>),
    String(String),
    ByteString(Vec<u8>),
    Number(f64),
    Bool(bool),
    Null,
}

impl Default for Value {
    fn default() -> Self {
        Self::Null
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_writer(f)
    }
}

impl FromStr for Value {
    type Err = io::Error;

    /// Deserialize a [`Value`] from a MASON string.
    ///
    /// # Example
    ///
    /// ```
    /// # use mason_rs::Value;
    /// # use std::str::FromStr;
    /// #
    /// let data = Value::from_str("[1.0, true, null]").unwrap();
    /// assert_eq!(data, Value::Array(vec![Value::Number(1.0), Value::Bool(true), Value::Null]))
    ///
    /// ```
    ///
    /// # Errors
    ///
    /// This function can fail if the string is not valid MASON.
    fn from_str(string: &str) -> io::Result<Self> {
        Self::from_reader(string.as_bytes())
    }
}

impl Value {
    /// Deserialize a [`Value`] from an I/O stream of MASON.
    ///
    /// The content of the I/O stream is buffered in memory using a [`std::io::BufReader`].
    ///
    /// It is expected that the input stream ends after the deserialized value.
    /// If the stream does not end, such as in the case of a persistent socket connection,
    /// this function will not return.
    ///
    /// # Example
    ///
    /// ```
    /// # use mason_rs::Value;
    /// # use std::str::FromStr;
    /// #
    /// use std::fs::File;
    ///
    /// fn main() {
    /// # }
    /// # fn fake_main() {
    ///     let value = Value::from_reader(File::open("test.mason").unwrap()).unwrap();
    ///     println!("{:?}", value);
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// This function can fail if the I/O stream is not valid MASON, or if any errors were
    /// encountered while reading from the stream.
    pub fn from_reader(reader: impl Read) -> io::Result<Self> {
        let mut peek_reader = PeekReader::new(reader);
        deserialize::parse_document(&mut peek_reader)
    }

    /// Deserialize a [`Value`] from a slice of MASON bytes.
    ///
    /// # Example
    ///
    /// ```
    /// # use mason_rs::Value;
    /// # use std::str::FromStr;
    /// #
    /// let data = Value::from_slice(b"[1.0, true, null]").unwrap();
    /// assert_eq!(data, Value::Array(vec![Value::Number(1.0), Value::Bool(true), Value::Null]))
    /// ```
    ///
    /// # Errors
    ///
    /// This function can fail if the byte slice is not valid MASON.
    pub fn from_slice(bytes: &[u8]) -> io::Result<Self> {
        Self::from_reader(bytes)
    }

    /// Serialize a [`Value`] using the given writer.
    ///
    /// # Example
    ///
    /// ```
    /// # use mason_rs::Value;
    /// # use std::str::FromStr;
    /// #
    /// let value_string = r#"vec: [1, true, false, null]"#;
    /// let value = Value::from_str(value_string).unwrap();
    ///
    /// let mut writer = String::new();
    /// Value::to_writer(&value, &mut writer);
    /// assert_eq!(writer, value_string);
    /// ```
    ///
    /// This is also the function used by `Value`'s display implementation:
    ///
    /// ```
    /// # use mason_rs::Value;
    /// # use std::str::FromStr;
    /// #
    /// let value_string = r#""some bytes": b"This \b \x0e\t is \x7f bytes!""#;
    /// let value = Value::from_str(value_string).unwrap();
    ///
    /// assert_eq!(value.to_string(), value_string);
    /// ```
    pub fn to_writer<W: Write>(&self, writer: &mut W) -> fmt::Result {
        write_indented_value(self, writer, "    ", 0)
    }

    /// Return a string description of the `Value`.
    ///
    /// ```
    /// # use mason_rs::Value;
    /// # use std::str::FromStr;
    /// #
    /// let value = Value::from_str(r#"{a: 2, b: false}"#).unwrap();
    /// assert_eq!(value.value_type(), "object");
    /// assert_eq!(value["a"].value_type(), "number");
    /// assert_eq!(value["b"].value_type(), "boolean");
    /// ```
    pub fn value_type(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Bool(_) => "boolean",
            Self::Number(_) => "number",
            Self::String(_) => "string",
            Self::ByteString(_) => "byte string",
            Self::Array(_) => "array",
            Self::Object(_) => "object",
        }
    }

    /// Index into a MASON array or object. A string index can be used to access a
    /// value in an object, and a usize index can be used to access an element of an
    /// array.
    ///
    /// Returns `None` if the type of `self` does not match the type of the
    /// index, for example if the index is a string and `self` is an array or a
    /// number. Also returns `None` if the given key does not exist in the object
    /// or the given index is not within the bounds of the array.
    ///
    /// ```
    /// # use mason_rs::Value;
    /// # use std::str::FromStr;
    /// #
    /// let object = Value::from_str(r#"{ "A": 65, "B": 66, "C": 67 }"#).unwrap();
    /// assert_eq!(*object.get("A").unwrap(), Value::Number(65.0));
    ///
    /// let array = Value::from_str(r#"[ "A", "B", "C" ]"#).unwrap();
    /// assert_eq!(*array.get(2).unwrap(), Value::String("C".into()));
    ///
    /// assert_eq!(array.get("A"), None);
    /// ```
    ///
    /// Square brackets can also be used to index into a value in a more concise
    /// way. This returns `Value::Null` in cases where `get` would have returned
    /// `None`.
    ///
    /// ```
    /// # use mason_rs::Value;
    /// # use std::str::FromStr;
    /// #
    /// let object = Value::from_str(r#"{
    ///     "A": ["a", "á", "à"],
    ///     "B": ["b", "b́"],
    ///     "C": ["c", "ć", "ć̣", "ḉ"],
    /// }"#).unwrap();
    /// assert_eq!(object["B"][0], Value::String("b".into()));
    ///
    /// assert_eq!(object["D"], Value::Null);
    /// assert_eq!(object[0]["x"]["y"]["z"], Value::Null);
    /// ```
    pub fn get<I: Index>(&self, index: I) -> Option<&Self> {
        index.index_into(self)
    }

    /// Mutably index into a MASON array or object. A string index can be used to
    /// access a value in an object, and a usize index can be used to access an
    /// element of an array.
    ///
    /// Returns `None` if the type of `self` does not match the type of the
    /// index, for example if the index is a string and `self` is an array or a
    /// number. Also returns `None` if the given key does not exist in the object
    /// or the given index is not within the bounds of the array.
    ///
    /// ```
    /// # use mason_rs::Value;
    /// # use std::str::FromStr;
    /// #
    /// let mut object = Value::from_str(r#"{ "A": 65, "B": 66, "C": 67 }"#).unwrap();
    /// *object.get_mut("A").unwrap() = Value::Number(69.0);
    ///
    /// let mut array = Value::from_str(r#"[ "A", "B", "C" ]"#).unwrap();
    /// *array.get_mut(2).unwrap() = Value::String("D".into());
    /// ```
    pub fn get_mut<I: Index>(&mut self, index: I) -> Option<&mut Self> {
        index.index_into_mut(self)
    }

    /// Returns true if the `Value` is an Object. Returns false otherwise.
    ///
    /// For any Value on which `is_object` returns true, `as_object` and
    /// `as_object_mut` are guaranteed to return the hashmap representing the object.
    ///
    /// ```
    /// # use mason_rs::Value;
    /// # use std::str::FromStr;
    /// #
    /// let obj = Value::from_str(r#"{ "a": { "nested": true }, "b": ["an", "array"] }"#).unwrap();
    ///
    /// assert!(obj.is_object());
    /// assert!(obj["a"].is_object());
    ///
    /// // array, not an object
    /// assert!(!obj["b"].is_object());
    /// ```
    pub fn is_object(&self) -> bool {
        self.as_object().is_some()
    }

    /// If the `Value` is an Object, returns the associated object. Returns None
    /// otherwise.
    ///
    /// ```
    /// # use mason_rs::Value;
    /// # use std::str::FromStr;
    /// #
    /// let v = Value::from_str(r#"{ "a": { "nested": true }, "b": ["an", "array"] }"#).unwrap();
    ///
    /// // The length of `{"nested": true}` is 1 entry.
    /// assert_eq!(v["a"].as_object().unwrap().len(), 1);
    ///
    /// // The array `["an", "array"]` is not an object.
    /// assert_eq!(v["b"].as_object(), None);
    /// ```
    pub fn as_object(&self) -> Option<&HashMap<String, Self>> {
        match self {
            Self::Object(map) => Some(map),
            _ => None,
        }
    }

    /// If the `Value` is an Object, returns the associated mutable object.
    /// Returns None otherwise.
    ///
    /// ```
    /// # use mason_rs::Value;
    /// # use std::str::FromStr;
    /// #
    /// let mut v = Value::from_str(r#"{ "a": { "nested": true } }"#).unwrap();
    ///
    /// v["a"].as_object_mut().unwrap().clear();
    /// assert_eq!(v, Value::from_str(r#"{ "a": {} }"#).unwrap());
    /// ```
    pub fn as_object_mut(&mut self) -> Option<&mut HashMap<String, Self>> {
        match self {
            Self::Object(map) => Some(map),
            _ => None,
        }
    }

    /// Returns true if the `Value` is an Array. Returns false otherwise.
    ///
    /// For any Value on which `is_array` returns true, `as_array` and
    /// `as_array_mut` are guaranteed to return the vector representing the
    /// array.
    ///
    /// ```
    /// # use mason_rs::Value;
    /// # use std::str::FromStr;
    /// #
    /// let obj = Value::from_str(r#"{ "a": ["an", "array"], "b": { "an": "object" } }"#).unwrap();
    ///
    /// assert!(obj["a"].is_array());
    ///
    /// // an object, not an array
    /// assert!(!obj["b"].is_array());
    /// ```
    pub fn is_array(&self) -> bool {
        self.as_array().is_some()
    }

    /// If the `Value` is an Array, returns the associated vector. Returns None
    /// otherwise.
    ///
    /// ```
    /// # use mason_rs::Value;
    /// # use std::str::FromStr;
    /// #
    /// let v = Value::from_str(r#"{ "a": ["an", "array"], "b": { "an": "object" } }"#).unwrap();
    ///
    /// // The length of `["an", "array"]` is 2 elements.
    /// assert_eq!(v["a"].as_array().unwrap().len(), 2);
    ///
    /// // The object `{"an": "object"}` is not an array.
    /// assert_eq!(v["b"].as_array(), None);
    /// ```
    pub fn as_array(&self) -> Option<&Vec<Self>> {
        match self {
            Self::Array(array) => Some(array),
            _ => None,
        }
    }

    /// If the `Value` is an Array, returns the associated mutable vector.
    /// Returns None otherwise.
    ///
    /// ```
    /// # use mason_rs::Value;
    /// # use std::str::FromStr;
    /// #
    /// let mut v = Value::from_str(r#"{ "a": ["an", "array"] }"#).unwrap();
    ///
    /// v["a"].as_array_mut().unwrap().clear();
    /// assert_eq!(v, Value::from_str(r#"{ "a": [] }"#).unwrap());
    /// ```
    pub fn as_array_mut(&mut self) -> Option<&mut Vec<Self>> {
        match self {
            Self::Array(list) => Some(list),
            _ => None,
        }
    }

    /// Returns true if the `Value` is a String. Returns false otherwise.
    ///
    /// For any Value on which `is_string` returns true, `as_str` is guaranteed
    /// to return the string slice.
    ///
    /// ```
    /// # use mason_rs::Value;
    /// # use std::str::FromStr;
    /// #
    /// let v = Value::from_str(r#"{ "a": "some string", "b": false }"#).unwrap();
    ///
    /// assert!(v["a"].is_string());
    ///
    /// // The boolean `false` is not a string.
    /// assert!(!v["b"].is_string());
    /// ```
    pub fn is_string(&self) -> bool {
        self.as_str().is_some()
    }

    /// If the `Value` is a String, returns the associated str. Returns None
    /// otherwise.
    ///
    /// ```
    /// # use mason_rs::Value;
    /// # use std::str::FromStr;
    /// #
    /// let v = Value::from_str(r#"{ "a": "some string", "b": false }"#).unwrap();
    ///
    /// assert_eq!(v["a"].as_str(), Some("some string"));
    ///
    /// // The boolean `false` is not a string.
    /// assert_eq!(v["b"].as_str(), None);
    /// ```
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    /// Returns true if the `Value` is a Number. Returns false otherwise.
    ///
    /// ```
    /// # use mason_rs::Value;
    /// # use std::str::FromStr;
    /// #
    /// let v = Value::from_str(r#"{ "a": 1, "b": "2" }"#).unwrap();
    ///
    /// assert!(v["a"].is_number());
    ///
    /// // The string `"2"` is a string, not a number.
    /// assert!(!v["b"].is_number());
    /// ```
    pub fn is_number(&self) -> bool {
        self.as_number().is_some()
    }

    /// If the `Value` is a Number, returns the associated double. Returns
    /// None otherwise.
    ///
    /// ```
    /// # use mason_rs::Value;
    /// # use std::str::FromStr;
    /// #
    /// let v = Value::from_str(r#"{ "a": 1, "b": "2" }"#).unwrap();
    ///
    /// assert_eq!(v["a"].as_number(), Some(&1.0));
    ///
    /// // The string `"2"` is not a number.
    /// assert_eq!(v["d"].as_number(), None);
    /// ```
    pub fn as_number(&self) -> Option<&f64> {
        match self {
            Self::Number(number) => Some(number),
            _ => None,
        }
    }

    /// Returns true if the `Value` is a Boolean. Returns false otherwise.
    ///
    /// For any Value on which `is_boolean` returns true, `as_bool` is
    /// guaranteed to return the boolean value.
    ///
    /// ```
    /// # use mason_rs::Value;
    /// # use std::str::FromStr;
    /// #
    /// let v = Value::from_str(r#"{ "a": false, "b": "false" }"#).unwrap();
    ///
    /// assert!(v["a"].is_boolean());
    ///
    /// // The string `"false"` is a string, not a boolean.
    /// assert!(!v["b"].is_boolean());
    /// ```
    pub fn is_boolean(&self) -> bool {
        self.as_bool().is_some()
    }

    /// If the `Value` is a Boolean, returns the associated bool. Returns None
    /// otherwise.
    ///
    /// ```
    /// # use mason_rs::Value;
    /// # use std::str::FromStr;
    /// #
    /// let v = Value::from_str(r#"{ "a": false, "b": "false" }"#).unwrap();
    ///
    /// assert_eq!(v["a"].as_bool(), Some(false));
    ///
    /// // The string `"false"` is a string, not a boolean.
    /// assert_eq!(v["b"].as_bool(), None);
    /// ```
    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            Self::Bool(b) => Some(b),
            _ => None,
        }
    }

    /// Returns true if the `Value` is a Null. Returns false otherwise.
    ///
    /// For any Value on which `is_null` returns true, `as_null` is guaranteed
    /// to return `Some(())`.
    ///
    /// ```
    /// # use mason_rs::Value;
    /// # use std::str::FromStr;
    /// #
    /// let v = Value::from_str(r#"{ "a": null, "b": false }"#).unwrap();
    ///
    /// assert!(v["a"].is_null());
    ///
    /// // The boolean `false` is not null.
    /// assert!(!v["b"].is_null());
    /// ```
    pub fn is_null(&self) -> bool {
        self.as_null().is_some()
    }

    /// If the `Value` is a Null, returns (). Returns None otherwise.
    ///
    /// ```
    /// # use mason_rs::Value;
    /// # use std::str::FromStr;
    /// #
    /// let v = Value::from_str(r#"{ "a": null, "b": false }"#).unwrap();
    ///
    /// assert_eq!(v["a"].as_null(), Some(()));
    ///
    /// // The boolean `false` is not null.
    /// assert_eq!(v["b"].as_null(), None);
    /// ```
    pub fn as_null(&self) -> Option<()> {
        match *self {
            Self::Null => Some(()),
            _ => None,
        }
    }

    /// Takes the value out of the `Value`, leaving a `Null` in its place.
    ///
    /// ```
    /// # use mason_rs::Value;
    /// # use std::str::FromStr;
    /// #
    /// let mut v = Value::from_str(r#"{ "x": "y" }"#).unwrap();
    /// assert_eq!(v["x"].take(), Value::String("y".into()));
    /// assert_eq!(v, Value::from_str(r#"{ "x": null }"#).unwrap());
    /// ```
    pub fn take(&mut self) -> Self {
        mem::replace(self, Self::Null)
    }
}
