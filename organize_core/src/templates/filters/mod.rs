use serde::Deserialize;
use std::{collections::HashMap, path::PathBuf};
use tera::{to_value, Result, Value};

pub struct Parent;

impl tera::Filter for Parent {
	fn filter(&self, value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
		let value = String::deserialize(value).unwrap();
		let path = PathBuf::from(value);
		let parent = match path.parent() {
			Some(p) => p,
			None => return Err("No parent found for the given path".into()),
		};
		Ok(to_value(parent)?)
	}
}

pub struct Stem;

impl tera::Filter for Stem {
	fn filter(&self, value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
		let value = String::deserialize(value).unwrap();
		let path = PathBuf::from(value);
		let parent = match path.file_stem().and_then(|f| f.to_str()) {
			Some(p) => p,
			None => return Err("No filestem found for the given path".into()),
		};
		Ok(to_value(parent)?)
	}
}

pub struct Filename;

impl tera::Filter for Filename {
	fn filter(&self, value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
		let value = String::deserialize(value).unwrap();
		let path = PathBuf::from(value);
		let parent = match path.file_name().and_then(|f| f.to_str()) {
			Some(p) => p,
			None => return Err("No filename found for the given path".into()),
		};
		Ok(to_value(parent)?)
	}
}

pub struct Extension;

impl tera::Filter for Extension {
	fn filter(&self, value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
		let value = String::deserialize(value).unwrap();
		let path = PathBuf::from(value);
		let parent = match path.extension().and_then(|f| f.to_str()) {
			Some(p) => p,
			None => return Err("No extension found for the given path".into()),
		};
		Ok(to_value(parent)?)
	}
}

pub struct Mime;

impl tera::Filter for Mime {
	fn filter(&self, value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
		let value = String::deserialize(value).unwrap();
		let path = PathBuf::from(value);
		let mime = mime_guess::from_path(path).first_or_octet_stream().to_string();
		Ok(to_value(mime)?)
	}
}
