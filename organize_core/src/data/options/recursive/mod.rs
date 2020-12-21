mod de;

use crate::utils::DefaultOpt;
use serde::Serialize;

#[derive(Serialize, Debug, Clone, Eq, PartialEq)]
pub struct Recursive {
	pub enabled: Option<bool>,
	pub depth: Option<u16>, // if depth is some, enabled should be true
}

impl DefaultOpt for Recursive {
	fn default_none() -> Self {
		Self {
			enabled: Some(false),
			depth: None,
		}
	}

	fn default_some() -> Self {
		Self {
			enabled: Some(false),
			depth: Some(1),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn test_validity() {
		let some = Recursive::default_some();
		assert!(some.depth.is_some() && some.enabled.is_some() && some.enabled.unwrap());
		let none = Recursive::default_none();
		assert!(none.depth.is_none() && none.enabled.is_some() && !none.enabled.unwrap())
	}
}
