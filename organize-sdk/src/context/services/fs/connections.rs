use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path, sync::Arc};
use tokio::fs; // For reading the file
use toml;

use crate::{error::Error, plugins::storage::StorageProvider, PROJECT_NAME};

#[derive(Debug, Clone, Deserialize, Default, Serialize)]
pub struct Connections {
	pub backends: HashMap<String, Arc<dyn StorageProvider>>,
}

impl std::ops::Deref for Connections {
	type Target = HashMap<String, Arc<dyn StorageProvider>>;

	fn deref(&self) -> &Self::Target {
		&self.backends
	}
}

impl Connections {
	pub async fn from_file(path: &Path) -> Result<Self, Error> {
		let content = fs::read_to_string(path)
			.await
			.map_err(Error::Io) // Convert std::io::Error to your custom Error::Io
			.context(format!("Failed to read connections file: {}", path.display()))?; // Add context for clarity

		Ok(toml::from_str(&content)
			.map_err(|e| Error::Config(format!("Failed to parse connections TOML: {e}"))) // Convert toml::de::Error to your custom Error::Config
			.context(format!("Invalid connections file format: {}", path.display()))?)
	}

	pub async fn from_config_dir() -> Result<Self, Error> {
		let config_base_dir = dirs::config_dir().context("Could not determine OS-specific config directory.")?;
		let connections_file_path = config_base_dir.join(PROJECT_NAME).join("connections.toml");

		let content = match fs::read_to_string(&connections_file_path).await {
			Ok(c) => c,

			Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
				tracing::info!(
					"No connections file found at {}. Using default (empty) connections.",
					connections_file_path.display()
				);
				return Ok(Self::default());
			}

			Err(e) => {
				return Err(Error::Io(e)).context(format!("Failed to read connections file: {}", connections_file_path.display()))?;
			}
		};

		Ok(toml::from_str(&content).context(format!("Invalid connections file format: {}", connections_file_path.display()))?)
	}
}
