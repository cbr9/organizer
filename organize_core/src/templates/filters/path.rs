use std::{collections::HashMap, path::PathBuf};

use serde::Deserialize;
use tera::{to_value, Result, Value};

pub fn parent(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
	let value = String::deserialize(value).unwrap();
	let path = PathBuf::from(value);
	let parent = match path.parent() {
		Some(p) => p,
		None => return Err(format!("No parent found for path {}", path.display()).into()),
	};
	Ok(to_value(parent)?)
}

pub fn stem(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
	let value = String::deserialize(value).unwrap();
	let path = PathBuf::from(value);
	let parent = match path.file_stem().and_then(|f| f.to_str()) {
		Some(p) => p,
		None => return Err(format!("No stem found for path {}", path.display()).into()),
	};
	Ok(to_value(parent)?)
}

pub fn filename(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
	let value = String::deserialize(value).unwrap();
	let path = PathBuf::from(value);
	let parent = match path.file_name().and_then(|f| f.to_str()) {
		Some(p) => p,
		None => return Err(format!("No filename found for path {}", path.display()).into()),
	};
	Ok(to_value(parent)?)
}

pub fn extension(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
	let value = String::deserialize(value).unwrap();
	let path = PathBuf::from(value);
	let parent = match path.extension().and_then(|f| f.to_str()) {
		Some(p) => p,
		None => return Err(tera::Error::msg(format!("No extension found for path {}", path.display()))),
	};
	Ok(to_value(parent)?)
}
