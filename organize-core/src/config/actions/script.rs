use std::{
	path::PathBuf,
	process::{Command, Output as ProcessOutput, Stdio},
	str::FromStr,
};

use crate::{
	config::{
		actions::{common::enabled, Change, Output},
		context::ExecutionContext,
	},
	errors::{Error, ErrorContext},
	templates::Context,
};
use serde::{Deserialize, Serialize};
use tempfile;

use crate::{config::filters::Filter, templates::template::Template};
use anyhow::Result;

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

	fn execute(&self, ctx: &ExecutionContext) -> Result<Output, Error> {
		if self.enabled {
			let Some(target) = self.run_script(ctx).map(|output| {
				let output = String::from_utf8_lossy(&output.stdout);
				output.lines().last().map(|last| PathBuf::from(&last.trim()))
			})?
			else {
				return Ok(Output::Continue);
			};

			return Ok(Output::Modified(Change {
				before: ctx.scope.resource.path().to_path_buf(),
				after: target.clone(),
				current: target,
			}));
		}
		Ok(Output::Continue)
	}
}

#[typetag::serde(name = "script")]
impl Filter for Script {
	fn templates(&self) -> Vec<&Template> {
		vec![&self.content]
	}

	#[tracing::instrument(ret, level = "debug", skip(ctx))]
	fn filter(&self, ctx: &ExecutionContext) -> bool {
		self.run_script(ctx)
			.map(|output| {
				// get the last line in stdout and parse it as a boolean
				// if it can't be parsed, return false
				let out = String::from_utf8_lossy(&output.stdout);
				out.lines().last().map(|last| {
					let last = last.trim().to_lowercase();
					bool::from_str(&last)
						.inspect_err(|e| tracing::warn!("Filter script did not output a valid boolean to stdout: {}", e))
						.unwrap_or(false)
				})
			})
			.ok()
			.flatten()
			.unwrap_or_default()
	}
}

impl Script {
	pub fn new<T: AsRef<str>, C: AsRef<str>>(exec: T, content: C) -> Result<Self, tera::Error> {
		Ok(Self {
			exec: exec.as_ref().to_string(),
			content: Template::from(content.as_ref()),
			args: vec![],
			enabled: true,
			parallel: false,
		})
	}

	fn write(&self, ctx: &ExecutionContext) -> Result<PathBuf, Error> {
		let script = tempfile::NamedTempFile::new().map_err(|e| Error::Io {
			source: e,
			path: ctx.scope.resource.path().to_path_buf(),
			target: None,
			context: ErrorContext::from_scope(&ctx.scope),
		})?;

		let script_path = script.into_temp_path().to_path_buf();

		let context = Context::new(ctx);

		let maybe_rendered = ctx
			.services
			.templater
			.render(&self.content, &context)
			.map_err(|e| Error::Template {
				source: e,
				template: self.content.clone(),
				context: ErrorContext::from_scope(&ctx.scope),
			})?;

		if let Some(content) = maybe_rendered {
			std::fs::write(&script_path, content).map_err(|e| Error::Io {
				source: e,
				path: ctx.scope.resource.path().to_path_buf(),
				target: None,
				context: ErrorContext::from_scope(&ctx.scope),
			})?;
		}
		Ok(script_path)
	}

	fn run_script(&self, ctx: &ExecutionContext) -> anyhow::Result<ProcessOutput, Error> {
		let script = self.write(ctx)?;
		let output = Command::new(&self.exec)
			.args(self.args.as_slice())
			.arg(&script)
			.stdout(Stdio::piped())
			.spawn()
			.map_err(|e| Error::Script {
				source: e,
				script: script.clone(),
				context: ErrorContext::from_scope(&ctx.scope),
			})?
			.wait_with_output()
			.map_err(|e| Error::Script {
				source: e,
				script: script.clone(),
				context: ErrorContext::from_scope(&ctx.scope),
			})?;
		Ok(output)
	}
}

// #[cfg(test)]
// mod tests {
// 	use crate::{config::context::ContextHarness, templates::Templater};

// 	use super::*;

// 	#[test]
// 	fn test_script_filter() -> Result<()> {
// 		let src = Resource::new("/home", Some("/")).unwrap();
// 		let content = String::from("print('huh')\nprint('{{path}}'.islower())");
// 		let mut script = Script::new("python", content.clone()).unwrap();
// 		let mut template_engine = Templater::default();
// 		template_engine.add_templates(Filter::templates(&script))?;
// 		let mut harness = ContextHarness::new();
// 		harness.services.templater = template_engine;
// 		let ctx = harness.context();

// 		script.run_script(&src, &ctx).unwrap_or_else(|_| {
// 			// some linux distributions don't have a `python` executable, but a `python3`
// 			script = Script::new("python3", content).unwrap();
// 			script.run_script(&src, &ctx).unwrap()
// 		});
// 		assert!(script.filter(&src, &ctx));
// 		Ok(())
// 	}
// }
