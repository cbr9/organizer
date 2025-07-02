use std::sync::Arc;

use crate::{
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

// This accessor is created dynamically. It stores the environment
// variable key that it needs to look up.
#[derive(Debug, Clone, PartialEq, Eq)]
struct EnvAccessor {
	key: String,
}

#[async_trait]
impl Accessor for EnvAccessor {
	async fn get(&self, _ctx: &ExecutionContext) -> Result<Value> {
		let value = std::env::var(&self.key).unwrap_or("<UNDEFINED>".to_string());
		Ok(Value::String(value))
	}
}

/// The provider for the `{{ env }}` variable.
#[derive(Debug, Clone)]
pub struct EnvProvider;

impl StatelessVariable for EnvProvider {
	fn name(&self) -> &'static str {
		"env"
	}

	fn schema(&self) -> Property {
		Property {
			name: self.name(), // Use the canonical name from the trait method.
			// This node indicates that `env` is a map with dynamic keys.
			node: SchemaNode::DynamicMap(Arc::new(|key: &str| {
				// The constructor captures the key from the property chain
				// (e.g., "HOME") and creates an EnvAccessor for it.
				Box::new(EnvAccessor { key: key.to_string() })
			})),
		}
	}
}

// Automatically register the `EnvProvider` with the global inventory.
static ENV_PROVIDER: EnvProvider = EnvProvider;
inventory::submit!(VariableInventory { provider: &ENV_PROVIDER });
