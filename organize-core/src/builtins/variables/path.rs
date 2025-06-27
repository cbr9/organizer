use crate::{context::ExecutionContext, errors::Error, templates::prelude::*};
use anyhow::Result;
use async_trait::async_trait;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, IntoEnumIterator};

#[derive(Debug, Clone, Deserialize, Display, Serialize, EnumIter, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
enum Args {
	Stem,
	Extension,
	Name,
	Path,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct File(Option<Args>);

#[async_trait]
#[typetag::serde(name = "file")]
impl Variable for File {
	fn name(&self) -> String {
		self.typetag_name().to_string()
	}

	async fn compute(&self, ctx: &ExecutionContext<'_>) -> Result<serde_json::Value, Error> {
		let Some(arg) = &self.0 else {
			return Err(Error::TemplateError(TemplateError::MissingField {
				variable: self.name(),
				fields: Args::iter().join(", "),
			}));
		};
		let resource = ctx.scope.resource()?;
		match arg {
			Args::Stem => Ok(serde_json::to_value(resource.as_path().file_stem().unwrap().to_string_lossy())?),
			Args::Extension => Ok(serde_json::to_value(resource.as_path().extension().unwrap().to_string_lossy())?),
			Args::Name => todo!(),
			Args::Path => todo!(),
		}
	}
}
