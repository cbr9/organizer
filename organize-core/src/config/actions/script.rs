use std::{
	path::PathBuf,
	process::{Command, Output, Stdio},
	str::FromStr,
};

use crate::config::{actions::common::enabled, context::ExecutionContext};
use serde::{Deserialize, Serialize};
use tempfile;

use crate::{config::filters::Filter, resource::Resource, templates::template::Template};
use anyhow::{bail, Result};

use super::{Action, ExecutionModel};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Script {
	exec: String,
	#[serde(default)]
	args: Vec<String>,
	content: Template,
	#[serde(default = "enabled")]
	enabled: bool,
	#[serde(default)]
	parallel: bool,
}

#[typetag::serde(name = "script")]
impl Action for Script {
	fn templates(&self) -> Vec<&Template> {
		Filter::templates(self)
	}

	fn execution_model(&self) -> ExecutionModel {
		if self.parallel {
			ExecutionModel::Parallel
		} else {
			ExecutionModel::Linear
		}
	}

	#[tracing::instrument(ret(level = "info"), err(Debug), level = "debug", skip(ctx))]
	fn execute(&self, res: &Resource, ctx: &ExecutionContext) -> Result<Option<PathBuf>> {
		if ctx.settings.dry_run {
			bail!("Cannot run scripted actions during a dry run")
		}
		if self.enabled {
			return self.run_script(res, ctx).map(|output| {
				let output = String::from_utf8_lossy(&output.stdout);
				output.lines().last().map(|last| PathBuf::from(&last.trim()))
			});
		}
		Ok(None)
	}
}

#[typetag::serde(name = "script")]
impl Filter for Script {
	fn templates(&self) -> Vec<&Template> {
		vec![&self.content]
	}

	#[tracing::instrument(ret, level = "debug", skip(ctx))]
	fn filter(&self, res: &Resource, ctx: &ExecutionContext) -> bool {
		self.run_script(res, ctx)
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
			enabled: true,
			parallel: false,
		}
	}

	fn write(&self, res: &Resource, ctx: &ExecutionContext) -> anyhow::Result<PathBuf> {
		let script = tempfile::NamedTempFile::new()?;
		let script_path = script.into_temp_path().to_path_buf();
		let context = ctx.services.template_engine.new_context(res);
		if let Some(content) = ctx.services.template_engine.render(&self.content, &context)? {
			std::fs::write(&script_path, content)?;
		}
		Ok(script_path)
	}

	fn run_script(&self, res: &Resource, ctx: &ExecutionContext) -> anyhow::Result<Output> {
		let script = self.write(res, ctx)?;
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
	use crate::{config::context::ContextHarness, templates::TemplateEngine};

	use super::*;

	#[test]
	fn test_script_filter() -> Result<()> {
		let src = Resource::new("/home", "/").unwrap();
		let content = String::from("print('huh')\nprint('{{path}}'.islower())");
		let mut script = Script::new("python", content.clone());
		let mut template_engine = TemplateEngine::default();
		template_engine.add_templates(&Filter::templates(&script))?;
		let mut harness = ContextHarness::new();
		harness.services.template_engine = template_engine;
		let ctx = harness.context();

		script.run_script(&src, &ctx).unwrap_or_else(|_| {
			// some linux distributions don't have a `python` executable, but a `python3`
			script = Script::new("python3", content);
			script.run_script(&src, &ctx).unwrap()
		});
		assert!(script.filter(&src, &ctx));
		Ok(())
	}
}
