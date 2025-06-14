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
	pub templates: HashSet<String>,
}

impl PartialEq for TemplateEngine {
	fn eq(&self, other: &Self) -> bool {
		self.templates == other.templates
	}
}

impl TemplateEngine {
	pub fn new_empty_context(variables: &[Box<dyn Variable>]) -> Context {
		let mut context = Context::new();

		let mut engine = TemplateEngine::default();
		for var in variables.iter() {
			let templates = var.templates();
			engine.add_templates(&templates).unwrap();
		}

		for var in variables.iter() {
			var.register(&engine, &mut context);
		}
		context
	}

	pub fn new_context(resource: &Resource, variables: &[Box<dyn Variable>]) -> Context {
		let mut context = Context::new();
		context.insert("path", &resource.path);
		context.insert("root", &resource.root);

		let mut engine = TemplateEngine::default();
		for var in variables.iter() {
			let templates = var.templates();
			engine.add_templates(&templates).unwrap();
		}

		for var in variables.iter() {
			var.register(&engine, &mut context);
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
		if !self.templates.contains(&template.id) {
			self.tera.add_raw_template(&template.id, &template.text)?;
			self.templates.insert(template.id.clone());
		}
		Ok(())
	}

	pub fn add_templates(&mut self, templates: &[Template]) -> Result<()> {
		for template in templates.iter() {
			if !self.templates.contains(&template.id) {
				self.tera.add_raw_template(&template.id, &template.text)?;
				self.templates.insert(template.id.clone());
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
		Self {
			tera,
			templates: HashSet::new(),
		}
	}
}
