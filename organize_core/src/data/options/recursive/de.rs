use super::*;
use serde::{Deserialize, Deserializer};
use serde::de::{Visitor, Error};
use std::fmt;

impl<'de> Deserialize<'de> for Recursive {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>
    {
        struct RecursiveVisitor;
        impl<'de> Visitor<'de> for RecursiveVisitor {
            type Value = Recursive;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("bool or u16")
            }

            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: Error
            {
                Ok(Recursive {
                    enabled: Some(v),
                    depth: None
                })
            }

            fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
            where
                E: Error
            {
                if v <= 0 {
                    return Err(E::custom("depth must be greater than zero"))
                }
                Ok(Recursive {
                    enabled: Some(true),
                    depth: Some(v)
                })
            }
        }
        deserializer.deserialize_any(RecursiveVisitor)
    }
}

