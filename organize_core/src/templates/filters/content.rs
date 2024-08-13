use serde::Deserialize;
use std::{collections::HashMap, path::PathBuf};
use tera::{to_value, Value};

pub fn file_content(value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
	let value = PathBuf::deserialize(value)?;
	let mime = mime_guess::from_path(&value).first_or_text_plain();

	if mime.type_() == "text" {
		let content = std::fs::read_to_string(value).map_err(tera::Error::msg)?;
		return Ok(to_value(content)?);
	}

	return Err(tera::Error::msg("file content not available for this type of file"));
}
