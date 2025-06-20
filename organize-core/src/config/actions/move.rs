use std::path::PathBuf;

use crate::{
	config::{
		actions::{common::enabled, Contract, Undo},
		context::ExecutionContext,
	},
	errors::Error,
	utils::{backup::Backup, fs::move_safely},
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

	async fn execute(&self, ctx: &ExecutionContext<'_>) -> Result<Contract, Error> {
		let receipt = ctx
			.services
			.blackboard
			.locker
			.with_locked_destination(ctx, &self.destination, &self.strategy, true, |target: PathBuf| async move {
				let source = ctx.scope.resource.path();
				let backup = Backup::new(source, ctx).await?;
				if !ctx.settings.dry_run && self.enabled {
					move_safely(source, &target, ctx).await?;
				}

				let target = ctx.scope.resource.with_new_path(target);

				Ok(Contract {
					created: vec![target.clone()],
					deleted: vec![ctx.scope.resource.clone()],
					current: vec![target.clone()],
					undo: vec![Box::new(UndoMove {
						original: ctx.scope.resource.path().to_path_buf(),
						new: target.path().to_path_buf(),
						backup,
					})],
				})
			})
			.await?
			.unwrap_or(Contract::default());

		Ok(receipt)
	}
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct UndoMove {
	pub original: PathBuf,
	pub new: PathBuf,
	pub backup: Backup,
}

#[typetag::serde(name = "undo_move")]
impl Undo for UndoMove {
	fn undo(&self) -> Result<()> {
		Ok(std::fs::rename(&self.new, &self.original)?)
	}

	fn backup(&self) -> Option<&Backup> {
		Some(&self.backup)
	}
}
