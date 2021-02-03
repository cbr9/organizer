mod de;
mod se;

use crate::utils::DefaultOpt;
use notify::RecursiveMode;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Recursive {
	pub enabled: Option<RecursiveMode>,
	pub depth: Option<u16>, // if depth is some, enabled should be true
}

impl DefaultOpt for Recursive {
	fn default_none() -> Self {
		Self {
			enabled: None,
			depth: None,
		}
	}

	fn default_some() -> Self {
		Self {
			enabled: Some(RecursiveMode::NonRecursive),
			depth: Some(1),
		}
	}
}
