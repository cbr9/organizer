use std::path::PathBuf;

use serde::Deserialize;

use crate::{
	resource::Resource,
	templates::{Template},
};

use super::{common::ConflictOption, script::ActionConfig, AsAction};

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
enum WriteMode {
	Append,
	Prepend,
}

#[derive(Deserialize, PartialEq, Debug)]
#[serde(deny_unknown_fields)]
struct Write {
	to: Template,
	if_exists: ConflictOption,
	text: Template,
	mode: WriteMode,
}

impl<'a> AsAction<'a> for Write {
	fn execute<T: AsRef<std::path::Path>>(&self, src: &Resource, dest: Option<T>, dry_run: bool) -> anyhow::Result<Option<PathBuf>> {
		todo!()
	}

	const CONFIG: ActionConfig<'a> = ActionConfig {
		requires_dest: true,
		log_hint: "WRITE",
	};
}
