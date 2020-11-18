use crate::register::Register;
use serde::{Deserialize, Deserializer};

impl<'de> Deserialize<'de> for Register {
	// the derived version of deserialize doesn't seem to handle flattened vectors correctly
	// so a custom implementation is needed
	fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
	where
		D: Deserializer<'de>,
	{
		let reg = Register {
			path: Default::default(),
			sections: Vec::deserialize(deserializer)?,
		};
		Ok(reg)
	}
}
