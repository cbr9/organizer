use std::collections::HashSet;

use anyhow::Result;
use filters::{
	content::file_content,
	misc::{hash, mime},
	path::{extension, filename, parent, stem},
	size::size,
};
use template::Template;
use tera::{Context, Tera};

use crate::{config::variables::Variable, resource::Resource};

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

	fn get_template_names(&self) -> HashSet<&str> {
		self.tera.get_template_names().collect()
	}
	pub fn new_empty_context(&self) -> Context {
		let mut context = Context::new();

		for var in self.variables.iter() {
			var.register(self, &mut context);
		}

		context
	}

	pub fn new_context(&self, resource: &Resource) -> Context {
		let mut context = Context::new();
		context.insert("path", &resource.path);
		context.insert("root", &resource.root);

		for var in self.variables.iter() {
			var.register(self, &mut context);
		}

		context
	}
	pub fn render(&self, template: &Template, context: &Context) -> tera::Result<String> {
		self.tera.render(&template.id, context)
	}

	pub fn render_without_context(&self, template: &Template) -> tera::Result<String> {
		let context = Context::new();
		self.tera.render(&template.id, &context)
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
		tera.register_filter("filecontent", file_content);
		Self { tera, variables: vec![] }
	}
}
