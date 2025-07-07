use std::{collections::HashMap, path::PathBuf};

#[derive(Debug, Clone)]
pub struct RunSettings {
	pub dry_run: bool,
	pub args: HashMap<String, String>,
	pub snapshot: Option<PathBuf>,
}
