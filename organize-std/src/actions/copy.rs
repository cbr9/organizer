use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use organize_sdk::{
	context::{
		services::fs::manager::{Destination, DestinationBuilder},
		ExecutionContext,
	},
	error::Error,
	plugins::action::{Action, ActionBuilder, Input, Output, Receipt},
	stdx::path::PathBufExt,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub struct Copy {
	#[serde(flatten)]
	destination: Destination,
}

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub struct CopyBuilder {
	#[serde(flatten)]
	destination: DestinationBuilder,
}

#[async_trait]
#[typetag::serde(name = "copy")]
impl ActionBuilder for CopyBuilder {
	async fn build(&self, ctx: &ExecutionContext) -> Result<Box<dyn Action>, Error> {
		let destination = self.destination.build(ctx).await?;
		Ok(Box::new(Copy { destination }))
	}
}

#[async_trait]
#[typetag::serde(name = "copy")]
impl Action for Copy {
	async fn commit(&self, ctx: Arc<ExecutionContext>) -> Result<Receipt, Error> {
		let res = ctx.scope.resource()?;
		let backend = ctx.services.fs.get_provider(&self.destination.host)?;
		let dest = self
			.destination
			.resolve(&ctx)
			.await?
			.as_resource(&ctx, None, self.destination.host.clone(), backend)
			.await;

		if !ctx.settings.dry_run {
			ctx.services.fs.copy(&res, &dest, &ctx).await?;
		}

		let receipt = Receipt {
			inputs: vec![Input::Processed(res.clone())],
			outputs: vec![Output::Created(dest.clone())],
			next: vec![dest.clone()],
			..Default::default()
		};

		Ok(receipt)
	}
}
