use std::path::PathBuf;

use anyhow::bail;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{
	config::{context::ExecutionContext, variables::Variable},
	resource::Resource,
	templates::template::Template,
};

#[derive(Debug, Clone)]
struct Cache(moka::future::Cache<Resource, String>);

impl std::ops::Deref for Cache {
	type Target = moka::future::Cache<Resource, String>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl Default for Cache {
	fn default() -> Self {
		Cache(moka::future::Cache::new(10_000))
	}
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Args {
	path: Option<Template>,
	name: Option<String>,
}

// -- Hash Variable (Stateful with Cache) --------------------------------------
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Hash {
	#[serde(skip)]
	cache: Cache,
	#[serde(flatten)]
	args: Args,
}

impl Eq for Hash {}
impl PartialEq for Hash {
	fn eq(&self, other: &Self) -> bool {
		self.args == other.args
	}
}

#[async_trait]
#[typetag::serde(name = "hash")]
impl Variable for Hash {
	fn name(&self) -> String {
		self.args.name.clone().unwrap_or(self.typetag_name().to_string())
	}

	fn templates(&self) -> Vec<&Template> {
		if let Some(path) = &self.args.path {
			return vec![path];
		}
		vec![]
	}

	async fn compute(&self, ctx: &ExecutionContext<'_>) -> anyhow::Result<tera::Value> {
		let resource = if let Some(path) = &self.args.path {
			let Some(rendered) = ctx.services.templater.render(path, ctx).await? else {
				bail!("specified path doesn't render to anything")
			};
			let path = PathBuf::from(rendered);
			Resource::from(path)
		} else {
			ctx.scope.resource.clone()
		};

		let hash = self
			.cache
			.try_get_with::<_, std::io::Error>(resource.clone(), async move {
				let content = tokio::fs::read(&resource).await?;
				let hash = sha256::digest(&content);
				Ok(hash)
			})
			.await
			.map_err(|e| anyhow::anyhow!(e.to_string()))?;
		Ok(serde_json::to_value(hash)?)
	}
}
