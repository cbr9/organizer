use crate::{context::ExecutionContext, errors::Error, templates::prelude::*};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Args {
	Name,
	Uid,
	Gid,
	Home,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct User(Args);

#[async_trait]
#[typetag::serde(name = "user")]
impl Variable for User {
	fn name(&self) -> String {
		self.typetag_name().to_string()
	}

	async fn compute(&self, _ctx: &ExecutionContext<'_>) -> Result<serde_json::Value, Error> {
		todo!()
	}
}
