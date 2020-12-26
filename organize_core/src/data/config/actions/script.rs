use std::{
	fs,
	ops::Deref,
	path::{Path, PathBuf},
	process::{Command, Output, Stdio},
	result,
	str::FromStr,
};

use colored::Colorize;
use log::info;
use serde::{de::Error, Deserialize, Deserializer};

use crate::data::config::actions::ActionType;
use crate::{
	data::config::{actions::AsAction, filters::AsFilter, Config},
	string::{deserialize_placeholder_string, Placeholder},
};

#[derive(Deserialize, Debug, Clone, Default, Eq, PartialEq)]
pub struct Script {
	#[serde(deserialize_with = "deserialize_exec")]
	exec: String,
	#[serde(deserialize_with = "deserialize_placeholder_string")]
	content: String,
}

impl AsAction for Script {
	fn act<T: Into<PathBuf>>(&self, path: T, simulate: bool) -> Option<PathBuf> {
		let path = path.into();
		if !simulate {
			self.run(&path)
				.map(|output| {
					info!("({}) run script on {}", self.exec.bold(), path.display());
					let output = String::from_utf8_lossy(&output.stdout);
					output.lines().last().map(|last| PathBuf::from(&last.trim()))
				})
				.ok()?
		} else {
			info!("(simulate {}) run script on {}", self.exec.bold(), path.display());
			None
		}
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
		let out = self.run(path.as_ref());
		out.map(|out| {
			// get the last line in stdout and parse it as a boolean
			// if it can't be parsed, return false
			let out = String::from_utf8_lossy(&out.stdout);
			out.lines()
				.last()
				.map(|last| bool::from_str(&last.to_lowercase().trim()).unwrap_or_default())
		})
		// unwrap the underlying boolean: if there was no line in stdout, return false
		.map(|x| x.unwrap_or_default())
		// unwrap the underlying boolean: if running the script produced an error, return false
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
		let content = self.content.as_str();
		let content = content.expand_placeholders(path)?;
		let dir = Config::default_dir()?.join("scripts");
		if !dir.exists() {
			fs::create_dir_all(&dir)?;
		}
		let script = dir.join("temp_script");
		fs::write(&script, content.deref())?;
		Ok(script)
	}

	fn run<T: AsRef<Path>>(&self, path: T) -> anyhow::Result<Output> {
		let script = self.write(path.as_ref())?;
		let output = Command::new(&self.exec).arg(&script).stdout(Stdio::piped()).spawn()?.wait_with_output()?;
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
