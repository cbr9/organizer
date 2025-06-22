use std::collections::HashMap;

use crate::{
	config::{
		actions::{common::enabled, Input, Output, Receipt, Undo},
		context::ExecutionContext,
	},
	errors::Error,
	resource::Resource,
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

	async fn commit(&self, ctx: &ExecutionContext<'_>) -> Result<Receipt, Error> {
		let receipt = ctx
			.services
			.locker
			.with_locked_destination(ctx, &self.destination, &self.strategy, true, |target| async move {
				let source = ctx.scope.resource;

				let backup: Option<Backup> = if !ctx.settings.dry_run && self.enabled {
					async {
						let backup_opt = if ctx.scope.folder.settings.backup {
							let b = Backup::new(ctx).await?;
							b.persist(ctx.scope.resource.clone(), ctx).await?;
							Some(b)
						} else {
							None
						};

						move_safely(source, &target, ctx).await?;
						Ok(backup_opt)
					}
					.await?
				} else {
					None
				};
				Ok(Receipt {
					inputs: vec![Input::Processed(ctx.scope.resource.clone())],
					outputs: vec![Output::Created(target.clone()), Output::Deleted(ctx.scope.resource.clone())],
					next: vec![target.clone()],
					undo: vec![Box::new(UndoMove {
						original: ctx.scope.resource.clone(),
						new: target,
						backup,
					})],
					metadata: HashMap::new(),
				})
			})
			.await?
			.unwrap_or(Receipt::default());

		Ok(receipt)
	}
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct UndoMove {
	pub original: Resource,
	pub new: Resource,
	pub backup: Option<Backup>,
}

#[typetag::serde(name = "undo_move")]
impl Undo for UndoMove {
	fn undo(&self) -> Result<()> {
		Ok(std::fs::rename(&self.new, &self.original)?)
	}

	fn backup(&self) -> Option<&Backup> {
		self.backup.as_ref()
	}
}
