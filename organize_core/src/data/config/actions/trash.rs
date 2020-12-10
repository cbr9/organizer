use std::{
	borrow::Cow,
	io::{Error, ErrorKind, Result},
	ops::Deref,
	path::Path,
};

use crate::data::config::actions::{ActionType, AsAction};
use colored::Colorize;
use log::info;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default, Eq, PartialEq)]
pub struct Trash(bool);

impl Deref for Trash {
	type Target = bool;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl AsAction<Self> for Trash {
	fn act<'a>(&self, path: Cow<'a, Path>, simulate: bool) -> Result<Cow<'a, Path>> {
		if self.0 {
			if !simulate {
				return match trash::delete(&path) {
					Ok(_) => {
						info!("({}) {}", ActionType::Trash.to_string().bold(), path.display());
						Ok(path)
					}
					Err(e) => Err(Error::new(ErrorKind::Other, e.to_string())),
				};
			} else {
				info!("({}) {}", ActionType::Trash.to_string().bold(), path.display());
			}
		}
		Ok(path)
	}
}
