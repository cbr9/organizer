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
	fn templates(&self) -> Vec<&Template> {
		let mut templates = vec![];
		templates.extend(self.startswith.iter());
		templates.extend(self.endswith.iter());
		templates.extend(self.contains.iter());
		templates
	}

	#[tracing::instrument(ret, level = "debug", skip(template_engine, variables))]
	fn filter(&self, res: &Resource, template_engine: &TemplateEngine, variables: &[Box<dyn Variable>]) -> bool {
		let filename = res.path.file_name().unwrap_or_default().to_string_lossy();

		if filename.is_empty() {
			return false;
		}

		let filename_cmp = if self.case_sensitive {
			filename.to_string()
		} else {
			filename.to_lowercase()
		};

		let context = TemplateEngine::new_context(res, variables);

		let startswith = if self.startswith.is_empty() {
			true
		} else {
			self.startswith.iter().any(|template| {
				// The rendered string must also be lowercased for a case-insensitive match.
				let rendered = template_engine.render(template, &context).unwrap_or(template.text.clone());
				let pattern = if self.case_sensitive { rendered } else { rendered.to_lowercase() };

				let (pattern, negate) = if let Some(stripped) = pattern.strip_prefix('!') {
					(stripped, true)
				} else {
					(pattern.as_str(), false)
				};

				let matches = filename_cmp.starts_with(pattern);
				if negate {
					!matches
				} else {
					matches
				}
			})
		};

		let endswith = if self.endswith.is_empty() {
			true
		} else {
			self.endswith.iter().any(|template| {
				let rendered = template_engine.render(template, &context).unwrap_or(template.text.clone());
				let pattern = if self.case_sensitive { rendered } else { rendered.to_lowercase() };

				let (pattern, negate) = if let Some(stripped) = pattern.strip_prefix('!') {
					(stripped, true)
				} else {
					(pattern.as_str(), false)
				};

				let matches = filename_cmp.ends_with(pattern);
				if negate {
					!matches
				} else {
					matches
				}
			})
		};

		let contains = if self.contains.is_empty() {
			true
		} else {
			self.contains.iter().any(|template| {
				let rendered = template_engine.render(template, &context).unwrap_or(template.text.clone());
				let pattern = if self.case_sensitive { rendered } else { rendered.to_lowercase() };

				let (pattern, negate) = if let Some(stripped) = pattern.strip_prefix('!') {
					(stripped, true)
				} else {
					(pattern.as_str(), false)
				};

				let matches = filename_cmp.contains(pattern);
				if negate {
					!matches
				} else {
					matches
				}
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
