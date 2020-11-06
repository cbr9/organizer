use crate::user_config::rules::actions::{ActionType, AsAction};
use colored::Colorize;
use log::info;
use serde::Deserialize;
use std::{
	borrow::Cow,
	io::{Error, ErrorKind, Result},
	ops::Deref,
	path::Path,
};

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Trash(bool);

impl Deref for Trash {
	type Target = bool;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl AsAction<Self> for Trash {
	fn act<'a>(&self, path: Cow<'a, Path>) -> Result<Cow<'a, Path>> {
		if self.0 {
			return match trash::delete(&path) {
				Ok(_) => {
					info!("({}) {}", ActionType::Trash.to_string().bold(), path.display());
					Ok(path)
				}
				Err(e) => Err(Error::new(ErrorKind::Other, e.to_string())),
			};
		}
		Ok(path)
	}
}
