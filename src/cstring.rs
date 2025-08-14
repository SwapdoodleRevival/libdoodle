use std::{ffi::CString, fmt, str::FromStr};

use serde::{
    Deserializer, Serializer,
    de::{self, Visitor},
};

pub fn serialize<S>(string: &CString, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&string.to_string_lossy())
}

struct CStringVisitor;

impl Visitor<'_> for CStringVisitor {
    type Value = CString;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a string")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        CString::from_str(value)
            .map_err(|_| E::invalid_value(de::Unexpected::Str("invalid CString"), &self))
    }
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<CString, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_str(CStringVisitor)
}
