use std::sync::Arc;

use organize_sdk::{
	context::ExecutionContext,
	templates::{
		accessor::Accessor,
		schema::{Property, SchemaNode},
		value::Value,
		variable::{StatelessVariable, VariableInventory},
	},
};
use anyhow::Result;
use async_trait::async_trait;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileProvider;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Path;

#[async_trait]
impl Accessor for Path {
	async fn get(&self, ctx: &ExecutionContext) -> Result<Value> {
		let resource = ctx.scope.resource()?;
		let path_str = resource.path.to_string_lossy().to_string();
		Ok(Value::String(path_str))
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Name;

#[async_trait]
impl Accessor for Name {
	async fn get(&self, ctx: &ExecutionContext) -> Result<Value> {
		let resource = ctx.scope.resource()?;
		let value = resource.path.file_name().map(|name| name.to_string_lossy().to_string());
		Ok(Value::OptionString(value))
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Stem;

#[async_trait]
impl Accessor for Stem {
	async fn get(&self, ctx: &ExecutionContext) -> Result<Value> {
		let resource = ctx.scope.resource()?;
		let value = resource.path.file_stem().map(|stem| stem.to_string_lossy().to_string());
		Ok(Value::OptionString(value))
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Extension;

#[async_trait]
impl Accessor for Extension {
	async fn get(&self, ctx: &ExecutionContext) -> Result<Value> {
		let resource = ctx.scope.resource()?;
		let value = resource.path.extension().map(|ext| ext.to_string_lossy().to_string());
		Ok(Value::OptionString(value))
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Parent;

#[async_trait]
impl Accessor for Parent {
	async fn get(&self, ctx: &ExecutionContext) -> Result<Value> {
		let resource = ctx.scope.resource()?;
		let value = resource.path.parent().map(|parent| parent.to_string_lossy().to_string());
		Ok(Value::OptionString(value))
	}
}

// SCHEMA AND REGISTRATION

impl StatelessVariable for FileProvider {
	fn name(&self) -> &'static str {
		"file"
	}

	/// Defines the schema for the `file` variable.
	fn schema(&self) -> Property {
		Property {
			name: self.name(), // Use the canonical name from the trait method.
			node: SchemaNode::Object(vec![
				Property {
					name: "path",
					node: SchemaNode::Terminal(Arc::new(|| Box::new(Path))),
				},
				Property {
					name: "name",
					node: SchemaNode::Terminal(Arc::new(|| Box::new(Name))),
				},
				Property {
					name: "stem",
					node: SchemaNode::Terminal(Arc::new(|| Box::new(Stem))),
				},
				Property {
					name: "extension",
					node: SchemaNode::Terminal(Arc::new(|| Box::new(Extension))),
				},
				Property {
					name: "parent",
					node: SchemaNode::Terminal(Arc::new(|| Box::new(Parent))),
				},
			]),
		}
	}
}

inventory::submit! {
	VariableInventory {
		provider: &FileProvider
	}
}
