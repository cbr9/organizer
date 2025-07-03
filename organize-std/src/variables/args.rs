use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use organize_sdk::{
	context::ExecutionContext,
	templates::{
		accessor::Accessor,
		schema::{Property, SchemaNode},
		value::Value,
		variable::{StatelessVariable, VariableInventory},
	},
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct ArgsAccessor {
	key: String,
}

#[async_trait]
impl Accessor for ArgsAccessor {
	async fn get(&self, ctx: &ExecutionContext) -> Result<Value> {
		let value = ctx.settings.args.get(&self.key).cloned().unwrap_or("<UNDEFINED>".to_string());
		Ok(Value::String(value))
	}
}

#[derive(Debug, Clone)]
pub struct ArgsProvider;

impl StatelessVariable for ArgsProvider {
	fn name(&self) -> &'static str {
		"args"
	}

	fn schema(&self) -> Property {
		Property {
			name: self.name(),
			node: SchemaNode::DynamicMap(Arc::new(|key: &str| {
				Box::new(ArgsAccessor { key: key.to_string() })
			})),
		}
	}
}

static ARGS_PROVIDER: ArgsProvider = ArgsProvider;
inventory::submit!(VariableInventory { provider: &ARGS_PROVIDER });
