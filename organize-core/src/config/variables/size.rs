use std::path::PathBuf;

use anyhow::bail;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{
	config::{context::ExecutionContext, variables::Variable},
	resource::Resource,
	templates::template::Template,
};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Default)]
enum Unit {
	#[default]
	#[serde(rename = "lowercase")]
	Bytes,
	KiB,
	MiB,
	GiB,
	TiB,
	PiB,
	EiB,
	ZiB,
	YiB,
	KB,
	MB,
	GB,
	TB,
	PB,
	EB,
	ZB,
	YB,
}

impl Unit {
	pub fn value(&self) -> f64 {
		match self {
			Unit::Bytes => 1.0,
			Unit::KiB => 1024.0_f64.powi(1),
			Unit::MiB => 1024.0_f64.powi(2),
			Unit::GiB => 1024.0_f64.powi(3),
			Unit::TiB => 1024.0_f64.powi(4),
			Unit::PiB => 1024.0_f64.powi(5),
			Unit::EiB => 1024.0_f64.powi(6),
			Unit::ZiB => 1024.0_f64.powi(7),
			Unit::YiB => 1024.0_f64.powi(8),
			Unit::KB => 1000.0_f64.powi(1),
			Unit::MB => 1000.0_f64.powi(2),
			Unit::GB => 1000.0_f64.powi(3),
			Unit::TB => 1000.0_f64.powi(4),
			Unit::PB => 1000.0_f64.powi(5),
			Unit::EB => 1000.0_f64.powi(6),
			Unit::ZB => 1000.0_f64.powi(7),
			Unit::YB => 1000.0_f64.powi(8),
		}
	}
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone, Default)]
struct Args {
	unit: Unit,
	input: Option<Template>,
	name: Option<String>,
}

#[derive(Debug, Clone)]
struct Cache(moka::future::Cache<Resource, f64>);

impl std::ops::Deref for Cache {
	type Target = moka::future::Cache<Resource, f64>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl Default for Cache {
	fn default() -> Self {
		Self(moka::future::Cache::new(10_000))
	}
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Size {
	#[serde(skip)]
	cache: Cache,
	#[serde(flatten, default)]
	args: Args,
}

impl PartialEq for Size {
	fn eq(&self, other: &Self) -> bool {
		self.args == other.args
	}
}

impl Eq for Size {}

#[async_trait]
#[typetag::serde(name = "size")]
impl Variable for Size {
	fn name(&self) -> String {
		self.args.name.clone().unwrap_or(self.typetag_name().to_string())
	}

	fn templates(&self) -> Vec<&Template> {
		if let Some(input) = &self.args.input {
			return vec![input];
		}
		vec![]
	}

	async fn compute(&self, ctx: &ExecutionContext<'_>) -> anyhow::Result<tera::Value> {
		let resource = if let Some(path) = &self.args.input {
			let Some(rendered) = ctx.services.templater.render(path, ctx).await? else {
				bail!("specified path doesn't render to anything")
			};
			let path = PathBuf::from(rendered);
			Resource::from(path)
		} else {
			ctx.scope.resource.clone()
		};

		let bytes = self
			.cache
			.try_get_with::<_, std::io::Error>(resource.clone(), async move {
				let metadata = tokio::fs::metadata(&resource).await?;
				Ok(metadata.len() as f64)
			})
			.await
			.map_err(|e| anyhow::anyhow!(e.to_string()))?;

		Ok(serde_json::to_value(bytes / self.args.unit.value())?)
	}
}
