#![allow(dead_code)]

use std::fmt;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ParseAsStr<T>(pub T);
impl<T> ParseAsStr<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}
impl<T> Deref for ParseAsStr<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}
impl<T> DerefMut for ParseAsStr<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

// Serialize via the module methods:
impl<T> Serialize for ParseAsStr<T>
where
    T: fmt::Display,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.to_string().serialize(serializer)
    }
}
impl<'de, T, E> Deserialize<'de> for ParseAsStr<T>
where
    T: FromStr<Err = E>,
    E: fmt::Display,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let text: String = Deserialize::deserialize(deserializer)?;
        T::from_str(&text)
            .map_err(|e| {
                D::Error::invalid_value(
                    // Unexpected:
                    serde::de::Unexpected::Str(&text),
                    // Expected:
                    &CustomParseError(e),
                )
            })
            .map(Self)
    }
}

struct CustomParseError<E: fmt::Display>(E);
impl<E: fmt::Display> serde::de::Expected for CustomParseError<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a string that could be parsed (parse error: {})", self.0)
    }
}

pub mod blanket_impl {
    //! This allows some types to use the parent module via a serde attribute like `#[serde(with = "serde_as_json_str")]`.
    //!
    //! # Option
    //! Deserialization of `Option<T>` via the module function will do the following:
    //!
    //! 1. If no data was provided then return `None`.
    //! 2. If the provided data wasn't a string then return `None`.
    //! 3. If the deserialized string can't be deserialized as a `JSON` string representation of the type `T` then return `None`.
    //! 4. If the deserialized string was deserialized via a `JSON` deserializer to the type `T` then return `Some(T)`.

    use super::*;

    impl<T> InnerSerializableData for Option<T>
    where
        T: fmt::Display,
    {
        fn handle_serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            match self {
                Some(value) => value.to_string().serialize(serializer),
                None => serializer.serialize_none(),
            }
        }
    }
    impl<T, E> InnerDeserializableData for Option<T>
    where
        T: FromStr<Err = E>,
        E: fmt::Display,
    {
        fn get_string<'de, D>(deserializer: D) -> Result<Result<String, Self>, D::Error>
        where
            D: Deserializer<'de>,
        {
            // Deserialize to `None` if the provided data couldn't be parsed as a string.
            let text: Option<String> = Deserialize::deserialize(deserializer)?;
            Ok(text.ok_or(None))
        }

        fn from_string<'de, D: Deserializer<'de>>(text: String) -> Result<Self, D::Error> {
            Ok(T::from_str(&text).ok())
        }
    }
}

pub trait InnerSerializableData {
    fn handle_serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error>;
}
pub trait InnerDeserializableData: Sized {
    /// Determines how the string is deserialized. Can optionally return Self if it could be directly deserialized.
    ///
    /// Default implementation returns an error if a string can't be deserialized.
    fn get_string<'de, D>(deserializer: D) -> Result<Result<String, Self>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Ok(Deserialize::deserialize(deserializer)?))
    }

    /// Create `Self` from a string.
    fn from_string<'de, D: Deserializer<'de>>(text: String) -> Result<Self, D::Error>;
}

pub fn serialize<T, S>(data: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: InnerSerializableData,
    S: Serializer,
{
    data.handle_serialize(serializer)
}
pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: InnerDeserializableData,
{
    match T::get_string(deserializer)? {
        Ok(text) => T::from_string::<D>(text),
        Err(value) => Ok(value),
    }
}
