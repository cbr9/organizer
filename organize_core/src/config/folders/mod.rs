use std::path::PathBuf;

use serde::{Deserialize, Deserializer};
use tera::Tera;

use crate::{
	config::options::FolderOptions,
	path::{get_env_context, Expand},
};

#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(deny_unknown_fields)]
pub struct Folder {
	#[serde(deserialize_with = "deserialize_template_folder")]
	pub path: PathBuf,
	#[serde(flatten, default = "FolderOptions::default_none")]
	pub options: FolderOptions,
	#[serde(default)]
	pub interactive: bool,
}

fn deserialize_template_folder<'de, D>(deserializer: D) -> Result<PathBuf, D::Error>
where
	D: Deserializer<'de>,
{
	// Deserialize as a Vec<String>
	let str: String = String::deserialize(deserializer)?;
	let context = get_env_context();
	let tera = Tera::one_off(&str, &context, false).map_err(serde::de::Error::custom)?;
	PathBuf::from(tera)
		.canonicalize()
		.map(|path| path.expand_user())
		.map_err(|e| serde::de::Error::custom(e.to_string()))
}

pub type Folders = Vec<Folder>;
