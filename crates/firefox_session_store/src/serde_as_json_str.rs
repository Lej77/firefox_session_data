//! Some data that is serialized as strings but actually represents some JSON data.
//!
//! This makes it easier to serialize and deserialize such data.
#![allow(dead_code)]

use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;
pub mod wrapper {
    //! A value can be wrapped in one of these types to be serialized as a JSON string.

    use super::*;
    use std::ops::{Deref, DerefMut};

    /// The type `T` should be serialized as a JSON string and deserialization should always provide a JSON
    /// string that can be deserialized to the type `T`.
    #[derive(Debug, Default, Clone, PartialEq, Eq)]
    pub struct JSONString<T>(pub T);
    impl<T> JSONString<T> {
        pub fn into_inner(self) -> T {
            self.0
        }
    }
    impl<T> Deref for JSONString<T> {
        type Target = T;
        fn deref(&self) -> &T {
            &self.0
        }
    }
    impl<T> DerefMut for JSONString<T> {
        fn deref_mut(&mut self) -> &mut T {
            &mut self.0
        }
    }

    // Serialize via the module methods:
    impl<T> Serialize for JSONString<T>
    where
        T: Serialize,
    {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serialize(self, serializer)
        }
    }
    impl<'de, T> Deserialize<'de> for JSONString<T>
    where
        T: for<'a> Deserialize<'a>,
    {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserialize(deserializer)
        }
    }

    // Describe how this type should be used by the module methods:
    impl<T> InnerSerializableData<T> for JSONString<T>
    where
        T: Serialize,
    {
        fn get_inner_data(&self) -> Result<&T, Option<&str>> {
            Ok(self)
        }
    }
    impl<T> InnerDeserializableData<T> for JSONString<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        fn from_string<'de, D: Deserializer<'de>>(text: String) -> Result<Self, D::Error> {
            Err(D::Error::invalid_value(
                // Unexpected:
                serde::de::Unexpected::Str(&text),
                // Expected:
                &"a string that can be deserialized to a specific JSON value",
            ))
        }
        fn from_data(data: T) -> Self {
            Self(data)
        }
    }

    /// The type `T` should be serialized as a JSON string but the deserialized string might not be a `T`.
    ///
    /// Note that if this type isn't wrapped in an a option then it can cause deserialization to fail if the
    /// provided data can't be deserialized as a string.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum FallibleJSONString<T> {
        Text(String),
        Data(T),
    }
    impl<T> FallibleJSONString<T> {
        pub fn get_data(self) -> Result<T, Self> {
            if let FallibleJSONString::Data(value) = self {
                Ok(value)
            } else {
                Err(self)
            }
        }
    }

    // Serialize via the module methods:
    impl<T> Serialize for FallibleJSONString<T>
    where
        T: Serialize,
    {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serialize(self, serializer)
        }
    }
    impl<'de, T> Deserialize<'de> for FallibleJSONString<T>
    where
        T: for<'a> Deserialize<'a>,
    {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserialize(deserializer)
        }
    }

    // Describe how this type should be used by the module methods:
    impl<T> InnerSerializableData<T> for FallibleJSONString<T>
    where
        T: Serialize,
    {
        fn get_inner_data(&self) -> Result<&T, Option<&str>> {
            match self {
                FallibleJSONString::Text(text) => Err(Some(text)),
                FallibleJSONString::Data(data) => Ok(data),
            }
        }
    }
    impl<T> InnerDeserializableData<T> for FallibleJSONString<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        fn from_string<'de, D: Deserializer<'de>>(text: String) -> Result<Self, D::Error> {
            Ok(FallibleJSONString::Text(text.to_owned()))
        }
        fn from_data(data: T) -> Self {
            FallibleJSONString::Data(data)
        }
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

    impl<T> InnerSerializableData<T> for Option<T>
    where
        T: Serialize,
    {
        fn get_inner_data(&self) -> Result<&T, Option<&str>> {
            match self {
                Some(v) => Ok(v),
                None => Err(None),
            }
        }
    }
    impl<T> InnerDeserializableData<T> for Option<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        fn get_string<'de, D>(deserializer: D) -> Result<Result<String, Self>, D::Error>
        where
            D: Deserializer<'de>,
        {
            // Deserialize to `None` if the provided data couldn't be parsed as a string.
            let text: Option<String> = Deserialize::deserialize(deserializer)?;
            Ok(text.ok_or(None))
        }

        fn from_string<'de, D: Deserializer<'de>>(_: String) -> Result<Self, D::Error> {
            Ok(None)
        }

        fn from_data(data: T) -> Self {
            Some(data)
        }
    }
}

pub trait InnerSerializableData<T>
where
    T: Serialize,
{
    /// Get the data that should be serialized to JSON. Alternatively a string that should be serialized directly or nothing to not serialize anything.
    fn get_inner_data(&self) -> Result<&T, Option<&str>>;
}
pub trait InnerDeserializableData<T>: Sized
where
    T: for<'de> Deserialize<'de>,
{
    /// Determines how the string is deserialized. Can optionally return Self if it could be directly deserialized.
    ///
    /// Default implementation returns an error if a string can't be deserialized.
    fn get_string<'de, D>(deserializer: D) -> Result<Result<String, Self>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Ok(Deserialize::deserialize(deserializer)?))
    }

    /// Create `Self` from a JSON text.
    fn from_string<'de, D: Deserializer<'de>>(text: String) -> Result<Self, D::Error>;
    /// Create `Self` from a value that was deserialized from a JSON string.
    fn from_data(data: T) -> Self;
}

pub fn serialize<T, TI, S>(data: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: InnerSerializableData<TI>,
    TI: Serialize,
    S: Serializer,
{
    let text = match data.get_inner_data() {
        Ok(data) => serde_json::to_string(&data).ok().map(Cow::from),
        Err(maybe_text) => maybe_text.map(Cow::from),
    };
    text.serialize(serializer)
}
pub fn deserialize<'de, T, TI, D>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: InnerDeserializableData<TI>,
    TI: for<'a> Deserialize<'a>,
{
    match T::get_string(deserializer)? {
        Ok(text) => match serde_json::from_str::<TI>(&text) {
            Ok(data) => Ok(T::from_data(data)),
            Err(_) => Ok(T::from_string::<D>(text)?),
        },
        Err(value) => Ok(value),
    }
}
