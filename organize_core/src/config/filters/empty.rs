use serde::Deserialize;

use crate::resource::Resource;

use super::AsFilter;

#[derive(Eq, PartialEq, Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Empty;

impl AsFilter for Empty {
	fn matches(&self, res: &Resource) -> bool {
		let path = res.path();
		if path.is_file() {
			std::fs::metadata(path.as_ref()).map(|md| md.len() == 0).unwrap_or(false)
		} else {
			path.read_dir().map(|mut i| i.next().is_none()).unwrap_or(false)
		}
	}
}
