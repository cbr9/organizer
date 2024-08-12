use serde::Deserialize;
use std::{collections::HashMap, path::PathBuf, str::FromStr};
use strum::{EnumString, VariantNames};
use tera::{to_value, try_get_value, Result, Value};

struct Size {
	bytes: u64,
	format: SizeFormat,
}

#[derive(EnumString, strum::VariantNames, PartialEq)]
#[strum(serialize_all = "lowercase")]
enum SizeFormat {
	Bytes,
	Decimal,
	Binary,
}

impl SizeFormat {
	const BINARY_UNITS: [&'static str; 8] = ["KiB", "MiB", "GiB", "TiB", "PiB", "EiB", "ZiB", "YiB"];
	const DECIMAL_UNITS: [&'static str; 8] = ["KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];

	pub fn base(&self) -> u16 {
		match self {
			SizeFormat::Bytes => 0,
			SizeFormat::Decimal => 1000,
			SizeFormat::Binary => 1024,
		}
	}
}

impl Size {
	fn format(&self) -> String {
		let base = self.format.base() as u64;
		if self.format == SizeFormat::Bytes || self.bytes < base {
			return self.bytes.to_string();
		}

		let units = match self.format {
			SizeFormat::Bytes => unreachable!(),
			SizeFormat::Decimal => SizeFormat::DECIMAL_UNITS,
			SizeFormat::Binary => SizeFormat::BINARY_UNITS,
		};

		let (unit, suffix) = units
			.iter()
			.enumerate()
			.find_map(|(i, suffix)| {
				let unit = base.pow((i + 2) as u32);
				if self.bytes < unit {
					return Some((unit, suffix));
				}
				None
			})
			.unwrap();

		return format!("{} {}", (base * self.bytes) / unit, suffix);
	}
}

pub fn size(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
	let value = PathBuf::deserialize(value).unwrap();
	let unit = match args.get("unit") {
		Some(unit) => SizeFormat::from_str(&unit.to_string()).map_err(tera::Error::msg)?,
		None => SizeFormat::Binary,
	};
	let metadata = std::fs::metadata(value)?;
	let size = Size {
		bytes: metadata.len(),
		format: unit,
	};

	Ok(to_value(size.format())?)
}

pub struct Parent;

impl tera::Filter for Parent {
	fn filter(&self, value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
		let value = String::deserialize(value).unwrap();
		let path = PathBuf::from(value);
		let parent = match path.parent() {
			Some(p) => p,
			None => return Err(format!("No parent found for path {}", path.display()).into()),
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
			None => return Err(format!("No stem found for path {}", path.display()).into()),
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
			None => return Err(format!("No filename found for path {}", path.display()).into()),
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
			None => return Err(tera::Error::msg(format!("No extension found for path {}", path.display()))),
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
