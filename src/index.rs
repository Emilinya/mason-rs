use std::{collections::HashMap, ops};

use crate::Value;

/// A type that can be used to index into a `mason_rs::Value`.
///
/// The [`get`] and [`get_mut`] methods of `Value` accept any type that
/// implements `Index`, as does the [square-bracket indexing operator]. This
/// trait is implemented for strings which are used as the index into a MASON
/// object, and for `usize` which is used as the index into a MASON array.
///
/// [`get`]: Value::get
/// [`get_mut`]: Value::get_mut
/// [square-bracket indexing operator]: Value#impl-Index%3CI%3E-for-Value
///
/// This trait is sealed and cannot be implemented for types outside of
/// `mason`.
///
/// # Examples
///
/// ```
/// # use mason_rs::Value;
/// #
/// let data = mason_rs::from_string(r#"{ "inner": [1, 2, 3] }"#).unwrap();
///
/// // Data is a MASON object so it can be indexed with a string.
/// let inner = &data["inner"];
///
/// // Inner is a MASON array so it can be indexed with an integer.
/// let first = &inner[0];
///
/// assert_eq!(*first, Value::Number(1.0));
/// ```
pub trait Index: private::Sealed {
    #[doc(hidden)]
    fn index_into<'v>(&self, v: &'v Value) -> Option<&'v Value>;

    #[doc(hidden)]
    fn index_into_mut<'v>(&self, v: &'v mut Value) -> Option<&'v mut Value>;

    #[doc(hidden)]
    fn index_or_insert<'v>(&self, v: &'v mut Value) -> &'v mut Value;
}

impl Index for usize {
    fn index_into<'v>(&self, v: &'v Value) -> Option<&'v Value> {
        match v {
            Value::Array(vec) => vec.get(*self),
            _ => None,
        }
    }
    fn index_into_mut<'v>(&self, v: &'v mut Value) -> Option<&'v mut Value> {
        match v {
            Value::Array(vec) => vec.get_mut(*self),
            _ => None,
        }
    }

    /// Panics if index is bigger than array length. If index is equal to length,
    /// a Value of Null is inserted and returned.
    fn index_or_insert<'v>(&self, v: &'v mut Value) -> &'v mut Value {
        match v {
            Value::Array(vec) => {
                let mut len = vec.len();

                // Insert default value to make it possible to insert elements
                // using indexing like you can with objects.
                if *self == len {
                    vec.push(Value::Null);
                    len += 1;
                }

                vec.get_mut(*self).unwrap_or_else(|| {
                    panic!("cannot access index {self} of MASON array of length {len}")
                })
            }
            _ => panic!("cannot access index {} of MASON {}", self, v.value_type()),
        }
    }
}

impl Index for str {
    fn index_into<'v>(&self, v: &'v Value) -> Option<&'v Value> {
        match v {
            Value::Object(map) => map.get(self),
            _ => None,
        }
    }
    fn index_into_mut<'v>(&self, v: &'v mut Value) -> Option<&'v mut Value> {
        match v {
            Value::Object(map) => map.get_mut(self),
            _ => None,
        }
    }

    /// Panics if Value is neither an Object or Null. If Value is Null,
    /// it will be treated as an empty object. If the key is not already
    /// in the object, insert it with a value of null.
    fn index_or_insert<'v>(&self, v: &'v mut Value) -> &'v mut Value {
        if matches!(v, Value::Null) {
            *v = Value::Object(HashMap::new());
        }
        match v {
            Value::Object(map) => map.entry(self.to_owned()).or_insert(Value::Null),
            _ => panic!("cannot access key {:?} in MASON {}", self, v.value_type()),
        }
    }
}

impl Index for String {
    fn index_into<'v>(&self, v: &'v Value) -> Option<&'v Value> {
        self[..].index_into(v)
    }
    fn index_into_mut<'v>(&self, v: &'v mut Value) -> Option<&'v mut Value> {
        self[..].index_into_mut(v)
    }
    fn index_or_insert<'v>(&self, v: &'v mut Value) -> &'v mut Value {
        self[..].index_or_insert(v)
    }
}

impl<T> Index for &T
where
    T: ?Sized + Index,
{
    fn index_into<'v>(&self, v: &'v Value) -> Option<&'v Value> {
        (**self).index_into(v)
    }
    fn index_into_mut<'v>(&self, v: &'v mut Value) -> Option<&'v mut Value> {
        (**self).index_into_mut(v)
    }
    fn index_or_insert<'v>(&self, v: &'v mut Value) -> &'v mut Value {
        (**self).index_or_insert(v)
    }
}

// The usual semantics of Index is to panic on invalid indexing, but this
// does not make much sense for indexing into a Value. For this reason,
// invalid indexing returns `Value::Null`.
impl<I> ops::Index<I> for Value
where
    I: Index,
{
    type Output = Self;

    /// Index into a `mason_rs::Value` using the syntax `value[0]` or
    /// `value["k"]`.
    ///
    /// Returns `Value::Null` if the type of `self` does not match the type of
    /// the index, or if the given key does not exist in the map or the given
    /// index is not within the bounds of the array.
    ///
    /// # Examples
    ///
    /// ```
    /// # use mason_rs::Value;
    /// #
    /// let data = mason_rs::from_string(r#"{
    ///     "x": {
    ///         "y": ["z", "zz"]
    ///     }
    /// }"#).unwrap();
    ///
    /// assert_eq!(
    ///     data["x"]["y"],
    ///     Value::Array(vec![
    ///         Value::String("z".into()),
    ///         Value::String("zz".into()),
    ///     ]),
    /// );
    /// assert_eq!(data["x"]["y"][0], Value::String("z".into()));
    ///
    /// assert_eq!(data["a"], Value::Null); // returns null for undefined values
    /// assert_eq!(data["a"]["b"], Value::Null); // does not panic
    /// ```
    fn index(&self, index: I) -> &Self {
        static NULL: Value = Value::Null;
        index.index_into(self).unwrap_or(&NULL)
    }
}

impl<I> ops::IndexMut<I> for Value
where
    I: Index,
{
    /// Write into a `mason_rs::Value` using the syntax `value[0] = ...` or
    /// `value["k"] = ...`.
    ///
    /// If the index is a number, the value must be an array of length bigger
    /// than or equal to the index. Indexing into a value that is not an array or an array
    /// that is too small will panic.
    ///
    /// If the index is a string, the value must be an object or null which is
    /// treated like an empty object. If the key is not already present in the
    /// object, it will be inserted with a value of null. Indexing into a value
    /// that is neither an object nor null will panic.
    ///
    /// # Examples
    ///
    /// ```
    /// # use mason_rs::Value;
    /// #
    /// let mut data = mason_rs::from_string(r#"{ "x": 0 }"#).unwrap();
    ///
    /// // replace an existing key
    /// data["x"] = Value::Number(1.0);
    ///
    /// // insert a new key
    /// data["y"] = Value::Array(vec![Value::Bool(false), Value::Bool(true)]);
    ///
    /// // replace an array value
    /// data["y"][0] = Value::Bool(true);
    ///
    /// // insert a new array value
    /// data["y"][2] = Value::Number(1.3);
    ///
    /// // inserted a deeply nested key
    /// data["a"]["b"]["c"]["d"] = Value::Bool(true);
    ///
    /// println!("{:?}", data);
    /// ```
    fn index_mut(&mut self, index: I) -> &mut Self {
        index.index_or_insert(self)
    }
}

// Prevent users from implementing the Index trait.
mod private {
    pub trait Sealed {}
    impl Sealed for usize {}
    impl Sealed for str {}
    impl Sealed for String {}
    impl<T> Sealed for &T where T: ?Sized + Sealed {}
}
