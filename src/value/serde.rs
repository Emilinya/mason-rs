use std::{collections::HashMap, fmt};

use serde::{
    Deserialize, Serialize,
    de::{MapAccess, SeqAccess, Visitor},
};

use crate::Value;

impl Serialize for Value {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Null => serializer.serialize_unit(),
            Self::Bool(b) => serializer.serialize_bool(*b),
            Self::Number(f) => serializer.serialize_f64(*f),
            Self::String(s) => serializer.serialize_str(s),
            Self::ByteString(v) => serializer.serialize_bytes(v),
            Self::Array(v) => v.serialize(serializer),
            Self::Object(m) => m.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for Value {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ValueVisitor;

        impl<'de> Visitor<'de> for ValueVisitor {
            type Value = Value;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("any valid MASON value")
            }

            #[inline]
            fn visit_bool<E>(self, value: bool) -> Result<Value, E> {
                Ok(Value::Bool(value))
            }

            #[inline]
            fn visit_i64<E>(self, value: i64) -> Result<Value, E>
            where
                E: serde::de::Error,
            {
                // The largest whole number representable by a f64
                const MAX: i64 = 2i64.pow(f64::MANTISSA_DIGITS) + 1;

                if value.abs() <= MAX {
                    Ok(Value::Number(value as f64))
                } else {
                    Err(serde::de::Error::invalid_value(
                        serde::de::Unexpected::Signed(value),
                        &self,
                    ))
                }
            }

            #[inline]
            fn visit_i128<E>(self, value: i128) -> Result<Value, E>
            where
                E: serde::de::Error,
            {
                // The largest whole number representable by a f64
                const MAX: i128 = 2i128.pow(f64::MANTISSA_DIGITS) + 1;

                if value.abs() <= MAX {
                    Ok(Value::Number(value as f64))
                } else {
                    Err(serde::de::Error::invalid_value(
                        serde::de::Unexpected::Other(&format!("integer `{value}` as i128")),
                        &self,
                    ))
                }
            }

            #[inline]
            fn visit_u64<E>(self, value: u64) -> Result<Value, E>
            where
                E: serde::de::Error,
            {
                // The largest whole number representable by a f64
                const MAX: u64 = 2u64.pow(f64::MANTISSA_DIGITS) + 1;

                if value <= MAX {
                    Ok(Value::Number(value as f64))
                } else {
                    Err(serde::de::Error::invalid_value(
                        serde::de::Unexpected::Unsigned(value),
                        &self,
                    ))
                }
            }

            #[inline]
            fn visit_u128<E>(self, value: u128) -> Result<Value, E>
            where
                E: serde::de::Error,
            {
                // The largest whole number representable by a f64
                const MAX: u128 = 2u128.pow(f64::MANTISSA_DIGITS) + 1;

                if value <= MAX {
                    Ok(Value::Number(value as f64))
                } else {
                    Err(serde::de::Error::invalid_value(
                        serde::de::Unexpected::Other(&format!("integer `{value}` as u128")),
                        &self,
                    ))
                }
            }

            #[inline]
            fn visit_f64<E>(self, value: f64) -> Result<Value, E> {
                Ok(Value::Number(value))
            }

            #[inline]
            fn visit_str<E>(self, value: &str) -> Result<Value, E>
            where
                E: serde::de::Error,
            {
                self.visit_string(String::from(value))
            }

            #[inline]
            fn visit_string<E>(self, value: String) -> Result<Value, E> {
                Ok(Value::String(value))
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E> {
                Ok(Value::ByteString(v.to_vec()))
            }

            fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E> {
                Ok(Value::ByteString(v))
            }

            #[inline]
            fn visit_none<E>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }

            #[inline]
            fn visit_some<D>(self, deserializer: D) -> Result<Value, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                Deserialize::deserialize(deserializer)
            }

            #[inline]
            fn visit_unit<E>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }

            #[inline]
            fn visit_seq<V>(self, mut visitor: V) -> Result<Value, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let mut vec = Vec::new();

                while let Some(elem) = visitor.next_element()? {
                    vec.push(elem);
                }

                Ok(Value::Array(vec))
            }

            fn visit_map<V>(self, mut visitor: V) -> Result<Value, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut values = HashMap::new();

                while let Some((key, value)) = visitor.next_entry()? {
                    values.insert(key, value);
                }

                Ok(Value::Object(values))
            }
        }

        deserializer.deserialize_any(ValueVisitor)
    }
}
