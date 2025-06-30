use async_trait::async_trait;
use clap::ValueEnum;
use dialoguer::{theme::ColorfulTheme, Input as RenameInput, Select};
use serde::{Deserialize, Serialize};
use std::{
	collections::HashMap,
	ffi::OsStr,
	fmt::Debug,
	path::Path,
	sync::Arc,
};
use strum::{Display, EnumIter, IntoEnumIterator};

use anyhow::Result;
use dyn_clone::DynClone;
use dyn_eq::DynEq;
use std::path::PathBuf;
use thiserror::Error;

use crate::{context::ExecutionContext, engine::ExecutionModel, errors::Error, resource::Resource, utils::backup::Backup};

#[derive(Debug, Serialize, Deserialize, Clone, Display)]
#[serde(rename_all = "lowercase")]
pub enum Input {
	Processed(Arc<Resource>),
	Skipped(Arc<Resource>),
}

#[derive(Debug, Serialize, Deserialize, Clone, Display)]
#[serde(rename_all = "lowercase")]
pub enum Output {
	Created(Arc<Resource>),
	Deleted(Arc<Resource>),
	Modified(Arc<Resource>),
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Receipt {
	pub inputs: Vec<Input>,
	pub outputs: Vec<Output>,
	pub next: Vec<Arc<Resource>>,
	pub undo: Vec<Box<dyn Undo>>,
	pub metadata: HashMap<String, serde_json::Value>,
}

impl From<String> for Receipt {
	fn from(value: String) -> Self {
		serde_json::from_str(&value).expect("Could not convert string to Receipt object")
	}
}

dyn_clone::clone_trait_object!(Undo);
dyn_eq::eq_trait_object!(Undo);

#[derive(Debug, Error)]
pub enum UndoError {
	#[error("Path '{0}' does not exist, but is required for the undo operation.")]
	PathNotFound(PathBuf),

	#[error("Path '{0}' already exists. The undo operation would overwrite it.")]
	PathAlreadyExists(PathBuf),

	#[error("Backup file is missing for path '{0}'. Cannot restore.")]
	BackupMissing(PathBuf),

	#[error("Parent directory '{0}' for the restore path does not exist.")]
	ParentDirectoryNotFound(PathBuf),

	#[error("Any error")]
	Anyhow(#[from] anyhow::Error),

	#[error("IO error")]
	IO(#[from] std::io::Error),

	#[error("Undo aborted by the user")]
	Abort,
}

#[async_trait]
#[typetag::serde(tag = "type")]
pub trait Undo: Debug + DynEq + DynClone + Send + Sync {
	async fn undo(&self, settings: &UndoSettings) -> Result<(), Error>;

	fn backup(&self) -> Option<&Backup> {
		None
	}

	async fn verify(&self) -> Result<(), Error>;
}

pub struct UndoSettings {
	pub interactive: bool,
	pub on_conflict: UndoConflict,
}

#[derive(Clone, Debug, ValueEnum, EnumIter, Display)]
#[strum(serialize_all = "snake_case")]
pub enum UndoConflict {
	Skip,
	Abort,
	Overwrite,
	AutoRename,
	Rename,
}

async fn suggest_new_path(resource: Resource) -> Result<Resource> {
	let parent = resource.as_path().parent().unwrap_or_else(|| Path::new(""));
	let stem = resource.as_path().file_stem().unwrap_or_else(|| OsStr::new("file"));
	let extension = resource.as_path().extension().unwrap_or_else(|| OsStr::new(""));

	let mut count = 1;
	loop {
		// 2. Construct the new filename purely from strings and path components.
		let new_filename_str = format!("{} ({}).{}", stem.to_string_lossy(), count, extension.to_string_lossy());

		// 3. Create a new PathBuf to check for existence. This does not touch the original `resource`.
		let new_path = parent.join(&new_filename_str);

		if !tokio::fs::try_exists(&new_path).await? {
			// 4. Once a valid path is found, consume the original `resource` exactly once
			//    to create the final, evolved struct and return it. The loop is guaranteed to terminate here.
			return Ok(resource.with_path(new_path));
		}
		count += 1;
	}
}

impl UndoConflict {
	/// This new method encapsulates all the conflict handling logic.
	/// It takes a mutable reference to the destination to allow the Rename variant to change it.
	pub async fn resolve(resource: Resource) -> Result<Option<Resource>, UndoError> {
		let choices: Vec<Self> = Self::iter().collect();
		let strategy: &UndoConflict = Select::with_theme(&ColorfulTheme::default())
			.with_prompt(format!("Destination '{}' already exists.", resource.path.display()))
			.items(&choices)
			.interact()
			.map(|choice| &choices[choice])
			.expect("Unknown option");
		strategy.handle(resource).await
	}

	pub async fn handle(&self, resource: Resource) -> Result<Option<Resource>, UndoError> {
		match self {
			UndoConflict::Overwrite => {
				// The logic for overwriting the destination file.
				if resource.as_path().is_file() {
					tokio::fs::remove_file(resource.as_path()).await?;
				} else {
					tokio::fs::remove_dir_all(resource.as_path()).await?;
				}
				Ok(Some(resource))
			}
			UndoConflict::Rename => {
				// The logic for prompting the user and renaming the destination.
				let theme = ColorfulTheme::default();
				let input = RenameInput::<String>::with_theme(&theme)
					.with_prompt("Enter a new name for the destination")
					.with_initial_text(format!("{}", resource.as_path().file_name().unwrap_or_default().display()))
					.interact_text()
					.map(PathBuf::from)?;
				let new = resource.with_path(input);
				Ok(Some(new))
			}
			UndoConflict::Skip => Ok(None),
			UndoConflict::Abort => Err(UndoError::Abort),
			UndoConflict::AutoRename => Ok(Some(suggest_new_path(resource).await?)),
		}
	}
}

dyn_clone::clone_trait_object!(Action);
dyn_eq::eq_trait_object!(Action);

#[async_trait]
#[typetag::serde(tag = "type")]
pub trait Action: DynEq + DynClone + Sync + Send + Debug {
	fn execution_model(&self) -> ExecutionModel {
		ExecutionModel::default()
	}
	async fn commit(&self, _ctx: &ExecutionContext<'_>) -> Result<Receipt, Error>;
}
