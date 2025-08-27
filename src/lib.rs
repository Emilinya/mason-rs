mod deserialize;
mod hex;
mod index;
mod peek_reader;
mod serialize;
mod unescape_string;
mod utils;
mod value;

#[cfg(feature = "serde")]
pub mod serde;

#[cfg(test)]
mod tests;

pub use value::Value;

#[cfg(feature = "serde")]
#[doc(inline)]
pub use serde::{
    de::{Deserializer, from_reader, from_slice, from_str},
    ser::{Serializer, to_string, to_writer},
};
