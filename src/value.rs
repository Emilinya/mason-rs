use std::{collections::HashMap, mem};

use crate::index::Index;

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

impl Value {
    /// Return a string description of the `Value`.
    ///
    /// ```
    /// # use mason_rs::Value;
    /// #
    /// let value = mason_rs::from_string(r#"{a: 2, b: false}"#).unwrap();
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
    /// #
    /// let object = mason_rs::from_string(r#"{ "A": 65, "B": 66, "C": 67 }"#).unwrap();
    /// assert_eq!(*object.get("A").unwrap(), Value::Number(65.0));
    ///
    /// let array = mason_rs::from_string(r#"[ "A", "B", "C" ]"#).unwrap();
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
    /// #
    /// let object = mason_rs::from_string(r#"{
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
    /// #
    /// let mut object = mason_rs::from_string(r#"{ "A": 65, "B": 66, "C": 67 }"#).unwrap();
    /// *object.get_mut("A").unwrap() = Value::Number(69.0);
    ///
    /// let mut array = mason_rs::from_string(r#"[ "A", "B", "C" ]"#).unwrap();
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
    /// let obj = mason_rs::from_string(r#"{ "a": { "nested": true }, "b": ["an", "array"] }"#).unwrap();
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
    /// let v = mason_rs::from_string(r#"{ "a": { "nested": true }, "b": ["an", "array"] }"#).unwrap();
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
    /// let mut v = mason_rs::from_string(r#"{ "a": { "nested": true } }"#).unwrap();
    ///
    /// v["a"].as_object_mut().unwrap().clear();
    /// assert_eq!(v, mason_rs::from_string(r#"{ "a": {} }"#).unwrap());
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
    /// let obj = mason_rs::from_string(r#"{ "a": ["an", "array"], "b": { "an": "object" } }"#).unwrap();
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
    /// let v = mason_rs::from_string(r#"{ "a": ["an", "array"], "b": { "an": "object" } }"#).unwrap();
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
    /// let mut v = mason_rs::from_string(r#"{ "a": ["an", "array"] }"#).unwrap();
    ///
    /// v["a"].as_array_mut().unwrap().clear();
    /// assert_eq!(v, mason_rs::from_string(r#"{ "a": [] }"#).unwrap());
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
    /// let v = mason_rs::from_string(r#"{ "a": "some string", "b": false }"#).unwrap();
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
    /// let v = mason_rs::from_string(r#"{ "a": "some string", "b": false }"#).unwrap();
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
    /// let v = mason_rs::from_string(r#"{ "a": 1, "b": "2" }"#).unwrap();
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
    /// let v = mason_rs::from_string(r#"{ "a": 1, "b": "2" }"#).unwrap();
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
    /// let v = mason_rs::from_string(r#"{ "a": false, "b": "false" }"#).unwrap();
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
    /// let v = mason_rs::from_string(r#"{ "a": false, "b": "false" }"#).unwrap();
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
    /// let v = mason_rs::from_string(r#"{ "a": null, "b": false }"#).unwrap();
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
    /// let v = mason_rs::from_string(r#"{ "a": null, "b": false }"#).unwrap();
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
    /// #
    /// let mut v = mason_rs::from_string(r#"{ "x": "y" }"#).unwrap();
    /// assert_eq!(v["x"].take(), Value::String("y".into()));
    /// assert_eq!(v, mason_rs::from_string(r#"{ "x": null }"#).unwrap());
    /// ```
    pub fn take(&mut self) -> Self {
        mem::replace(self, Self::Null)
    }
}
