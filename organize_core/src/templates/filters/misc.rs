use std::{collections::HashMap, path::PathBuf};

use serde::Deserialize;
use tera::{to_value, Result, Value};

pub fn mime(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
	let value = String::deserialize(value).unwrap();
	let path = PathBuf::from(value);
	let mime = mime_guess::from_path(path).first_or_octet_stream().to_string();
	Ok(to_value(mime)?)
}

pub fn hash(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
	let value = PathBuf::deserialize(value)?;
	let bytes = std::fs::read(value)?; // Vec<u8>
	let hash = sha256::digest(&bytes);
	Ok(to_value(hash)?)
}
