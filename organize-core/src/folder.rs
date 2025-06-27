use std::{
	collections::HashSet,
	path::{Path, PathBuf},
	sync::Arc,
};

use anyhow::{Context as ErrorContext, Result};
use futures::{stream, StreamExt};
use serde::{Deserialize, Serialize};

use crate::{context::ExecutionContext, options::OptionsBuilder, resource::Resource, stdx::path::PathExt};

use super::options::{Options, Target};

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
#[serde(deny_unknown_fields)]
pub struct FolderBuilder {
	pub root: PathBuf,
	#[serde(flatten)]
	pub settings: OptionsBuilder,
}

impl FolderBuilder {
	pub fn build(self, index: usize, defaults: &OptionsBuilder, rule_options: &OptionsBuilder) -> Result<Folder> {
		let options = Options::compile(defaults, rule_options, &self.settings);
		Ok(Folder {
			path: self.root,
			settings: options,
			index,
		})
	}
}

/// The final, compiled `Folder` object, ready for execution.
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct Folder {
	pub index: usize,
	pub path: PathBuf,
	pub settings: Options,
}

impl Folder {
	async fn find_all_files(&self, min_depth: usize, max_depth: usize, ctx: &ExecutionContext<'_>) -> Result<Vec<PathBuf>> {
		let mut collected_paths = Vec::new();
		let mut dirs_to_visit: Vec<(PathBuf, usize)> = vec![];

		let start_path = &self.path;

		let excluded_paths_vec = self.get_excluded_paths(ctx).await;
		let excluded_paths_set: HashSet<PathBuf> = excluded_paths_vec.into_iter().collect();

		if excluded_paths_set.contains(start_path) {
			tracing::warn!("Start directory '{}' is in the excluded paths. Aborting search.", start_path.display());
			return Ok(Vec::new());
		}

		if min_depth == 0 {
			collected_paths.push(start_path.clone());
		}

		dirs_to_visit.push((start_path.clone(), 0));

		while let Some((current_dir, current_depth)) = dirs_to_visit.pop() {
			if current_depth >= max_depth {
				continue;
			}

			let mut entries = match tokio::fs::read_dir(&current_dir).await {
				Ok(e) => e,
				Err(e) => {
					eprintln!("Warning: Could not read directory {}: {}", current_dir.display(), e);
					continue;
				}
			};

			while let Some(entry) = entries.next_entry().await? {
				let path = entry.path();
				let next_depth = current_depth + 1;

				// --- Exclusion Logic for encountered paths (files or directories) ---
				if excluded_paths_set.contains(&path) {
					eprintln!("Excluding path: {}", path.display());
					// If it's a directory, we effectively prune the branch.
					// If it's a file, we just don't collect it.
					continue;
				}
				// --- End Exclusion Logic ---

				// Only add path if it's within the specified depth range
				if next_depth >= min_depth && next_depth <= max_depth {
					collected_paths.push(path.clone());
				}

				// If it's a directory and still within max_depth, add it to dirs_to_visit
				// (after checking for exclusion, which is done above)
				if path.is_dir() && next_depth < max_depth {
					dirs_to_visit.push((path, next_depth));
				}
			}
		}

		Ok(collected_paths)
	}

	pub async fn get_resources(&self, ctx: &ExecutionContext<'_>) -> Result<Vec<Arc<Resource>>> {
		let concurrency_limit = 50;
		let home = &dirs::home_dir().context("unable to find home directory")?;
		let min_depth = {
			let base = if &self.path == home { 1.0 as usize } else { self.settings.min_depth };
			(base as f64).max(1.0) as usize
		};

		let max_depth = if &self.path == home { 1.0 as usize } else { self.settings.max_depth };

		let all_files = self.find_all_files(min_depth, max_depth, ctx).await?;
		let resource_creation_futures = all_files.into_iter().filter(|e| self.filter_entries(e)).map(|e| {
			// Capture `e` by moving it into the async block
			// Capture `ctx` by reference (or clone/Arc if its lifetime is an issue)
			let ctx_ref = ctx;
			async move {
				e.as_resource(ctx_ref).await // Returns Result<Resource, AsResourceError>
			}
		});

		Ok(stream::iter(resource_creation_futures) // stream::iter expects an Iterator<Item=Future>
			.buffer_unordered(concurrency_limit) // Execute Futures concurrently
			.collect()
			.await)
	}

	async fn get_excluded_paths(&self, ctx: &ExecutionContext<'_>) -> Vec<PathBuf> {
		let concurrency_limit = 50; // Adjust as needed

		let futures_to_render = self
			.settings
			.exclude
			.clone()
			.into_iter()
			.map(|t| async move { t.render(ctx).await.ok() });

		// 2. Use `StreamExt::buffer_unordered` (or `buffered`) on this iterator of futures.
		// This takes the iterator of futures and runs `concurrency_limit` of them concurrently.
		let exclude: Vec<PathBuf> = stream::iter(futures_to_render) // stream::iter now takes an Iterator<Item=Future>
			.buffer_unordered(concurrency_limit)
			.filter_map(|opt_s| async move { opt_s }) // `filter_map` takes Future<Output=Option<U>>, so we just pass through the Option<String>
			.map(PathBuf::from) // Convert String to PathBuf
			.collect()
			.await;

		exclude
	}

	fn filter_entries(&self, path: &Path) -> bool {
		if path.is_file() && self.settings.target == Target::Folders {
			return false;
		}
		if path.is_dir() && self.settings.target == Target::Files {
			return false;
		}

		if path.is_file() {
			if let Some(extension) = path.extension() {
				let partial_extensions = &["crdownload", "part", "download"];
				if partial_extensions.contains(&&*extension.to_string_lossy()) && !self.settings.partial_files {
					return false;
				}
			}
			if path.is_hidden().unwrap_or(false) && !self.settings.hidden_files {
				return false;
			}
		}
		true
	}
}

pub type Folders = Vec<FolderBuilder>;
