use anyhow::Result;
use async_trait::async_trait;
use organize_sdk::{
	context::{services::fs::manager::DestinationBuilder, ExecutionContext},
	error::Error,
	plugins::action::{Action, ActionBuilder, Receipt},
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
	async fn commit(&self, ctx: &ExecutionContext<'_>) -> Result<Receipt, Error> {
		let from = ctx.scope.resource()?;
		let to = self.0.clone().build(ctx)?.resolve(ctx).await?;
		let to_backend = ctx.services.fs.get_provider(&to)?;
		let to = to.as_resource(ctx, None, to_backend).await;

		ctx.services.fs.copy(&from, &to, ctx).await?;

		let receipt = Receipt {
			inputs: vec![organize_sdk::plugins::action::Input::Processed(from.clone())],
			outputs: vec![organize_sdk::plugins::action::Output::Created(to)],
			..Default::default()
		};
		Ok(receipt)
	}
}
