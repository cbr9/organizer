use std::{collections::HashMap, path::PathBuf};

use serde::Deserialize;
use tera::{to_value, Result, Value};

struct Size {
	bytes: f64,
	format: SizeUnit,
}

#[derive(Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum SizeUnit {
	Bytes,
	KiB,
	MiB,
	GiB,
	TiB,
	PiB,
	EiB,
	ZiB,
	YiB,
	KB,
	MB,
	GB,
	TB,
	PB,
	EB,
	ZB,
	YB,
}

impl SizeUnit {
	pub fn unit_value(&self) -> f64 {
		match self {
			SizeUnit::Bytes => 1.0,
			SizeUnit::KiB => 1024.0_f64.powi(1),
			SizeUnit::MiB => 1024.0_f64.powi(2),
			SizeUnit::GiB => 1024.0_f64.powi(3),
			SizeUnit::TiB => 1024.0_f64.powi(4),
			SizeUnit::PiB => 1024.0_f64.powi(5),
			SizeUnit::EiB => 1024.0_f64.powi(6),
			SizeUnit::ZiB => 1024.0_f64.powi(7),
			SizeUnit::YiB => 1024.0_f64.powi(8),
			SizeUnit::KB => 1000.0_f64.powi(1),
			SizeUnit::MB => 1000.0_f64.powi(2),
			SizeUnit::GB => 1000.0_f64.powi(3),
			SizeUnit::TB => 1000.0_f64.powi(4),
			SizeUnit::PB => 1000.0_f64.powi(5),
			SizeUnit::EB => 1000.0_f64.powi(6),
			SizeUnit::ZB => 1000.0_f64.powi(7),
			SizeUnit::YB => 1000.0_f64.powi(8),
		}
	}
}

impl Size {
	fn format(&self) -> f64 {
		self.bytes / self.format.unit_value()
	}
}

pub fn size(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
	let value = PathBuf::deserialize(value)?;

	let unit = match args.get("unit") {
		Some(unit) => SizeUnit::deserialize(unit).map_err(tera::Error::msg)?,
		None => SizeUnit::Bytes,
	};
	let metadata = std::fs::metadata(value)?;
	let size = Size {
		bytes: metadata.len() as f64,
		format: unit,
	};

	Ok(to_value(size.format())?)
}
