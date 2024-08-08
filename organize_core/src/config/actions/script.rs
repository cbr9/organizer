use std::{
	path::{Path, PathBuf},
	process::{Command, Output, Stdio},
	str::FromStr,
};

use serde::Deserialize;
use tempfile;

use crate::{
	config::filters::AsFilter,
	resource::Resource,
	templates::{Template},
};
use anyhow::{bail, Result};

use super::AsAction;

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Script {
	exec: String,
	#[serde(default)]
	args: Vec<String>,
	content: Template,
}

pub struct ActionConfig<'a> {
	pub requires_dest: bool,
	pub log_hint: &'a str,
}

impl<'a> AsAction<'a> for Script {
	const CONFIG: ActionConfig<'a> = ActionConfig {
		requires_dest: true,
		log_hint: "SCRIPT",
	};

	fn execute<T: AsRef<Path>>(&self, src: &Resource, _: Option<T>, dry_run: bool) -> Result<Option<PathBuf>> {
		if dry_run {
			bail!("Cannot run scripted actions during a dry run")
		}
		self.run_script(src).map(|output| {
			let output = String::from_utf8_lossy(&output.stdout);
			output.lines().last().map(|last| PathBuf::from(&last.trim()))
		})
	}
}

impl AsFilter for Script {
	fn matches(&self, res: &Resource) -> bool {
		self.run_script(res)
			.map(|output| {
				// get the last line in stdout and parse it as a boolean
				// if it can't be parsed, return false
				let out = String::from_utf8_lossy(&output.stdout);
				out.lines().last().map(|last| {
					let last = last.trim().to_lowercase();
					bool::from_str(&last).expect("Filter script did not output a valid boolean to stdout")
				})
			})
			.ok()
			.flatten()
			.unwrap_or_default()
	}
}

impl Script {
	pub fn new<T: Into<String>, C: Into<Template>>(exec: T, content: C) -> Self {
		Self {
			exec: exec.into(),
			content: content.into(),
			args: vec![],
		}
	}

	fn write(&self, src: &Resource) -> anyhow::Result<PathBuf> {
		let script = tempfile::NamedTempFile::new()?;
		let script_path = script.into_temp_path().to_path_buf();
		let content = self.content.render(&src.context)?;
		std::fs::write(&script_path, content)?;
		Ok(script_path)
	}

	fn run_script(&self, src: &Resource) -> anyhow::Result<Output> {
		let script = self.write(src)?;
		let output = Command::new(&self.exec)
			.args(self.args.as_slice())
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
		let src = Resource::new("/home", "/", &[]);
		let content = String::from("print('huh')\nprint('{{path}}'.islower())");
		let mut script = Script::new("python", content.clone());
		script.run_script(&src).unwrap_or_else(|_| {
			// some linux distributions don't have a `python` executable, but a `python3`
			script = Script::new("python3", content);
			script.run_script(&src).unwrap()
		});
		assert!(script.matches(&src))
	}
}
