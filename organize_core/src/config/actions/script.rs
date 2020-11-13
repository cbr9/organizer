use std::{
	borrow::Cow,
	fs,
	io::Result,
	ops::Deref,
	path::{Path, PathBuf},
	process::{Command, Output, Stdio},
	result,
	str::FromStr,
};

use colored::Colorize;
use log::info;
use serde::{de::Error, Deserialize, Deserializer};

use crate::{
	config::{AsAction, AsFilter, UserConfig},
	string::{deserialize_placeholder_string, Placeholder},
};

#[derive(Deserialize, Debug, Clone, Default)]
pub struct Script {
	#[serde(deserialize_with = "deserialize_exec")]
	exec: String,
	#[serde(deserialize_with = "deserialize_placeholder_string")]
	content: String,
}

impl AsAction<Self> for Script {
	fn act<'a>(&self, path: Cow<'a, Path>) -> Result<Cow<'a, Path>> {
		match self.run(&path) {
			Ok(_output) => {
				// improve output
				info!("({}) run script on {}", self.exec.bold(), path.display());
				Ok(path)
			}
			Err(e) => Err(e),
		}
	}
}

fn deserialize_exec<'de, D>(deserializer: D) -> result::Result<String, D::Error>
where
	D: Deserializer<'de>,
{
	let str = String::deserialize(deserializer)?;
	let mut command = std::process::Command::new(&str);
	match command.spawn() {
		Ok(mut child) => {
			child.kill().unwrap_or_else(|_| ());
			Ok(str)
		}
		Err(_) => Err(D::Error::custom(format!("interpreter '{}' could not be run", str))),
	}
}

impl AsFilter for Script {
	fn matches(&self, path: &Path) -> bool {
		let out = self.run(path);
		out.map(|out| {
			let out = String::from_utf8_lossy(&out.stdout);
			out.lines()
				.last()
				.map(|last| bool::from_str(&last.to_lowercase().trim()).unwrap_or_default())
		})
		.map(|x| x.unwrap_or_default())
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

	fn write(&self, path: &Path) -> Result<PathBuf> {
		let content = self.content.as_str();
		let content = content.expand_placeholders(path)?;
		let dir = UserConfig::default_dir().join("scripts");
		if !dir.exists() {
			fs::create_dir_all(&dir)?;
		}
		let script = dir.join("temp_script");
		fs::write(&script, content.deref())?;
		Ok(script)
	}

	fn run(&self, path: &Path) -> Result<Output> {
		let script = self.write(path)?;
		let output = Command::new(&self.exec)
			.arg(&script)
			.stdout(Stdio::piped())
			.spawn()
			.expect("could not run script")
			.wait_with_output()
			.expect("script terminated with an error");
		Ok(output)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	#[cfg(not(target_os = "windows"))] // python doesn't come installed by default on windows
	fn test_script_filter() {
		let script = Script::new("python", "print('huh')\nprint('{path}'.islower())");
		let path = Path::new("/home");
		assert!(script.matches(path))
	}
}
