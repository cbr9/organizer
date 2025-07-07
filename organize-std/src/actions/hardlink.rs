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

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub struct Hardlink {
    #[serde(flatten)]
    destination: Destination,
}

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub struct HardlinkBuilder {
    #[serde(flatten)]
    destination: DestinationBuilder,
}

#[async_trait]
#[typetag::serde(name = "hardlink")]
impl ActionBuilder for HardlinkBuilder {
    async fn build(&self, ctx: &ExecutionContext) -> Result<Box<dyn Action>, Error> {
        let destination = self.destination.build(ctx).await?;
        Ok(Box::new(Hardlink { destination }))
    }
}

#[async_trait]
#[typetag::serde(name = "hardlink")]
impl Action for Hardlink {
    async fn commit(&self, ctx: Arc<ExecutionContext>) -> Result<Receipt, Error> {
        let res = ctx.scope.resource()?;
        let new = ctx.services.fs.hardlink(&res, &self.destination, &ctx).await?;

        Ok(Receipt {
            next: vec![new],
            ..Default::default()
        })
    }
}
