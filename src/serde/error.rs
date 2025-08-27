//! When serializing or deserializing MASON goes wrong.

use std::{
    fmt::{self, Debug, Display},
    io,
};

use serde::{de, ser};

/// Alias for a `Result` with the error type `mason_rs::serde::error::Error`.
pub type Result<T> = std::result::Result<T, Error>;

/// This type represents all possible errors that can occur when serializing or
/// deserializing MASON data.
pub struct Error {
    /// This `Box` allows us to keep the size of `Error` as small as possible.
    inner: Box<InnerError>,
}

impl Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

#[derive(Debug)]
enum InnerError {
    // Created by data structures through the `ser::Error` and `de::Error` traits.
    Message(String),
    Io(io::Error),
    Eof,
    Fmt,
}

impl Error {
    #[inline]
    pub fn eof() -> Self {
        Self {
            inner: Box::new(InnerError::Eof),
        }
    }

    #[inline]
    pub fn fmt() -> Self {
        Self {
            inner: Box::new(InnerError::Fmt),
        }
    }
}

impl From<fmt::Error> for Error {
    fn from(_value: fmt::Error) -> Self {
        Self {
            inner: Box::new(InnerError::Fmt),
        }
    }
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        if matches!(value.kind(), io::ErrorKind::UnexpectedEof) {
            Self {
                inner: Box::new(InnerError::Eof),
            }
        } else {
            Self {
                inner: Box::new(InnerError::Io(value)),
            }
        }
    }
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Self {
            inner: Box::new(InnerError::Message(msg.to_string())),
        }
    }
}

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Self {
            inner: Box::new(InnerError::Message(msg.to_string())),
        }
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self.inner.as_ref() {
            InnerError::Message(msg) => formatter.write_str(msg),
            InnerError::Io(error) => write!(formatter, "{error}"),
            InnerError::Eof => formatter.write_str("unexpected end of input"),
            InnerError::Fmt => formatter.write_str("failed to write to writer"),
        }
    }
}

impl std::error::Error for Error {}
