use std::{collections::HashMap, sync::Arc};

use crate::{
	action::{Action, Input, Output, Receipt, Undo, UndoConflict, UndoError, UndoSettings},
	common::enabled,
	context::{
		services::fs::manager::{Destination, FileSystemManager},
		ExecutionContext,
	},
	engine::ConflictResolution,
	errors::Error,
	resource::Resource,
	utils::backup::Backup,
};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Move {
	#[serde(flatten)]
	pub destination: Destination,
	#[serde(default, rename = "if_exists")]
	pub strategy: ConflictResolution,
	#[serde(default = "enabled")]
	enabled: bool,
}

#[async_trait]
#[typetag::serde(name = "move")]
impl Action for Move {
	async fn commit(&self, ctx: &ExecutionContext<'_>) -> Result<Receipt, Error> {
		let receipt = ctx
			.services
			.fs
			.locker
			.with_locked_destination(ctx, &self.destination, &self.strategy, |target| async move {
				let source = ctx.scope.resource()?.clone();

				let backup = if !ctx.settings.dry_run && self.enabled {
					let backup = Backup::new(ctx).await?;
					backup.persist(ctx).await?;
					ctx.services.fs.r#move(source.clone(), target.clone()).await?;
					Some(backup)
				} else {
					None
				};

				Ok(Receipt {
					inputs: vec![Input::Processed(source.clone())],
					outputs: vec![Output::Created(target.clone()), Output::Deleted(source.clone())],
					next: vec![target.clone()],
					undo: vec![Box::new(UndoMove {
						original: source.clone(),
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
	pub original: Arc<Resource>,
	pub new: Arc<Resource>,
	pub backup: Option<Backup>,
}

#[async_trait]
#[typetag::serde(name = "undo_move")]
impl Undo for UndoMove {
	async fn undo(&self, settings: &UndoSettings) -> Result<(), Error> {
		let original = if tokio::fs::try_exists(&self.original.path).await? {
			if settings.interactive {
				UndoConflict::resolve(self.original.clone()).await
			} else {
				settings.on_conflict.handle(self.original.clone()).await
			}
		} else {
			Ok(Some(self.original.clone()))
		}?;

		let fs = FileSystemManager::default();

		if let Some(original) = original {
			if tokio::fs::try_exists(self.new.as_path()).await.unwrap_or(false) {
				fs.r#move(self.new.clone(), original).await?;
				return Ok(());
			}
			if let Some(backup) = &self.backup {
				if tokio::fs::try_exists(backup.as_path()).await.unwrap_or(false) {
					fs.r#move(backup.0.clone(), original).await?;
					return Ok(());
				}
			}
		}

		Ok(())
	}

	fn backup(&self) -> Option<&Backup> {
		self.backup.as_ref()
	}

	async fn verify(&self) -> Result<(), Error> {
		let backup_exists = if let Some(backup) = &self.backup {
			tokio::fs::try_exists(backup.as_path())
				.await
				.map_err(|_e| UndoError::BackupMissing(backup.0.to_path_buf()))?
		} else {
			false
		};

		let new_exists = tokio::fs::try_exists(self.new.as_path()).await.unwrap_or(false);

		if !new_exists && !backup_exists {
			return Err(Error::UndoError(UndoError::PathNotFound(self.new.to_path_buf())));
		}

		Ok(())
	}
}
