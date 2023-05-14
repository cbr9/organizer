use std::path::{Path, PathBuf};

use derive_more::Deref;
use serde::Deserialize;

use crate::{
	config::actions::{Act, ActionType, AsAction},
	string::{deserialize_placeholder_string, ExpandPlaceholder},
};
use anyhow::Result;

#[derive(Debug, Clone, Deserialize, Deref, Default, Eq, PartialEq)]
pub struct Echo(#[serde(deserialize_with = "deserialize_placeholder_string")] String);

impl Act for Echo {
	fn act<T, P>(&self, from: T, _to: Option<P>) -> Result<Option<PathBuf>>
	where
		T: AsRef<Path> + Into<PathBuf>,
		P: AsRef<Path> + Into<PathBuf>,
	{
		let from = from.into();
		let expanded = self.as_str().expand_placeholders(&from);
		match expanded {
			Ok(str) => {
				log::info!("({}) {:#?}", self.ty().to_string(), str);
				Ok(Some(from))
			}
			Err(e) => {
				log::error!("{:?}", e);
				Ok(None)
			}
		}
	}
}

impl AsAction for Echo {
	fn process<T: Into<PathBuf> + AsRef<Path>>(&self, path: T) -> Option<PathBuf> {
		let path = path.into();
		let to: Option<T> = None;
		self.act(path, to).unwrap()
	}

	fn ty(&self) -> ActionType {
		ActionType::Echo
	}
}
