use anyhow::Result;
use async_trait::async_trait;
use organize_sdk::{
	context::{scope::ExecutionScope, services::fs::manager::DestinationBuilder, ExecutionContext},
	engine::ExecutionModel,
	error::Error,
	plugins::action::{Action, ActionBuilder, Input, Receipt},
	stdx::path::PathBufExt,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub struct Copy(DestinationBuilder);

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub struct CopyBuilder(DestinationBuilder);

#[async_trait]
#[typetag::serde(name = "copy")]
impl ActionBuilder for CopyBuilder {
	async fn build(&self, _ctx: &ExecutionContext<'_>) -> Result<Box<dyn Action>, Error> {
		Ok(Box::new(Copy(self.0.clone())))
	}
}

#[async_trait]
#[typetag::serde(name = "copy")]
impl Action for Copy {
	fn execution_model(&self) -> ExecutionModel {
		ExecutionModel::Batch
	}

	async fn commit(&self, ctx: &ExecutionContext<'_>) -> Result<Receipt, Error> {
		let batch = ctx.scope.batch()?;
		let mut inputs = Vec::new();
		let mut outputs = Vec::new();
		let mut to_resources = Vec::new();

		for from_resource in &batch.files {
			let scope = ExecutionScope::new_resource_scope(ctx.scope.rule()?, from_resource.clone());
			let ctx = ctx.with_scope(scope);
			let to_path = self.0.clone().build(&ctx)?.resolve(&ctx).await?;
			let to_backend = ctx.services.fs.get_provider(&to_path.to_string_lossy())?;
			let to_resource = to_path.as_resource(&ctx, None, to_backend).await;
			to_resources.push(to_resource);
			inputs.push(Input::Processed(from_resource.clone()));
		}

		ctx.services.fs.copy_many(&batch.files, &to_resources).await?;

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
