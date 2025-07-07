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
pub struct Symlink {
    #[serde(flatten)]
    destination: Destination,
}

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub struct SymlinkBuilder {
    #[serde(flatten)]
    destination: DestinationBuilder,
}

#[async_trait]
#[typetag::serde(name = "symlink")]
impl ActionBuilder for SymlinkBuilder {
    async fn build(&self, ctx: &ExecutionContext) -> Result<Box<dyn Action>, Error> {
        let destination = self.destination.build(ctx).await?;
        Ok(Box::new(Symlink { destination }))
    }
}

#[async_trait]
#[typetag::serde(name = "symlink")]
impl Action for Symlink {
    async fn commit(&self, ctx: Arc<ExecutionContext>) -> Result<Receipt, Error> {
        let res = ctx.scope.resource()?;
        let new = ctx.services.fs.symlink(&res, &self.destination, &ctx).await?;

        Ok(Receipt {
            next: vec![new],
            ..Default::default()
        })
    }
}
