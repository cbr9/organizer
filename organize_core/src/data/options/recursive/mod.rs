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

