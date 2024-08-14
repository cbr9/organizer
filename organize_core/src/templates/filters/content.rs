use serde::Deserialize;
use std::{collections::HashMap, path::PathBuf};
use tera::{to_value, Value};

pub fn file_content(value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
	let value = PathBuf::deserialize(value)?;
	let mime = mime_guess::from_path(&value).first_or_text_plain();
	let mut content = String::new();

	if mime.type_() == "text" {
		content = std::fs::read_to_string(&value).map_err(tera::Error::msg)?;
	}

	if mime.subtype() == "pdf" {
		let bytes = std::fs::read(&value)?;
		content = pdf_extract::extract_text_from_mem(&bytes).map_err(tera::Error::msg)?;
	}

	Ok(to_value(content)?)
}
