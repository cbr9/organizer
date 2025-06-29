use std::{
	collections::HashSet,
	path::{Path, PathBuf},
	sync::{Arc, OnceLock},
};

use anyhow::{Context as ErrorContext, Result};
use async_trait::async_trait;
use futures::{stream, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::OnceCell;

use crate::{
	context::ExecutionContext,
	errors::Error,
	options::OptionsBuilder,
	resource::Resource,
	stdx::path::PathExt,
	storage::Location,
	templates::prelude::Template,
};

use super::options::{Options, Target};

/// The final, compiled `Folder` object, ready for execution.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct Folder {
	/// The path template for the location, resolved at runtime.
	pub path: Template,
	#[serde(flatten)]
	pub options: OptionsBuilder,
	#[serde(skip)]
	pub compiled_path: OnceCell<PathBuf>,
	#[serde(skip)]
	pub compiled_options: OnceCell<Options>,
}

#[async_trait]
#[typetag::serde(name = "local")]
impl Location for Folder {
	fn partial_options(&self) -> &OptionsBuilder {
		&self.options
	}

	fn options(&self) -> &Options {
		self.compiled_options
			.get()
			.expect("tried to retrieve options before they are initialized")
	}

	fn initialize_options(&self, options: Options) {
		self.compiled_options.set(options).expect("tried to initialize options twice");
	}

	fn initialize_path(&self, path: PathBuf) {
		self.compiled_path.set(path).expect("tried to initialize path twice")
	}

	fn partial_path(&self) -> &Template {
		&self.path
	}

	fn path(&self) -> &PathBuf {
		// let path = self
		// 	.compiled_path
		// 	.get_or_try_init(|| {
		// 		async {
		// 			// This entire block is the initialization future.
		// 			let path_str = self.path.render(ctx).await?;
		// 			let path = PathBuf::from(path_str);
		// 			Ok::<PathBuf, Error>(path)
		// 		}
		// 	})
		// 	.await?;
		// Ok(path)
		self.compiled_path
			.get()
			.expect("tried to retrieve compiled path before it is rendered")
	}

	async fn get_resources(&self, ctx: &ExecutionContext<'_>) -> Result<Vec<Arc<Resource>>, Error> {
		let concurrency_limit = 50;
		let home = &dirs::home_dir().context("unable to find home directory")?;
		let options = self.options();
		let path = self.path();
		let min_depth = {
			let base = if path == home { 1.0 as usize } else { options.min_depth };
			(base as f64).max(1.0) as usize
		};

		let max_depth = if path == home { 1.0 as usize } else { options.max_depth };

		let all_files = self.find_all_files(min_depth, max_depth, ctx).await?;
		let location: Arc<dyn Location> = Arc::new(self.clone());
		let resource_creation_futures = all_files.into_iter().filter(|e| self.filter_entries(e)).map(|e| {
			// Capture `e` by moving it into the async block
			// Capture `ctx` by reference (or clone/Arc if its lifetime is an issue)
			let ctx_ref = ctx;
			let location = location.clone();
			async move {
				e.as_resource(ctx_ref, location).await // Returns Result<Resource, AsResourceError>
			}
		});

		Ok(stream::iter(resource_creation_futures) // stream::iter expects an Iterator<Item=Future>
			.buffer_unordered(concurrency_limit) // Execute Futures concurrently
			.collect()
			.await)
	}
}

impl Folder {
	async fn find_all_files(&self, min_depth: usize, max_depth: usize, ctx: &ExecutionContext<'_>) -> Result<Vec<PathBuf>> {
		let mut collected_paths = Vec::new();
		let mut dirs_to_visit: Vec<(PathBuf, usize)> = vec![];

		let start_path = self.path();

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

	fn filter_entries(&self, path: &Path) -> bool {
		let options = self.options();
		if path.is_file() && options.target == Target::Folders {
			return false;
		}
		if path.is_dir() && options.target == Target::Files {
			return false;
		}

		if path.is_file() {
			if let Some(extension) = path.extension() {
				let partial_extensions = &["crdownload", "part", "download"];
				if partial_extensions.contains(&&*extension.to_string_lossy()) && !options.partial_files {
					return false;
				}
			}
			if path.is_hidden().unwrap_or(false) && !options.hidden_files {
				return false;
			}
		}
		true
	}
}
