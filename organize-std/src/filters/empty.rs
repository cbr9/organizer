use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};

use anyhow::Result;

use organize_sdk::{
	context::{services::fs::resource::Resource, ExecutionContext},
	error::Error,
	plugins::filter::Filter,
};

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Empty;

#[async_trait]
#[typetag::serde(name = "empty")]
impl Filter for Empty {
	async fn filter(&self, check: Option<&PathBuf>, ctx: &ExecutionContext) -> Result<Vec<Arc<Resource>>, Error> {
		let resource = ctx.scope.resource()?;
		let path = check.unwrap_or(&resource.path);

		let backend = &resource.backend;
		let content = backend.read(path, ctx).await?;

		if content.is_empty() {
			Ok(vec![resource.clone()])
		} else {
			Ok(vec![])
		}
	}
}
