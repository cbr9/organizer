use crate::{context::ExecutionContext, errors::Error, templates::prelude::*};
use anyhow::Result;
use async_trait::async_trait;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::{Display, EnumIter, IntoEnumIterator};
use typetag::deserialize;

#[derive(Debug, Clone, Deserialize, EnumIter, Display, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum Args {
	Os(OsArgs),
	Host(HostArgs),
}

#[derive(Debug, Clone, Default, Deserialize, EnumIter, Display, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum OsArgs {
	#[default]
	Name,
	Family,
	Version,
	Arch,
}

#[derive(Debug, Clone, Deserialize, EnumIter, Default, Display, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum HostArgs {
	#[default]
	Name,
	Uptime,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct System {
	#[serde(flatten)]
	args: Args,
}

#[async_trait]
#[typetag::serde(name = "sys")]
impl Variable for System {
	fn name(&self) -> String {
		self.typetag_name().to_string()
	}

	async fn compute(&self, _ctx: &ExecutionContext<'_>) -> Result<serde_json::Value, Error> {
		match &self.args {
			Args::Os(os) => match os {
				OsArgs::Name => todo!(),
				OsArgs::Family => todo!(),
				OsArgs::Version => todo!(),
				OsArgs::Arch => todo!(),
			},
			Args::Host(host) => match host {
				HostArgs::Name => todo!(),
				HostArgs::Uptime => todo!(),
			},
		}
	}
}
