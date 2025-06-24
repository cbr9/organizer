use std::collections::HashMap;

use crate::{
	config::{
		actions::{common::enabled, Input, Output, Receipt, Undo},
		context::ExecutionContext,
	},
	errors::Error,
	resource::Resource,
	utils::{self, backup::Backup, fs::r#move},
};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::templates::template::Template;

use super::{common::ConflictResolution, Action, UndoConflict, UndoError, UndoSettings};

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

				let backup = if !ctx.settings.dry_run && self.enabled {
					let backup = Backup::new(ctx).await?;
					backup.persist(ctx).await?;
					r#move(source, &target, ctx).await?;
					Some(backup)
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

#[async_trait]
#[typetag::serde(name = "undo_move")]
impl Undo for UndoMove {
	async fn undo(&self, settings: &UndoSettings) -> Result<(), UndoError> {
		let original = if tokio::fs::try_exists(&self.original).await? {
			if settings.interactive {
				UndoConflict::resolve(self.original.clone()).await
			} else {
				settings.on_conflict.handle(self.original.clone()).await
			}
		} else {
			Ok(Some(self.original.clone()))
		}?;

		if let Some(original) = original {
			if tokio::fs::try_exists(&self.new).await.unwrap_or(false) {
				utils::fs::move_file(&self.new, &original).await?;
				return Ok(());
			}
			if let Some(backup) = &self.backup {
				if tokio::fs::try_exists(backup.as_path()).await.unwrap_or(false) {
					utils::fs::move_file(backup, &original).await?;
					return Ok(());
				}
			}
		}

		Ok(())
	}

	fn backup(&self) -> Option<&Backup> {
		self.backup.as_ref()
	}

	async fn verify(&self) -> Result<(), UndoError> {
		let backup_exists = if let Some(backup) = &self.backup {
			tokio::fs::try_exists(backup.as_path())
				.await
				.map_err(|_e| UndoError::BackupMissing(backup.0.clone()))?
		} else {
			false
		};

		let new_exists = tokio::fs::try_exists(&self.new).await.unwrap_or(false);

		if !new_exists && !backup_exists {
			return Err(UndoError::PathNotFound(self.new.clone()));
		}

		Ok(())
	}
}
