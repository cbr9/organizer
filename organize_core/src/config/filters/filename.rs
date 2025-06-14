use crate::{
	config::{filters::Filter, variables::Variable},
	resource::Resource,
	templates::{template::Template, TemplateEngine},
};
use serde::{Deserialize, Serialize};

// TODO: refactor

#[derive(Eq, PartialEq, Deserialize, Serialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Filename {
	#[serde(default)]
	pub startswith: Vec<Template>,
	#[serde(default)]
	pub endswith: Vec<Template>,
	#[serde(default)]
	pub contains: Vec<Template>,
	#[serde(default)]
	pub case_sensitive: bool,
}

#[typetag::serde(name = "filename")]
impl Filter for Filename {
	fn templates(&self) -> Vec<Template> {
		let mut templates = vec![];
		templates.extend(self.startswith.clone());
		templates.extend(self.endswith.clone());
		templates.extend(self.contains.clone());
		templates
	}

	#[tracing::instrument(ret, level = "debug", skip(template_engine, variables))]
	fn filter(&self, res: &Resource, template_engine: &TemplateEngine, variables: &[Box<dyn Variable>]) -> bool {
		let filename = res.path.file_name().unwrap_or_default().to_string_lossy();
		let context = TemplateEngine::new_context(res, variables);

		if filename.is_empty() {
			return false;
		}

		let startswith = if self.startswith.is_empty() {
			true
		} else {
			self.startswith
				.iter()
				.flat_map(|s| {
					template_engine
						.render(s, &context)
						.map(|s| if !self.case_sensitive { s.to_lowercase() } else { s })
				})
				.any(|mut s| {
					let mut negate = false;
					if s.starts_with('!') {
						negate = true;
						s = s.replacen('!', "", 1);
					}
					let mut matches = filename.starts_with(&s);
					if negate {
						matches = !matches
					}
					matches
				})
		};

		let endswith = if self.endswith.is_empty() {
			true
		} else {
			self.endswith
				.iter()
				.flat_map(|s| {
					template_engine
						.render(s, &context)
						.map(|s| if !self.case_sensitive { s.to_lowercase() } else { s })
				})
				.any(|mut s| {
					let mut negate = false;
					if s.starts_with('!') {
						negate = true;
						s = s.replacen('!', "", 1);
					}
					let mut matches = filename.ends_with(&s);
					if negate {
						matches = !matches
					}
					matches
				})
		};

		let contains = if self.contains.is_empty() {
			true
		} else {
			self.contains
				.iter()
				.flat_map(|s| {
					template_engine
						.render(s, &context)
						.map(|s| if !self.case_sensitive { s.to_lowercase() } else { s })
				})
				.any(|mut s| {
					let mut negate = false;

					if s.starts_with('!') {
						negate = true;
						s = s.replacen('!', "", 1);
					}
					let mut matches = filename.contains(&s);
					if negate {
						matches = !matches
					}
					matches
				})
		};
		startswith && endswith && contains
	}
}

#[cfg(test)]
mod tests {
	use std::str::FromStr;

	use crate::templates::TemplateEngine;

	use super::*;

	#[test]
	fn match_beginning_case_insensitive() {
		let path = Resource::from_str("$HOME/Downloads/test.pdf").unwrap();
		let filename = Filename {
			startswith: vec!["TE".into()],
			..Default::default()
		};
		let template_engine = TemplateEngine::default();
		let variables = vec![];
		assert!(filename.filter(&path, &template_engine, &variables))
	}

	#[test]
	fn match_ending_case_insensitive() {
		let path = Resource::from_str("$HOME/Downloads/test.pdf").unwrap();
		let filename = Filename {
			endswith: vec!["DF".into()],
			..Default::default()
		};
		let template_engine = TemplateEngine::default();
		let variables = vec![];
		assert!(filename.filter(&path, &template_engine, &variables))
	}

	#[test]
	fn match_containing_case_insensitive() {
		let path = Resource::from_str("$HOME/Downloads/test.pdf").unwrap();
		let filename = Filename {
			contains: vec!["ES".into()],
			..Default::default()
		};
		let template_engine = TemplateEngine::default();
		let variables = vec![];
		assert!(filename.filter(&path, &template_engine, &variables))
	}

	#[test]
	fn no_match_beginning_case_sensitive() {
		let path = Resource::from_str("$HOME/Downloads/test.pdf").unwrap();
		let filename = Filename {
			case_sensitive: true,
			startswith: vec!["TE".into()],
			..Default::default()
		};
		let template_engine = TemplateEngine::default();
		let variables = vec![];
		assert!(!filename.filter(&path, &template_engine, &variables))
	}

	#[test]
	fn no_match_ending_case_sensitive() {
		let path = Resource::from_str("$HOME/Downloads/test.pdf").unwrap();
		let filename = Filename {
			case_sensitive: true,
			startswith: vec!["DF".into()],
			..Default::default()
		};
		let template_engine = TemplateEngine::default();
		let variables = vec![];
		assert!(!filename.filter(&path, &template_engine, &variables))
	}

	#[test]
	fn no_match_containing_case_sensitive() {
		let path = Resource::from_str("$HOME/Downloads/test.pdf").unwrap();
		let filename = Filename {
			case_sensitive: true,
			contains: vec!["ES".into()],
			..Default::default()
		};
		let template_engine = TemplateEngine::default();
		let variables = vec![];
		assert!(!filename.filter(&path, &template_engine, &variables))
	}
	#[test]
	fn match_containing_case_sensitive() {
		let path = Resource::from_str("$HOME/Downloads/tESt.pdf").unwrap();
		let filename = Filename {
			case_sensitive: true,
			contains: vec!["ES".into()],
			..Default::default()
		};
		let template_engine = TemplateEngine::default();
		let variables = vec![];
		assert!(filename.filter(&path, &template_engine, &variables))
	}
	#[test]
	fn match_multiple_conditions_case_sensitive() {
		let path = Resource::from_str("$HOME/Downloads/tESt.pdf").unwrap();
		let filename = Filename {
			case_sensitive: true,
			contains: vec!["ES".into()],
			startswith: vec!["t".into()],
			endswith: vec!["df".into()],
		};
		let template_engine = TemplateEngine::default();
		let variables = vec![];
		assert!(filename.filter(&path, &template_engine, &variables))
	}
	#[test]
	fn match_multiple_conditions_some_negative() {
		let path = Resource::from_str("$HOME/Downloads/tESt.pdf").unwrap();
		let filename = Filename {
			case_sensitive: true,
			contains: vec!["ES".into()],
			startswith: vec!["t".into()],
			endswith: vec!["!df".into()],
		};
		let template_engine = TemplateEngine::default();
		let variables = vec![];
		assert!(!filename.filter(&path, &template_engine, &variables))
	}
	#[test]
	fn match_multiple_conditions_some_negative_2() {
		let path = Resource::from_str("$HOME/Downloads/tESt.pdf").unwrap();
		let filename = Filename {
			case_sensitive: true,
			contains: vec!["!ES".into(), "ES".into()],
			startswith: vec!["t".into()],
			endswith: vec!["!df".into()],
		};
		let template_engine = TemplateEngine::default();
		let variables = vec![];
		assert!(!filename.filter(&path, &template_engine, &variables))
	}
	#[test]
	fn match_multiple_conditions_some_negative_3() {
		let path = Resource::from_str("$HOME/Downloads/tESt.txt").unwrap();
		let filename = Filename {
			case_sensitive: true,
			contains: vec!["!ES".into(), "ES".into()],
			startswith: vec!["t".into()],
			endswith: vec!["!df".into()],
		};
		let template_engine = TemplateEngine::default();
		let variables = vec![];
		assert!(filename.filter(&path, &template_engine, &variables))
	}
}
