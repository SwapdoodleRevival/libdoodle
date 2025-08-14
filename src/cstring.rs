use std::{ffi::{self}, fmt};

use serde::{de::{self, Visitor}, Deserializer, Serializer};


pub type CString = ffi::CString;

pub fn serialize<S>(string: &CString, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&string.to_string_lossy())
}


struct CStringVisitor;

impl Visitor<'_> for CStringVisitor {
    type Value = CString;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match CString::new(value) {
            Ok(v) => Ok(v),
            Err(_) => Err(E::custom("a"))
        }
    }
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<CString, D::Error>
where
    D: Deserializer<'de>
{
    deserializer.deserialize_str(CStringVisitor)
}
