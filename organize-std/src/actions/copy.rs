use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use organize_sdk::{
	context::{
		services::fs::manager::{Destination, DestinationBuilder},
		ExecutionContext,
	},
	engine::ExecutionModel,
	error::Error,
	plugins::action::{Action, ActionBuilder, Input, Receipt},
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
	async fn build(&self, ctx: &ExecutionContext<'_>) -> Result<Box<dyn Action>, Error> {
		let destination = self.destination.build(ctx).await?;
		Ok(Box::new(Copy { destination }))
	}
}

#[async_trait]
#[typetag::serde(name = "copy")]
impl Action for Copy {
	fn execution_model(&self) -> ExecutionModel {
		ExecutionModel::Batch
	}

	async fn commit(&self, ctx: Arc<ExecutionContext<'_>>) -> Result<Receipt, Error> {
		let batch = ctx.scope.batch()?;
		let mut inputs = Vec::new();
		let mut outputs = Vec::new();
		let mut to_resources = Vec::new();

		for from_resource in &batch.files {
			let ctx = ctx.with_resource(from_resource)?;
			let to_path = self.destination.resolve(&ctx).await?;
			let to_backend = ctx.services.fs.get_provider(&self.destination.host)?;
			let to_resource = to_path.as_resource(&ctx, None, self.destination.host.clone(), to_backend).await;
			to_resources.push(to_resource);
			inputs.push(Input::Processed(from_resource.clone()));
		}

		ctx.services.fs.copy_many(&batch.files, &to_resources, ctx).await?;

		for to_resource in to_resources {
			outputs.push(organize_sdk::plugins::action::Output::Created(to_resource));
		}

		let receipt = Receipt {
			inputs,
			outputs,
			next: batch.files.clone(),
			..Default::default()
		};
		Ok(receipt)
	}
}
