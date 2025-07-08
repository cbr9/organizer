use anyhow::Context;
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose};
use clap::{Args, ValueEnum};
use organize_sdk::{
	PROJECT_NAME,
	context::{
		ExecutionContext,
		services::fs::{connections::Connections, manager::Destination},
		settings::RunSettings,
	},
	engine::{
		ConflictResolution,
		rule::RuleBuilder,
		stage::{Stage, StageParams},
	},
	location::options::Target as SdkTarget,
	stdx::path::PathExt,
};

use organize_std::storage::vfs::{VfsEntryConfig, VfsEntryType};
use sha2::{Digest, Sha256};
use uuid::Uuid;
// Use VFS config structs
use std::{collections::HashMap, path::PathBuf};

use crate::cli::CliUi;

use super::Cmd; // For Base64 encoding

// Define the command-line arguments for the 'snapshot' subcommand
#[derive(Debug, Args)]
pub struct Snapshot {
	/// Path to the rule configuration file to process (e.g., 'rules/my_rule.toml').
	#[arg(long, short = 'r')]
	pub rule: PathBuf,
}

impl std::fmt::Display for Target {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Target::Files => write!(f, "files"),
			Target::Folders => write!(f, "folders"),
		}
	}
}
#[derive(Debug, Clone, PartialEq, Eq, ValueEnum)] // derive ValueEnum here!
#[clap(rename_all = "lowercase")] // Use clap's rename_all for consistent parsing
pub enum Target {
	Files,
	Folders,
}

// 2. Implement conversion from CliTarget to SdkTarget
impl From<Target> for SdkTarget {
	fn from(cli_target: Target) -> Self {
		match cli_target {
			Target::Files => SdkTarget::Files,
			Target::Folders => SdkTarget::Folders,
		}
	}
}

#[async_trait]
impl Cmd for Snapshot {
	async fn run(self) -> anyhow::Result<()> {
		let settings = RunSettings {
			dry_run: false,
			args: HashMap::new(),
			snapshot: None,
		};
		let connections = Connections::from_config_dir().await?;
		let ui = CliUi::new();

		let ctx = ExecutionContext::new(settings, connections, ui).await?;

		ctx.services
			.reporter
			.info(&format!("Loading rule from '{}'...", self.rule.display()));

		let rule = {
			let rule_content = tokio::fs::read_to_string(&self.rule)
				.await
				.context(format!("Failed to read rule file: {}", self.rule.display()))?;
			let rule_builder: RuleBuilder = toml::from_str(&rule_content) // Assuming `toml` is imported and in workspace deps
				.context("Failed to parse rule file as TOML")?;
			rule_builder.build(&ctx).await?
		};

		let mut include_content = false;
		for stage in &rule.pipeline {
			if let Stage::Filter { filter, .. } = stage {
				if filter.needs_content() {
					include_content = true;
					break;
				}
			}

			if let Stage::Action { action, .. } = stage {
				if action.needs_content() {
					include_content = true;
					break;
				}
			}
		}

		let hash = {
			let mut string_for_hash = rule
				.pipeline
				.iter()
				.filter_map(|stage| match stage {
					Stage::Search { location, params, .. } => {
						let StageParams { enabled, .. } = params;
						let mut serialized = serde_json::to_string(location).unwrap();
						serialized.push_str(&format!("__enabled:{}", enabled));
						Some(serialized)
					}
					_ => None,
				})
				.collect::<Vec<String>>()
				.join("__");

			string_for_hash.push_str(&format!("_include_content:{}", include_content));

			let mut hasher = Sha256::new();
			hasher.update(string_for_hash.as_bytes());
			let rule_content_hash = hasher.finalize();

			format!("{:x}", rule_content_hash)
		};

		let data_dir = {
			let mut data_dir =
				dirs::data_local_dir().ok_or_else(|| anyhow::anyhow!("Could not determine OS-specific data directory for snapshots."))?;
			data_dir = data_dir.join(PROJECT_NAME).join("snapshots").join(&hash);
			tokio::fs::create_dir_all(&data_dir).await?;
			data_dir
		};

		let mut vfs_entries_config: Vec<VfsEntryConfig> = Vec::new();
		ctx.services.reporter.info("Processing rule pipeline for search stages...");

		for (idx, stage) in rule.pipeline.into_iter().enumerate() {
			// Check if it's a Search stage
			if let Stage::Search { location, .. } = stage {
				ctx.services.reporter.info(&format!("Found search stage #{}", idx + 1));

				// Get the real StorageProvider for this host
				let provider = ctx.services.fs.get_provider(&location.host)?;

				// Discover resources from the real host for this search stage
				ctx.services.reporter.info(&format!(
					"Discovering resources for stage #{} from host '{}' at path '{}'...",
					idx + 1,
					location.host,
					location.path.display()
				));
				// The discover method needs the compiled Location struct
				let discovered_resources = provider.discover(&location, &ctx).await?;
				ctx.services.reporter.info(&format!(
					"Discovered {} resources for stage #{}. Collecting content for snapshot...",
					discovered_resources.len(),
					idx + 1
				));

				// Ensure the root path itself is added to the snapshot if it's a directory
				let root_metadata = provider.metadata(&location.path, &ctx).await?;
				if root_metadata.is_dir {
					vfs_entries_config.push(VfsEntryConfig {
						path: location.path.normalize(),
						entry_type: VfsEntryType::Dir,
						size: root_metadata.size,
						host: location.host.clone(),
						content_source: None,
					});
				}

				// Collect VfsEntryConfig data for discovered resources
				for resource in discovered_resources {
					if resource.as_path() == location.path.as_path() && root_metadata.is_dir {
						continue; // Skip if already added as root
					}

					let mut content_string: Option<String> = None;
					let mut size_from_content: Option<u64> = None;

					let metadata = resource.get_metadata(&ctx).await.inspect_err(|e| {
						ctx.services.reporter.warning(&format!(
							"Could not get metadata for resource '{}': {}. Skipping resource in snapshot.",
							resource.as_path().display(),
							e
						))
					});

					if let Ok(metadata) = metadata {
						let mut content_source = None;
						if include_content && metadata.is_file {
							size_from_content = Some(bytes.len() as u64);
							// NEW: Always write to an external companion file
							let unique_filename = Uuid::new_v4();
							let companion_file_full_path = data_dir.join("content").join(&unique_filename.to_string());

							let destination = Destination {
								folder: companion_file_full_path.parent().unwrap().to_string_lossy(),
								host: "local".to_string(),
								filename: companion_file_full_path.file_name().unwrap().to_string_lossy(),
								resolution_strategy: ConflictResolution::Overwrite,
							};

							ctx.services.fs.copy(&resource, to, &ctx).await?;

							content_source = Some(companion_file_full_path);
							size_from_content = Some(bytes.len() as u64);
						}
						vfs_entries_config.push(VfsEntryConfig {
							path: resource.as_path().normalize(),
							entry_type,
							content_source,
							size: if include_content { size_from_content } else { metadata.size },
							host: resource.host.clone(),
						});
					}
				}
			}
		}

		let output_str = serde_json::to_string_pretty(&vfs_entries_config).context("Failed to serialize snapshot")?;
		let snapshot_path = data_dir.join("snapshot.json");

		tokio::fs::write(&snapshot_path, output_str.as_bytes())
			.await
			.context("Failed to write snapshot")?;

		ctx.services
			.reporter
			.success(&format!("Created {} with {} entries", snapshot_path.display(), vfs_entries_config.len()));
		Ok(())
	}
}
