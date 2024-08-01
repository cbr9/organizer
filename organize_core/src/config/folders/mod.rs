use std::path::PathBuf;

use serde::{Deserialize, Deserializer};
use tera::Tera;

use crate::{
	config::options::Options,
	path::{get_context, get_env_context, Expand},
	utils::DefaultOpt,
};
use std::convert::TryFrom;

#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(deny_unknown_fields)]
pub struct Folder {
	#[serde(deserialize_with = "deserialize_template_folder")]
	pub path: PathBuf,
	#[serde(flatten, default)]
	pub options: Options,
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
	PathBuf::from(tera).expand_user().map_err(serde::de::Error::custom)
}

pub type Folders = Vec<Folder>;
