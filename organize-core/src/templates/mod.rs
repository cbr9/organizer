use std::collections::HashSet;

use anyhow::Result;
use filters::{
	misc::{hash, mime},
	path::{extension, filename, parent, stem},
	size::size,
};
use template::Template;
use tera::Tera;

use crate::{
	config::{context::ExecutionContext, variables::Variable, ConfigBuilder},
	templates::lazy::LazyVariable,
};

pub mod filters;
pub mod lazy;
pub mod template;

#[derive(Clone, Debug)]
pub struct Templater {
	pub tera: Tera,
	pub variables: Vec<Box<dyn Variable>>,
}

pub struct Context(tera::Context);

impl std::ops::Deref for Context {
	type Target = tera::Context;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl Context {
	pub fn new(ctx: &ExecutionContext) -> Self {
		let mut context = tera::Context::new();
		context.insert("path", ctx.scope.resource);
		context.insert("root", &ctx.scope.folder.path);

		for var in &ctx.scope.rule.variables {
			println!("{}", var.typetag_name());
			let lazy = LazyVariable { variable: var, context: ctx };
			context.insert(var.name(), &lazy);
		}

		Self(context)
	}
}

impl PartialEq for Templater {
	fn eq(&self, other: &Self) -> bool {
		self.get_template_names() == other.get_template_names()
	}
}

impl Templater {
	pub fn new(variables: &Vec<Box<dyn Variable>>) -> Self {
		let mut engine = Self::default();
		for var in variables.iter() {
			let templates = var.templates();
			engine.add_templates(templates).unwrap();
		}
		engine.variables = variables.clone();
		engine
	}

	pub fn from_config(config: &ConfigBuilder) -> anyhow::Result<Self> {
		let mut engine = Self::default();
		for rule in config.rules.iter() {
			for action in rule.actions.iter() {
				engine.add_templates(action.templates())?;
			}
			for variable in rule.variables.iter() {
				engine.add_templates(variable.templates())?;
			}
			for filter in rule.filters.iter() {
				engine.add_templates(filter.templates())?;
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

	pub fn render(&self, template: &Template, context: &tera::Context) -> tera::Result<Option<String>> {
		match self.tera.render(&template.id, context) {
			Ok(res) => {
				if res.is_empty() {
					Ok(None)
				} else {
					Ok(Some(res))
				}
			}
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
