use super::*;
use serde::{
	de::{Error, Visitor},
	Deserialize,
	Deserializer,
};
use std::fmt;

impl<'de> Deserialize<'de> for Recursive {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		struct RecursiveVisitor;
		impl<'de> Visitor<'de> for RecursiveVisitor {
			type Value = Recursive;

			fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
				formatter.write_str("bool or u16")
			}

			fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
			where
				E: Error,
			{
                let enabled = match v {
					true => RecursiveMode::Recursive,
					false => RecursiveMode::NonRecursive
				};
				Ok(Recursive {
					enabled: Some(enabled),
					depth: None,
				})
			}

			fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
			where
				E: Error,
			{
				let enabled = match v {
					0 => RecursiveMode::Recursive,
					1 => RecursiveMode::NonRecursive,
					_ => RecursiveMode::Recursive,
				};
				Ok(Recursive {
					enabled: Some(enabled),
					depth: Some(v as u16),
				})
			}
		}
		deserializer.deserialize_any(RecursiveVisitor)
	}
}
