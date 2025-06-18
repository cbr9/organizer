use std::{
	collections::HashMap,
	fmt::Debug,
	fs::OpenOptions,
	io::{Seek, Write as Writer},
	path::PathBuf,
};

use crate::config::{actions::common::enabled, context::ExecutionContext};
use anyhow::Result;
use itertools::Itertools;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};

use crate::{resource::Resource, templates::template::Template};

use super::{Action, ExecutionModel};

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, Default, Debug)]
#[serde(rename = "lowercase")]
pub enum WriteMode {
	#[default]
	Append,
	Prepend,
	Overwrite,
}

#[derive(Deserialize, Serialize, Default, PartialEq, Eq, Clone, Debug)]
#[serde(rename = "kebab_case")]
pub enum ContinueWith {
	#[default]
	Original,
	WrittenFile,
}

#[derive(Deserialize, Serialize, PartialEq, Eq, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Write {
	pub text: Template,
	pub outfile: Template,
	#[serde(default)]
	pub mode: WriteMode,
	#[serde(default)]
	pub sort_lines: bool,
	#[serde(default)]
	pub continue_with: ContinueWith,
	#[serde(default = "enabled")]
	pub enabled: bool,
}

#[typetag::serde]
impl Action for Write {
	fn templates(&self) -> Vec<&Template> {
		vec![&self.text, &self.outfile]
	}

	fn execution_model(&self) -> ExecutionModel {
		ExecutionModel::Collection
	}

	#[tracing::instrument(ret(level = "info"), err(Debug), level = "debug", skip(ctx))]
	fn execute_collection(&self, resources: Vec<&Resource>, ctx: &ExecutionContext) -> Result<Option<Vec<PathBuf>>> {
		if !self.enabled || resources.is_empty() {
			let paths: Vec<PathBuf> = resources.iter().map(|res| res.path().to_path_buf()).collect();
			return Ok(Some(paths));
		}

		let mut texts_by_outfile: HashMap<PathBuf, Vec<String>> = resources
			.par_iter()
			.filter_map(|res| {
				let context = ctx
					.services
					.template_engine
					.context()
					.path(res.path())
					.root(res.root())
					.build(&ctx.services.template_engine);
				let outfile_str = ctx.services.template_engine.render(&self.outfile, &context).ok()??;
				let text = ctx.services.template_engine.render(&self.text, &context).ok()??;
				Some((PathBuf::from(outfile_str), text))
			})
			.collect::<Vec<(PathBuf, String)>>() // Collect to un-parallelize before grouping
			.into_iter()
			.into_group_map();

		if !ctx.settings.dry_run {
			for (path, texts) in texts_by_outfile.iter_mut() {
				if let Some(parent) = path.parent() {
					std::fs::create_dir_all(parent)?;
				}

				let original_content = std::fs::read_to_string(path)?;
				let mut file = OpenOptions::new().truncate(true).read(true).write(true).open(path)?;

				if self.sort_lines {
					texts.sort_by_key(|a| a.to_lowercase());
				}

				// The file must be truncated before writing to correctly handle `overwrite` and `prepend`.
				file.set_len(0)?;
				file.seek(std::io::SeekFrom::Start(0))?;

				match self.mode {
					WriteMode::Append => {
						file.write_all(original_content.as_bytes())?;
						file.write_all(texts.join("\n").as_bytes())?;
					}
					WriteMode::Prepend => {
						file.write_all(texts.join("\n").as_bytes())?;
						if !original_content.is_empty() {
							file.write_all(b"\n")?;
							file.write_all(original_content.as_bytes())?;
						}
					}
					WriteMode::Overwrite => {
						file.write_all(texts.join("\n").as_bytes())?;
					}
				}
			}
		}

		match self.continue_with {
			ContinueWith::Original => {
				let paths = resources.iter().map(|res| res.path().to_path_buf()).collect();
				Ok(Some(paths))
			}
			ContinueWith::WrittenFile => {
				// Use the root of the first original resource for context.
				let written_files = texts_by_outfile.keys().map(|path| path.to_path_buf()).collect();
				Ok(Some(written_files))
			}
		}
	}
}
