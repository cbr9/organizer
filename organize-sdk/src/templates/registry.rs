use std::collections::HashMap;

use crate::{
	error::Error,
	templates::{
		accessor::Accessor,
		schema::{Property, SchemaNode},
		variable::VariableInventory,
	},
};

/// The central schema registry and compiler for template variables.
///
/// It discovers all variable providers at startup and uses their schemas
/// to parse and validate property chains.
#[derive(Clone, Debug)]
pub struct SchemaRegistry {
	/// A map of all discovered static schemas for fast lookups by name.
	root_properties: HashMap<&'static str, Property>,
}

impl Default for SchemaRegistry {
	fn default() -> Self {
		Self::new()
	}
}

impl SchemaRegistry {
	/// Creates a new registry by discovering all registered `Variable` providers
	/// via the `inventory` crate.
	pub fn new() -> Self {
		let root_properties = inventory::iter::<VariableInventory>
			.into_iter()
			.map(|inv| {
				let schema = inv.provider.schema();
				// The key is the static name of the variable (e.g., "file").
				(schema.name, schema)
			})
			.collect();

		Self { root_properties }
	}

	/// Parses a property chain against the compiled schema to get an Accessor.
	pub fn parse_property_chain(&self, parts: &[&str]) -> Result<Box<dyn Accessor>, Error> {
		if parts.is_empty() {
			return Err(Error::TemplateError(super::engine::TemplateError::UnknownVariable(
				"Property chain cannot be empty.".to_string(),
			)));
		}

		let root_part = parts[0];
		let mut current_prop = self.root_properties.get(root_part).ok_or_else(|| {
			let valid_options: Vec<_> = self.root_properties.keys().collect();
			Error::TemplateError(super::engine::TemplateError::UnknownVariable(format!(
				"Invalid root variable '{root_part}'. Valid options are: {valid_options:?}"
			)))
		})?;

		// Traverse the rest of the chain.
		for (i, &part) in parts.iter().skip(1).enumerate() {
			match &current_prop.node {
				SchemaNode::Object(properties) => {
					current_prop = properties.iter().find(|p| p.name == part).ok_or_else(|| {
						let valid_options: Vec<_> = properties.iter().map(|p| p.name).collect();
						Error::TemplateError(super::engine::TemplateError::UnknownVariable(format!(
							"Invalid property '{}' at index {}. Valid options for '{}' are: {:?}",
							part,
							i + 1,
							parts[..i + 1].join("."),
							valid_options
						)))
					})?;
				}
				SchemaNode::DynamicMap(constructor) => {
					if i + 2 < parts.len() {
						return Err(Error::TemplateError(super::engine::TemplateError::UnknownVariable(format!(
							"Cannot access properties on a dynamic value. Chain has too many parts after key '{}' in '{}'.",
							part,
							parts.join(".")
						))));
					}
					// The chain ends here. Call the constructor with the key.
					return Ok(constructor(part));
				}
				SchemaNode::Terminal(_) => {
					return Err(Error::TemplateError(super::engine::TemplateError::UnknownVariable(format!(
						"Cannot access property '{}' on a terminal value at '{}'.",
						part,
						parts[..i + 1].join(".")
					))));
				}
			}
		}

		// After the loop, the final node must be a Terminal.
		match &current_prop.node {
			SchemaNode::Terminal(constructor) => Ok(constructor()),
			_ => Err(Error::TemplateError(super::engine::TemplateError::UnknownVariable(format!(
				"Incomplete property chain '{}'. It points to an object, not a final value.",
				parts.join(".")
			)))),
		}
	}
}
