use std::{
	collections::HashMap,
	fmt::Debug,
	fs::{File, OpenOptions},
	io::{BufWriter, Read, Seek, Write as Writer},
	ops::Deref,
	path::PathBuf,
	sync::{LazyLock, Mutex},
};

use crate::config::actions::common::enabled;
use anyhow::{Context, Result};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{
	config::variables::Variable,
	path::prepare::prepare_target_path,
	resource::Resource,
	templates::{template::Template, TemplateEngine},
};

use super::{common::ConflictOption, Action};

static KNOWN_FILES: LazyLock<Mutex<HashMap<PathBuf, Mutex<File>>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, Default, Debug)]
#[serde(rename = "lowercase")]
pub enum WriteMode {
	#[default]
	Append,
	Prepend,
}

#[derive(Deserialize, Serialize, PartialEq, Eq, Clone, Debug)]
pub struct Write {
	text: Template,
	outfile: Template,
	#[serde(default)]
	mode: WriteMode,
	#[serde(default)]
	clear_before_first_write: bool,
	#[serde(default = "r#true")]
	sort_lines: bool,
	#[serde(default)]
	continue_with: ContinueWith,
	#[serde(default = "enabled")]
	enabled: bool,
}

#[derive(Deserialize, Serialize, Default, PartialEq, Eq, Clone, Debug)]
#[serde(rename = "kebab_case")]
pub enum ContinueWith {
	#[default]
	Original,
	WrittenFile,
}
fn r#true() -> bool {
	true
}

impl Write {
	fn get_target_path(&self, res: &Resource, template_engine: &TemplateEngine, variables: &[Box<dyn Variable>]) -> Result<Option<PathBuf>> {
		let path = prepare_target_path(&ConflictOption::Overwrite, res, &self.outfile, true, template_engine, variables)?;
		if let Some(path) = path.as_ref() {
			let mut lock = KNOWN_FILES.lock().unwrap();
			if !lock.contains_key(path) {
				let file = OpenOptions::new()
					.truncate(self.clear_before_first_write)
					.append(self.mode == WriteMode::Append)
					.create(true)
					.read(true)
					.open(path)?;

				lock.insert(path.clone(), Mutex::new(file));
			}
		}
		Ok(path)
	}
}

#[typetag::serde]
impl Action for Write {
	fn templates(&self) -> Vec<Template> {
		vec![self.text.clone(), self.outfile.clone()]
	}

	#[tracing::instrument(ret(level = "info"), err(Debug), level = "debug", skip(template_engine, variables))]
	fn execute(&self, res: &Resource, template_engine: &TemplateEngine, variables: &[Box<dyn Variable>], dry_run: bool) -> Result<Option<PathBuf>> {
		match self.get_target_path(res, template_engine, variables)? {
			Some(dest) => {
				if !dry_run && self.enabled {
					if let Some(parent) = dest.parent() {
						std::fs::create_dir_all(parent).with_context(|| format!("Could not create parent directory for {}", dest.display()))?;
					}
					if let Some(parent) = dest.parent() {
						std::fs::create_dir_all(parent).with_context(|| format!("Could not create parent directory for {}", dest.display()))?;
					}
					let context = TemplateEngine::new_context(res, variables);
					let mut text = template_engine.render(&self.text, &context)?;
					if self.mode == WriteMode::Prepend {
						let mut existing_content = std::fs::read_to_string(&dest)?;
						if !existing_content.ends_with('\n') {
							existing_content += "\n";
						}
						text = existing_content + text.as_str();
					}

					{
						let lock = KNOWN_FILES.lock().unwrap();
						let file = lock.get(&dest).expect("file should be there").lock().unwrap();
						let mut writer = BufWriter::new(file.deref());
						writeln!(writer, "{}", text)?;
						writer.flush()?;
					}
				}

				if self.continue_with == ContinueWith::WrittenFile {
					Ok(Some(dest))
				} else {
					Ok(Some(res.path.clone()))
				}
			}
			None => Ok(None),
		}
	}

	fn on_finish(&self, _resources: &[Resource], dry_run: bool) -> Result<()> {
		// sort lines in the file
		let mut lock = KNOWN_FILES.lock().unwrap();
		if self.sort_lines && !dry_run {
			for (_path, file) in lock.iter() {
				let mut file = file.lock().unwrap();
				file.seek(std::io::SeekFrom::Start(0))?;
				let mut buffer = String::new();
				file.read_to_string(&mut buffer)?;
				file.set_len(0)?;
				let mut writer = BufWriter::new(file.deref());
				write!(writer, "{}", buffer.lines().sorted_by_key(|s| s.to_lowercase()).join("\n"))?;
				writer.flush()?;
			}
		}

		lock.clear();
		Ok(())
	}
}
