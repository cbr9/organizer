use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct RunSettings {
	pub dry_run: bool,
	pub args: HashMap<String, String>,
}
