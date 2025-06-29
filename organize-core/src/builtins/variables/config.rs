use crate::{context::ExecutionContext, errors::Error, templates::prelude::*};
use anyhow::Result;
use async_trait::async_trait;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, IntoEnumIterator};

#[derive(Debug, Clone, EnumIter, Display, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
enum Args {
	Path,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Config {
	#[serde(flatten)]
	args: Args,
}

#[async_trait]
#[typetag::serde(name = "config")]
impl Variable for Config {
	fn name(&self) -> String {
		self.typetag_name().to_string()
	}

	async fn compute(&self, ctx: &ExecutionContext<'_>) -> Result<serde_json::Value, Error> {
		let config = ctx.scope.config()?;
		match &self.args {
			Args::Path => Ok(serde_json::to_value(&config.path.get().unwrap())?),
		}
	}
}
