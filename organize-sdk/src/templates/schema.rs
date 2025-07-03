use crate::templates::accessor::Accessor;
use std::{fmt::Debug, sync::Arc};

pub type TerminalAccessorConstructor = Arc<dyn Fn() -> Box<dyn Accessor> + Send + Sync>;
pub type DynamicMapAccessorConstructor = Arc<dyn Fn(&str) -> Box<dyn Accessor> + Send + Sync>;

#[derive(Clone)]
pub enum SchemaNode {
	/// A terminal node that creates a specific, type-safe Accessor.
	Terminal(TerminalAccessorConstructor),
	/// An object node with a fixed, known set of sub-properties.
	Object(Vec<Property>),
	/// A map node where sub-properties are dynamic keys.
	DynamicMap(DynamicMapAccessorConstructor),
}

#[derive(Clone)]
pub struct Property {
	pub name: &'static str,
	pub node: SchemaNode,
}

impl Debug for SchemaNode {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			SchemaNode::Terminal(_) => f.debug_tuple("Terminal").field(&"<closure>").finish(),
			SchemaNode::Object(properties) => f.debug_tuple("Object").field(properties).finish(),
			SchemaNode::DynamicMap(_) => f.debug_tuple("DynamicMap").field(&"<closure>").finish(),
		}
	}
}

// Manual `Debug` implementation for `Property`
impl Debug for Property {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Property")
			.field("name", &self.name)
			.field("node", &self.node)
			.finish()
	}
}
