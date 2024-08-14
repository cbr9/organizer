use std::{collections::HashMap, path::PathBuf};

use serde::Deserialize;
use tera::{to_value, Result, Value};

pub fn parent(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
	let value = PathBuf::deserialize(value).unwrap();
	let parent = match value.parent() {
		Some(p) => p,
		None => return Err(format!("No parent found for path {}", value.display()).into()),
	};
	Ok(to_value(parent)?)
}

pub fn stem(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
	let value = PathBuf::deserialize(value).unwrap();
	let parent = match value.file_stem().and_then(|f| f.to_str()) {
		Some(p) => p,
		None => return Err(format!("No stem found for path {}", value.display()).into()),
	};
	Ok(to_value(parent)?)
}

pub fn filename(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
	let value = PathBuf::deserialize(value).unwrap();
	let parent = match value.file_name().and_then(|f| f.to_str()) {
		Some(p) => p,
		None => return Err(format!("No filename found for path {}", value.display()).into()),
	};
	Ok(to_value(parent)?)
}

pub fn extension(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
	let value = PathBuf::deserialize(value).unwrap();
	let parent = match value.extension().and_then(|f| f.to_str()) {
		Some(p) => p,
		None => return Err(tera::Error::msg(format!("No extension found for path {}", value.display()))),
	};
	Ok(to_value(parent)?)
}
