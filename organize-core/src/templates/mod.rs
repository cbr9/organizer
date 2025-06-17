use std::collections::HashSet;

use anyhow::Result;
use filters::{
	misc::{hash, mime},
	path::{extension, filename, parent, stem},
	size::size,
};
use template::Template;
use tera::{Context, Tera};

use crate::{
	config::{variables::Variable, ConfigBuilder},
	resource::Resource,
};

pub mod filters;
pub mod template;

#[derive(Clone, Debug)]
pub struct TemplateEngine {
	pub tera: Tera,
	pub variables: Vec<Box<dyn Variable>>,
}

impl PartialEq for TemplateEngine {
	fn eq(&self, other: &Self) -> bool {
		self.get_template_names() == other.get_template_names()
	}
}

impl TemplateEngine {
	pub fn new(variables: &Vec<Box<dyn Variable>>) -> Self {
		let mut engine = Self::default();
		for var in variables.iter() {
			let templates = var.templates();
			engine.add_templates(&templates).unwrap();
		}
		engine.variables = variables.clone();
		engine
	}

	pub fn from_config(config: &ConfigBuilder) -> anyhow::Result<Self> {
		let mut engine = Self::default();
		for rule in config.rules.iter() {
			for action in rule.actions.iter() {
				engine.add_templates(&action.templates())?;
			}
			for filter in rule.filters.iter() {
				engine.add_templates(&filter.templates())?;
			}
			for folder in rule.folders.iter() {
				engine.add_template(&folder.root)?;
			}
		}
		Ok(engine)
	}

	pub fn get_template_names(&self) -> HashSet<&str> {
		self.tera.get_template_names().collect()
	}

	pub fn empty_context(&self) -> Context {
		let mut context = Context::new();

		for var in self.variables.iter() {
			var.register(self, &mut context);
		}

		context
	}

	pub fn context(&self, resource: &Resource) -> Context {
		let mut context = Context::new();
		context.insert("path", &resource.path());
		context.insert("root", &resource.root());

		for var in self.variables.iter() {
			var.register(self, &mut context);
		}

		context
	}

	#[tracing::instrument(err)]
	pub fn render(&self, template: &Template, context: &Context) -> tera::Result<Option<String>> {
		match self.tera.render(&template.id, context) {
			Ok(res) => {
				if res.is_empty() {
					return Ok(None);
				} else {
					return Ok(Some(res));
				}
			}
			Err(e) => Err(e),
		}
	}

	pub fn render_without_context(&self, template: &Template) -> tera::Result<Option<String>> {
		let context = Context::new();
		self.render(template, &context)
	}

	pub fn add_template(&mut self, template: &Template) -> Result<()> {
		if !self.get_template_names().contains(template.id.as_str()) {
			self.tera.add_raw_template(&template.id, &template.text)?;
		}
		Ok(())
	}

	pub fn add_templates(&mut self, templates: &[&Template]) -> Result<()> {
		for template in templates.iter() {
			if !self.get_template_names().contains(template.id.as_str()) {
				self.tera.add_raw_template(&template.id, &template.text)?;
			}
		}
		Ok(())
	}
}

impl Default for TemplateEngine {
	/// Creates a new, empty engine. This is primarily for convenience.
	/// The actual populated engine is created in `ConfigBuilder`.
	fn default() -> Self {
		let mut tera = Tera::default();
		tera.register_filter("parent", parent);
		tera.register_filter("stem", stem);
		tera.register_filter("filename", filename);
		tera.register_filter("extension", extension);
		tera.register_filter("mime", mime);
		tera.register_filter("filesize", size);
		tera.register_filter("hash", hash);
		Self { tera, variables: vec![] }
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::config::variables::simple::SimpleVariable;

	#[test]
	fn render_template_not_present_in_engine() {
		let engine = TemplateEngine::default();
		let template = Template::from("Hello, {{ name }}!");
		let mut context = Context::new();
		context.insert("name", "Andrés");
		let rendered = engine.render(&template, &context);
		assert!(rendered.is_err());
	}

	#[test]
	fn render_template_present_in_engine() {
		let mut engine = TemplateEngine::default();
		let template = Template::from("This is a stored template.");
		engine.add_template(&template).unwrap();
		let context = Context::new();
		let rendered = engine.render(&template, &context).unwrap();
		assert_eq!(rendered, Some("This is a stored template.".to_string()));
	}

	#[test]
	fn render_with_simple_variable() {
		let var = SimpleVariable {
			name: "location".into(),
			value: "world".into(),
		};
		let mut engine = TemplateEngine::new(&vec![Box::new(var)]);
		let template = Template::from("Hello, {{ location }}!");
		engine.add_template(&template).unwrap();
		let context = engine.empty_context();
		let rendered = engine.render(&template, &context).unwrap();
		assert_eq!(rendered, Some("Hello, world!".to_string()));
	}

	#[test]
	fn render_with_path_context() {
		let mut engine = TemplateEngine::default();
		let resource = Resource::new_tmp("test.txt");
		let template = Template::from("The path is {{ path | stem }}");
		engine.add_template(&template).unwrap();
		let context = engine.context(&resource);
		let rendered = engine.render(&template, &context).unwrap();
		assert_eq!(rendered, Some("The path is test".to_string()));
	}

	#[test]
	fn render_invalid_template_returns_none() {
		let engine = TemplateEngine::default();
		// Invalid syntax: `{%` instead of `{{`
		let template = Template::from("Hello, {% name }}!");
		let context = Context::new();
		let rendered = engine.render(&template, &context);
		assert!(rendered.is_err());
	}
}
