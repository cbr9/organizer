use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use organize_sdk::{
	context::{
		services::fs::manager::{Destination, DestinationBuilder},
		ExecutionContext,
	},
	error::Error,
	plugins::action::{Action, ActionBuilder, Receipt},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ContinueWith {
	Original,
	#[default]
	New,
	Both,
}

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub struct Copy {
	#[serde(flatten)]
	destination: Destination,
	#[serde(default)]
	continue_with: ContinueWith,
}

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub struct CopyBuilder {
	#[serde(flatten)]
	destination: DestinationBuilder,
	#[serde(default)]
	continue_with: ContinueWith,
}

#[async_trait]
#[typetag::serde(name = "copy")]
impl ActionBuilder for CopyBuilder {
	async fn build(&self, ctx: &ExecutionContext) -> Result<Box<dyn Action>, Error> {
		let CopyBuilder { destination, continue_with } = self;
		let destination = destination.build(ctx).await?;
		Ok(Box::new(Copy {
			destination,
			continue_with: continue_with.clone(),
		}))
	}
}

#[async_trait]
#[typetag::serde(name = "copy")]
impl Action for Copy {
	async fn commit(&self, ctx: Arc<ExecutionContext>) -> Result<Receipt, Error> {
		let res = ctx.scope.resource()?;
		match ctx.services.fs.copy(&res, &self.destination, &ctx).await? {
			Some((new, undo)) => {
				let next = match self.continue_with {
					ContinueWith::Original => vec![res],
					ContinueWith::New => vec![new],
					ContinueWith::Both => vec![res, new],
				};
				Ok(Receipt { next, undo: vec![undo] })
			}
			None => Ok(Receipt {
				next: vec![res],
				undo: vec![],
			}),
		}
	}

	fn needs_content(&self) -> bool {
		true
	}
}
