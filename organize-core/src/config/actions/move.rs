use std::{path::PathBuf, sync::Arc};

use crate::{
	config::{
		actions::{common::enabled, Contract, Undo},
		context::ExecutionContext,
	},
	errors::{Error, ErrorContext},
};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::templates::template::Template;

use super::{common::ConflictResolution, Action};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Move {
	#[serde(default, rename = "to")]
	pub destination: Template,
	#[serde(default, rename = "if_exists")]
	pub strategy: ConflictResolution,
	#[serde(default = "enabled")]
	enabled: bool,
}

#[async_trait]
#[typetag::serde(name = "move")]
impl Action for Move {
	fn templates(&self) -> Vec<&Template> {
		vec![&self.destination]
	}

	#[tracing::instrument(ret)]
	async fn execute(&self, ctx: &ExecutionContext<'_>) -> Result<Contract, Error> {
		let logic = |target: PathBuf| async move {
			if !ctx.settings.dry_run && self.enabled {
				tokio::fs::rename(ctx.scope.resource.path(), &target)
					.await
					.map_err(|e| Error::Io {
						source: e,
						path: ctx.scope.resource.path().to_path_buf(),
						target: Some(target.clone().to_path_buf()),
						context: ErrorContext::from_scope(&ctx.scope),
					})?;
			}
			let target_path = target.to_path_buf();

			Ok(Contract {
				created: vec![ctx.scope.resource.with_new_path(target_path.clone()).into()],
				deleted: vec![ctx.scope.resource.clone()],
				undo: vec![Box::new(UndoMove {
					original: ctx.scope.resource.path().to_path_buf(),
					new: target_path,
				})],
			})
		};

		let receipt = ctx
			.services
			.blackboard
			.locker
			.with_locked_destination(&ctx, &self.destination, &self.strategy, true, logic)
			.await?
			.unwrap_or(Contract::default());

		Ok(receipt)
	}
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct UndoMove {
	pub original: PathBuf,
	pub new: PathBuf,
}

#[typetag::serde(name = "undo_move")]
impl Undo for UndoMove {
	fn undo(&self) -> Result<()> {
		Ok(std::fs::rename(&self.new, &self.original)?)
	}

	fn describe(&self) -> String {
		todo!()
	}
}
