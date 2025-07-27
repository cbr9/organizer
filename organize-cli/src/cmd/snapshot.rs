use anyhow::Context;
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose};
use clap::{Args, ValueEnum};
use organize_sdk::{
	context::{ExecutionContext, services::fs::connections::Connections, settings::RunSettings},
	engine::{rule::RuleBuilder, stage::StageBuilder},
	location::options::Target as SdkTarget,
	stdx::path::PathExt,
};

use organize_std::storage::vfs::{VfsEntryConfig, VfsEntryType};
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

	/// The path to the output directory where snapshot JSON files will be saved.
	/// Multiple files will be created, one per search stage.
	#[arg(long, short = 'o')]
	pub output_path: PathBuf, // Changed from output to output_dir

	/// Optional: Include actual file content in the snapshot (Base64 encoded).
	/// Warning: This can make snapshot files very large.
	#[arg(long)]
	pub include_content: bool,
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
		let rule_content = tokio::fs::read_to_string(&self.rule)
			.await
			.context(format!("Failed to read rule file: {}", self.rule.display()))?;
		let rule_builder: RuleBuilder = toml::from_str(&rule_content) // Assuming `toml` is imported and in workspace deps
			.context("Failed to parse rule file as TOML")?;

		// 3. Iterate through pipeline stages to find 'search' stages
		let mut vfs_entries_config: Vec<VfsEntryConfig> = Vec::new();
		ctx.services.reporter.info("Processing rule pipeline for search stages...");

		for (idx, stage_builder) in rule_builder.pipeline.into_iter().enumerate() {
			// Check if it's a Search stage
			if let StageBuilder::Search(location_builder, ..) = stage_builder {
				ctx.services.reporter.info(&format!("Found search stage #{}", idx + 1));
				let compiled_location = location_builder.build(&ctx).await?; // This compiles path and options

				// Get the real StorageProvider for this host
				let provider = ctx.services.fs.get_provider(&compiled_location.host)?;

				// Discover resources from the real host for this search stage
				ctx.services.reporter.info(&format!(
					"Discovering resources for stage #{} from host '{}' at path '{}'...",
					idx + 1,
					compiled_location.host,
					compiled_location.path.display()
				));
				// The discover method needs the compiled Location struct
				let discovered_resources = provider.discover(&compiled_location, &ctx).await?;
				ctx.services.reporter.info(&format!(
					"Discovered {} resources for stage #{}. Collecting content for snapshot...",
					discovered_resources.len(),
					idx + 1
				));

				// Ensure the root path itself is added to the snapshot if it's a directory
				let root_metadata = provider.metadata(&compiled_location.path, &ctx).await?;
				if root_metadata.is_dir {
					vfs_entries_config.push(VfsEntryConfig {
						path: compiled_location.path.normalize(),
						entry_type: VfsEntryType::Dir,
						content: None,
						size: root_metadata.size,
						host: compiled_location.host.clone(),
					});
				}

				// Collect VfsEntryConfig data for discovered resources
				for resource in discovered_resources {
					if resource.as_path() == compiled_location.path.as_path() && root_metadata.is_dir {
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
						if self.include_content && metadata.is_file {
							let bytes = resource.get_bytes(&ctx).await?;
							size_from_content = Some(bytes.len() as u64);
							content_string = Some(general_purpose::STANDARD_NO_PAD.encode(bytes));
						}

						let entry_type = if metadata.is_dir { VfsEntryType::Dir } else { VfsEntryType::File };

						vfs_entries_config.push(VfsEntryConfig {
							path: resource.as_path().normalize(),
							entry_type,
							content: content_string,
							size: if self.include_content { size_from_content } else { metadata.size },
							host: resource.host.clone(),
						});
					}
				}
			}
		}

		let output_str = serde_json::to_string_pretty(&vfs_entries_config).context("Failed to serialize snapshot")?;

		tokio::fs::write(&self.output_path, output_str.as_bytes())
			.await
			.context("Failed to write snapshot")?;

		ctx.services.reporter.success(&format!(
			"Created {} with {} entries",
			self.output_path.display(),
			vfs_entries_config.len()
		));
		Ok(())
	}
}
