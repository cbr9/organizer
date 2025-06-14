use std::collections::HashSet;

use serde::Deserialize;

use crate::templates::TemplateEngine;

use super::{
	actions::Action,
	filters::Filter,
	folders::{Folder, FolderBuilder},
	options::OptionsBuilder,
	variables::Variable,
};

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct RuleBuilder {
	pub id: Option<String>,
	#[serde(default)]
	pub tags: HashSet<String>,
	#[serde(default)]
	pub r#continue: bool,
	pub actions: Vec<Box<dyn Action>>,
	pub filters: Vec<Box<dyn Filter>>,
	pub folders: Vec<FolderBuilder>,
	#[serde(flatten)]
	pub options: OptionsBuilder,
	#[serde(default)]
	pub variables: Vec<Box<dyn Variable>>,
}

impl RuleBuilder {
	pub fn build(self, defaults: &OptionsBuilder) -> anyhow::Result<Rule> {
		let mut template_engine = TemplateEngine::default();

		for action in self.actions.iter() {
			let templates = action.templates();
			template_engine.add_templates(&templates)?;
		}

		for filter in self.filters.iter() {
			let templates = filter.templates();
			template_engine.add_templates(&templates)?;
		}

		let folders = self
			.folders
			.clone()
			.into_iter()
			.map(|builder| builder.build(defaults, &self.options, &mut template_engine, &self.variables)) // Pass this rule's options builder
			.collect::<anyhow::Result<Vec<Folder>>>()?;

		Ok(Rule {
			id: self.id,
			tags: self.tags,
			r#continue: self.r#continue,
			actions: self.actions,
			filters: self.filters,
			folders, // Contains fully compiled Folders, each with its own Options
			variables: self.variables,
			template_engine,
		})
	}
}

#[derive(Debug, PartialEq, Clone)]
pub struct Rule {
	pub id: Option<String>,
	pub tags: HashSet<String>,
	pub r#continue: bool,
	pub actions: Vec<Box<dyn Action>>,
	pub filters: Vec<Box<dyn Filter>>,
	pub folders: Vec<Folder>,
	pub variables: Vec<Box<dyn Variable>>,
	pub template_engine: TemplateEngine,
}
