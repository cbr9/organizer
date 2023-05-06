use std::{
	path::{Path, PathBuf},
	process::{Command, Output, Stdio},
	result,
	str::FromStr,
};

use colored::Colorize;
use log::info;
use serde::{de::Error, Deserialize, Deserializer};
use tempfile;

use crate::{
	data::{
		actions::{Act, ActionType, AsAction},
		filters::AsFilter,
	},
	string::{deserialize_placeholder_string, ExpandPlaceholder},
};
use anyhow::Result;

#[derive(Deserialize, Debug, Clone, Default, Eq, PartialEq)]
pub struct Script {
	#[serde(deserialize_with = "deserialize_exec")]
	exec: String,
	#[serde(deserialize_with = "deserialize_placeholder_string")]
	content: String,
}

impl Act for Script {
	fn act<T, P>(&self, _from: T, _to: Option<P>) -> Result<Option<PathBuf>>
	where
		Self: Sized,
		T: AsRef<Path> + Into<PathBuf>,
		P: AsRef<Path> + Into<PathBuf>,
	{
		unimplemented!()
	}
}

impl AsAction for Script {
	fn process<T: Into<PathBuf>>(&self, path: T) -> Option<PathBuf> {
		let path = path.into();
		self.run(&path)
			.map(|output| {
				info!("({}) run script on {}", self.exec.bold(), path.display());
				let output = String::from_utf8_lossy(&output.stdout);
				output.lines().last().map(|last| PathBuf::from(&last.trim()))
			})
			.ok()?
	}

	fn ty(&self) -> ActionType {
		ActionType::Script
	}
}

fn deserialize_exec<'de, D>(deserializer: D) -> result::Result<String, D::Error>
where
	D: Deserializer<'de>,
{
	let str = String::deserialize(deserializer)?;
	std::process::Command::new(&str)
		.spawn()
		.map(|mut child| {
			child.kill().ok();
			str
		})
		.map_err(D::Error::custom)
}

impl AsFilter for Script {
	fn matches<T: AsRef<Path>>(&self, path: T) -> bool {
		self.run(path)
			.map(|output| {
				// get the last line in stdout and parse it as a boolean
				// if it can't be parsed, return false
				let out = String::from_utf8_lossy(&output.stdout);
				out.lines().last().map(|last| {
					let last = last.trim().to_lowercase();
					bool::from_str(&last).unwrap_or_default()
				})
			})
			.ok()
			.flatten()
			.unwrap_or_default()
	}
}

impl Script {
	pub fn new<T: Into<String>>(exec: T, content: T) -> Self {
		Self {
			exec: exec.into(),
			content: content.into(),
		}
	}

	fn write(&self, path: &Path) -> anyhow::Result<PathBuf> {
		let script = tempfile::NamedTempFile::new()?;
		let script_path = script.into_temp_path().to_path_buf();
		let content = self.content.as_str().expand_placeholders(path)?.into_string();
		if let Ok(content) = content {
			std::fs::write(&script_path, content)?;
		}
		Ok(script_path)
	}

	fn run<T: AsRef<Path>>(&self, path: T) -> anyhow::Result<Output> {
		let script = self.write(path.as_ref())?;
		let output = Command::new(&self.exec)
			.arg(&script)
			.stdout(Stdio::piped())
			.spawn()?
			.wait_with_output()?;
		Ok(output)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_script_filter() {
		let content = "print('huh')\nprint('{path}'.islower())";
		let mut script = Script::new("python", content);
		let path = "/home";
		script.run(path).unwrap_or_else(|_| {
			// some linux distributions don't have a `python` executable, but a `python3`
			script = Script::new("python3", content);
			script.run(path).unwrap()
		});
		assert!(script.matches(&path))
	}
}
