use std::collections::HashSet;

use anyhow::Result;
use filters::{
	misc::mime,
	path::{extension, filename, parent, stem},
	size::size,
};
use template::Template;
use tera::Tera;

use crate::config::{
	context::ExecutionContext,
	variables::{builtins, Variable},
	ConfigBuilder,
};

pub mod filters;
pub mod template;

#[derive(Clone, Debug)]
pub struct Templater {
	pub tera: Tera,
	pub variables: Vec<Box<dyn Variable>>,
}

impl PartialEq for Templater {
	fn eq(&self, other: &Self) -> bool {
		self.get_template_names() == other.get_template_names()
	}
}

impl Templater {
	pub fn from_config(config: &ConfigBuilder) -> anyhow::Result<Self> {
		let mut templater = Self::default();

		for rule in config.rules.iter() {
			for action in rule.actions.iter() {
				templater.add_templates(action.templates())?;
			}
			for filter in rule.filters.iter() {
				templater.add_templates(filter.templates())?;
			}
			for folder in rule.folders.iter() {
				templater.add_template(&folder.root)?;
			}

			for variable in rule.variables.iter() {
				templater.add_templates(variable.templates())?;
				templater.variables.push(variable.clone());
			}
		}

		Ok(templater)
	}

	pub fn get_template_names(&self) -> HashSet<&str> {
		self.tera.get_template_names().collect()
	}

	pub async fn render(&self, template: &Template, ctx: &ExecutionContext<'_>) -> tera::Result<Option<String>> {
		let dependencies = &template.dependencies;
		dbg!(&dependencies);
		let mut context = tera::Context::new();
		let available_variable_names: HashSet<&str> = self.variables.iter().map(|v| v.name().unwrap_or(v.typetag_name())).collect();
		let needed_variables = dependencies
			.iter()
			.map(String::as_str)
			.filter(|&s| available_variable_names.contains(s));

		for var_name in needed_variables {
			if let Some(var) = self.variables.iter().find(|v| v.name().unwrap_or(v.typetag_name()) == var_name) {
				let value = var.compute(ctx).await.map_err(|e| {
					tera::Error::msg(format!(
						"Failed to compute variable '{}': {}",
						var.name().unwrap_or(var.typetag_name()),
						e
					))
				})?;
				context.insert(var.name().unwrap_or(var.typetag_name()), &value);
			}
		}

		// 6. Render the template with the lean, just-in-time context.
		match self.tera.render(&template.id, &context) {
			Ok(res) if res.is_empty() => Ok(None),
			Ok(res) => Ok(Some(res)),
			Err(e) => Err(e),
		}
	}

	pub fn add_template(&mut self, template: &Template) -> Result<()> {
		if !self.get_template_names().contains(template.id.as_str()) {
			self.tera.add_raw_template(&template.id, &template.input)?;
		}
		Ok(())
	}

	pub fn add_templates(&mut self, templates: Vec<&Template>) -> Result<()> {
		for template in templates.into_iter() {
			if !self.get_template_names().contains(template.id.as_str()) {
				self.tera.add_raw_template(&template.id, &template.input)?;
			}
		}
		Ok(())
	}
}

impl Default for Templater {
	/// Creates a new Templater with all built-in filters and lazy variables.
	fn default() -> Self {
		let mut tera = Tera::default();
		tera.register_filter("parent", parent);
		tera.register_filter("stem", stem);
		tera.register_filter("filename", filename);
		tera.register_filter("extension", extension);
		tera.register_filter("mime", mime);
		tera.register_filter("filesize", size);

		// Initialize the templater with the complete set of built-in lazy variables
		let variables: Vec<Box<dyn Variable>> = vec![
			Box::new(builtins::Path),
			Box::new(builtins::Root),
			Box::new(builtins::Metadata::new()),
			Box::new(builtins::Hash::new()),
		];

		Self { tera, variables }
	}
}
// #[cfg(test)]
// mod tests {
// 	use std::convert::{TryFrom, TryInto};

// 	use super::*;
// 	use crate::{config::{context::RunServices, variables::simple::SimpleVariable}, resource::Resource};

// 	#[test]
// 	fn render_template_not_present_in_engine() {
// 		let engine = Templater::default();
// 		let template = Template::try_from("Hello, {{ name }}!").unwrap();
// 		let context = Context::new(ctx)
// 		let mut context = engine.context().build(&engine);
// 		context.insert("name", "Andrés");
// 		let rendered = engine.render(&template, &context);
// 		assert!(rendered.is_err());
// 	}

// 	#[test]
// 	fn render_template_present_in_engine() {
// 		let mut engine = Templater::default();
// 		let template = Template::try_from("This is a stored template.").unwrap();
// 		engine.add_template(&template).unwrap();
// 		let context = engine.context().build(&engine);
// 		let rendered = engine.render(&template, &context).unwrap();
// 		assert_eq!(rendered, Some("This is a stored template.".to_string()));
// 	}

// 	#[test]
// 	fn render_with_simple_variable() {
// 		let var = SimpleVariable {
// 			name: "location".into(),
// 			value: "world".try_into().unwrap(),
// 		};
// 		let mut engine = Templater::new(&vec![Box::new(var)]);
// 		let template = Template::try_from("Hello, {{ location }}!").unwrap();
// 		engine.add_template(&template).unwrap();
// 		let context = engine.context().build(&engine);
// 		let rendered = engine.render(&template, &context).unwrap();
// 		assert_eq!(rendered, Some("Hello, world!".to_string()));
// 	}

// 	#[test]
// 	fn render_with_path_context() {
// 		let mut engine = Templater::default();
// 		let resource = Resource::new_tmp("test.txt");
// 		let template = Template::try_from("The path is {{ path | stem }}").unwrap();
// 		engine.add_template(&template).unwrap();
// 		let context = engine.context().path(resource.path().to_path_buf()).build(&engine);
// 		let rendered = engine.render(&template, &context).unwrap();
// 		assert_eq!(rendered, Some("The path is test".to_string()));
// 	}

// 	#[test]
// 	fn render_invalid_template_returns_none() {
// 		let engine = Templater::default();
// 		// Invalid syntax: `{%` instead of `{{`
// 		let template = Template::try_from("Hello, {% name }}!").unwrap();
// 		let context = engine.context().build(&engine);
// 		let rendered = engine.render(&template, &context);
// 		assert!(rendered.is_err());
// 	}
// }
