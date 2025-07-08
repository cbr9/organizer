use async_trait::async_trait;
use base64::prelude::*;
use bytes::Bytes;
use dashmap::DashMap;
use organize_sdk::{
	context::{services::fs::resource::Resource, ExecutionContext},
	error::Error,
	location::{options::Target, Location},
	plugins::storage::{BackendType, Metadata, StorageProvider},
	stdx::path::PathExt,
};
use serde::{Deserialize, Serialize};
use std::{
	collections::HashMap,
	fmt::Debug,
	path::{Path, PathBuf},
	sync::Arc,
	time::SystemTime,
};
use tokio::{fs, sync::OnceCell};
use uuid::Uuid;

use anyhow::{Context, Result};
use futures::{future::BoxFuture, stream::BoxStream, TryStreamExt};
// Represents a file within the VFS
#[derive(Debug, Clone)]
pub struct VfsFile {
	pub metadata: Metadata, // Simulated metadata (size, timestamps, etc.)
	pub host: String,
	pub content_source: Option<PathBuf>,
}

// Represents a directory within the VFS
#[derive(Debug, Clone)]
pub struct VfsDir {
	// Children map: name of file/dir -> its VfsEntry
	pub children: HashMap<String, VfsEntry>,
	pub metadata: Metadata, // Added Metadata for directories for consistency
	pub host: String,
}

// Enum to differentiate between files and directories in the VFS tree
#[derive(Debug, Clone)]
pub enum VfsEntry {
	File(VfsFile),
	Dir(VfsDir),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")] // Enables "file" or "dir" in config
pub enum VfsEntryType {
	#[default]
	File,
	Dir,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct VfsEntryConfig {
	/// The absolute path of the VFS entry (e.g., "/my/simulated/file.txt").
	pub path: PathBuf,
	/// The type of entry: "file" or "dir". Defaults to "file".
	#[serde(default)]
	pub entry_type: VfsEntryType,

	pub content_source: Option<PathBuf>,
	/// Optional size for files. If provided, overrides size derived from `content`.
	pub size: Option<u64>,
	pub host: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VirtualFileSystem {
	#[serde(skip)]
	vfs_root: OnceCell<DashMap<PathBuf, VfsEntry>>,
	simulated_host: String,

	#[serde(default)] // Allows this field to be omitted in configuration (will default to empty Vec)
	pub initial_vfs_state: Vec<VfsEntryConfig>,
	pub snapshot: PathBuf,
}

impl PartialEq for VirtualFileSystem {
	fn eq(&self, _other: &Self) -> bool {
		true
	}
}

impl Eq for VirtualFileSystem {}

fn is_partial_in_vfs(path: &Path) -> bool {
	if let Some(extension) = path.extension() {
		let partial_extensions = &["crdownload", "part", "download"];
		if partial_extensions.contains(&&*extension.to_string_lossy()) {
			return true;
		}
	}
	false
}

impl VirtualFileSystem {
	async fn _discover_recursive(
		&self, // A reference to the DryRunFileSystem instance
		current_path: PathBuf,
		current_depth: usize,
		location: Arc<Location>,
		results: &mut Vec<Arc<Resource>>,
		backend: Arc<dyn StorageProvider>,
		ctx: &ExecutionContext,
	) -> Result<(), Error> {
		// 1. Check if the current path exists in the VFS.
		let entry_opt = self.get_vfs_entry(&current_path, ctx).await;
		let entry = match entry_opt {
			Some(e) => e,
			None => {
				// If the path doesn't exist in the VFS, stop here for this branch.
				return Ok(());
			}
		};

		let entry_original_host = match entry {
			VfsEntry::File(ref file) => &file.host,
			VfsEntry::Dir(ref dir) => &dir.host,
		};

		// If the resource's original host does not match the host specified in the current Location, skip it.
		// This ensures that discover for a specific host only yields files from that simulated host.
		if entry_original_host != &location.host {
			return Ok(());
		}

		// Determine if the current entry is a file or a directory.
		let is_dir = matches!(entry, VfsEntry::Dir(_));
		let is_file = matches!(entry, VfsEntry::File(_));

		// 2. Apply discovery filters to the current entry.

		// Filter by max_depth: If it's a directory and current depth is at or beyond max_depth,
		// we stop traversing *into* this directory (don't list its children).
		// max_depth of 0 typically means no limit, or only the root if min_depth is also 0.
		if is_dir && location.options.max_depth > 0 && current_depth >= location.options.max_depth {
			return Ok(());
		}

		// Apply `exclude` filter: If the current path is explicitly excluded, skip it and its children.
		if location.options.exclude.contains(&current_path) {
			return Ok(());
		}

		// Apply `hidden_files` filter: Skip hidden files/directories if not desired.
		let is_hidden = current_path
			.file_name()
			.and_then(|name| name.to_str())
			.is_some_and(|name_str| name_str.starts_with(".")); // Simple check for .开头 files/folders
		if !location.options.hidden_files && is_hidden {
			return Ok(());
		}

		// Apply `partial_files` filter: (Assuming VFS files are not "partial" unless explicitly marked).
		// Add specific VfsFile state/metadata to simulate partial files if needed.
		// For now, no specific logic for partial files based on VFS state.
		if !location.options.partial_files && matches!(entry, VfsEntry::File(_)) && is_partial_in_vfs(&current_path) {
			return Ok(());
		}

		// Determine if the current entry matches the `target` type (Files or Folders).
		let matches_target = match location.options.target {
			Target::Files => is_file,
			Target::Folders => is_dir,
		};

		// 3. Add to results if filters and min_depth criteria are met.
		// Only add to `results` if it matches the target type and is at or below the minimum depth.
		if matches_target && current_depth >= location.options.min_depth {
			let resource = Arc::new(Resource::new(
				current_path.as_path(),
				"local".to_string(),
				Some(location.clone()),
				backend.clone(),
			));
			results.push(resource);
		}

		// 4. Recurse into subdirectories if the current entry is a directory.
		if is_dir {
			// Use the `read_dir` method of DryRunFileSystem (already implemented)
			let children_paths = self.read_dir(&current_path, ctx).await?;
			for child_path in children_paths {
				Box::pin(self._discover_recursive(child_path, current_depth + 1, location.clone(), results, backend.clone(), ctx)).await?;
			}
		}
		Ok(())
	}

	/// Private helper to lazily load and populate the VFS root (DashMap).
	/// This method is called by all other StorageProvider methods that need the VFS.
	async fn _get_populated_vfs_root(&self, ctx: &ExecutionContext) -> Result<&DashMap<PathBuf, VfsEntry>, Error> {
		self.vfs_root
			.get_or_try_init(|| async {
				let map = DashMap::new();
				let current_time = SystemTime::now();

				// --- Determine the VFS entries to load (from snapshot or inline config) ---
				let vfs_entries_to_load: Vec<VfsEntryConfig> = {
					let snapshot = self.snapshot.join("snapshot.json");
					// If a snapshot file is specified, read and deserialize it.
					if !snapshot.exists() {
						let desc = format!("{} does not exist", snapshot.display());
						ctx.services.reporter.error(&desc, None);
						return Err(Error::Io(std::io::Error::new(std::io::ErrorKind::NotFound, desc)));
					}
					tracing::info!("Loading VFS state from snapshot file: {}", snapshot.display());
					let content = fs::read_to_string(&snapshot)
						.await
						.map_err(Error::Io)
						.context(format!("Failed to read VFS snapshot file: {}", snapshot.display()))?;
					serde_json::from_str(&content)
						.map_err(Error::Json)
						.context(format!("Invalid VFS snapshot JSON format in file: {}", snapshot.display()))?
				};

				// Populate the DashMap from `merged_entries`
				for config_entry in vfs_entries_to_load {
					self.ensure_vfs_parent_dirs_exist(&config_entry.path, ctx).await?;

					let metadata = Metadata {
						size: config_entry.size,
						modified: Some(current_time),
						created: Some(current_time),
						is_dir: config_entry.entry_type == VfsEntryType::Dir,
						is_file: config_entry.entry_type == VfsEntryType::File,
						extra: HashMap::new(),
					};

					let vfs_entry = match config_entry.entry_type {
						VfsEntryType::File => VfsEntry::File(VfsFile {
							content_source: config_entry.content_source,
							metadata,
							host: config_entry.host.clone(),
						}),
						VfsEntryType::Dir => VfsEntry::Dir(VfsDir {
							children: HashMap::new(),
							metadata,
							host: config_entry.host.clone(),
						}),
					};
					map.insert(config_entry.path.normalize(), vfs_entry);
				}
				Ok(map)
			})
			.await
	}

	async fn get_vfs_entry(&self, path: &Path, ctx: &ExecutionContext) -> Option<VfsEntry> {
		// Ensure the VFS is loaded before accessing it.
		let vfs_map = match self._get_populated_vfs_root(ctx).await {
			Ok(map) => map,
			Err(_) => return None, // Or log the error
		};
		vfs_map.get(path).map(|entry_ref| entry_ref.value().clone())
	}

	/// Private helper to insert or update a `VfsEntry` at a given path.
	/// This requires a write lock on the root.
	async fn insert_vfs_entry(&self, path: PathBuf, entry: VfsEntry, ctx: &ExecutionContext) {
		let vfs_map = match self._get_populated_vfs_root(ctx).await {
			Ok(map) => map,
			Err(_) => return, // Or handle the error appropriately
		};
		vfs_map.insert(path, entry);
	}

	/// Private helper to remove a `VfsEntry` from the VFS.
	/// This requires a write lock on the root.
	async fn remove_vfs_entry(&self, path: &Path, ctx: &ExecutionContext) {
		let vfs_map = match self._get_populated_vfs_root(ctx).await {
			Ok(map) => map,
			Err(_) => return, // Or handle the error appropriately
		};
		vfs_map.remove(path);
	}

	async fn append_to_vfs_file(&self, path: &Path, content: &[u8], ctx: &ExecutionContext) -> Result<(), Error> {
		let vfs_map = self._get_populated_vfs_root(ctx).await?;
		let normalized_path = path.normalize();
		self.ensure_vfs_parent_dirs_exist(&normalized_path, ctx).await?;

		let now = SystemTime::now();

		// Try to get a mutable reference to the existing entry
		if let Some(mut entry_ref) = vfs_map.get_mut(&normalized_path) {
			match *entry_ref.value_mut() {
				VfsEntry::File(ref mut file) => {
					// Read existing content from its companion file
					let current_content_vec = match &file.content_source {
						Some(source_path) => fs::read(source_path).await.map_err(Error::from)?,
						None => Vec::new(), // File exists but has no content
					};
					let mut combined_content = current_content_vec;
					combined_content.extend_from_slice(content);

					// Write to a new companion file for the updated content
					let unique_filename = Uuid::new_v4().to_string();
					let new_companion_path = self.snapshot.join("content").join(&unique_filename);
					fs::write(&new_companion_path, &combined_content).await.map_err(Error::from)?;

					file.content_source = Some(new_companion_path); // Update VfsFile to point to new companion
					file.metadata.size = Some(combined_content.len() as u64);
					file.metadata.modified = Some(now);

					fs::remove_file(path).await.map_err(Error::from)?;
				}
				VfsEntry::Dir(_) => {
					return Err(Error::Io(std::io::Error::new(
						std::io::ErrorKind::IsADirectory,
						format!("Path is a directory: {}", normalized_path.display()),
					)));
				}
			}
		} else {
			// File does not exist, create a new one
			let new_content_vec = content.to_vec();
			let unique_filename = Uuid::new_v4().to_string();
			let new_companion_path = self.snapshot.join("content").join(&unique_filename);
			fs::write(&new_companion_path, &new_content_vec).await.map_err(Error::from)?;

			let new_metadata = Metadata {
				size: Some(new_content_vec.len() as u64),
				modified: Some(now),
				created: Some(now),
				is_dir: false,
				is_file: true,
				extra: HashMap::new(),
			};

			let new_vfs_file = VfsFile {
				content_source: Some(new_companion_path),
				metadata: new_metadata,
				host: self.simulated_host.clone(),
			};
			self.insert_vfs_entry(normalized_path, VfsEntry::File(new_vfs_file), ctx).await;
		}

		Ok(())
	}

	/// Private helper to ensure all parent directories for a given path exist in the VFS.
	/// It creates them as `VfsDir` entries if they don't exist.
	fn ensure_vfs_parent_dirs_exist<'a>(&'a self, path: &'a Path, ctx: &'a ExecutionContext) -> BoxFuture<'a, Result<(), Error>> {
		Box::pin(async move {
			// Ensure VFS is loaded before traversing/modifying.
			let vfs_map = self._get_populated_vfs_root(ctx).await?;

			let parent_path_opt = path.parent();
			let parent = match parent_path_opt {
				Some(p) => p,
				None => return Ok(()),
			};

			let mut current_path_buf = PathBuf::new();
			let mut components = parent.components().peekable();

			if let Some(component) = components.peek() {
				if matches!(component, &std::path::Component::RootDir) || matches!(component, std::path::Component::Prefix(_)) {
					current_path_buf.push(component);
					if !vfs_map.contains_key(&current_path_buf) {
						vfs_map.insert(
							// Use the loaded map
							current_path_buf.clone(),
							VfsEntry::Dir(VfsDir {
								children: HashMap::new(),
								host: self.simulated_host.clone(),
								metadata: Metadata {
									size: None,
									modified: Some(SystemTime::now()),
									created: Some(SystemTime::now()),
									is_dir: true,
									is_file: false,
									extra: HashMap::new(),
								},
							}),
						);
					}
					components.next();
				}
			}

			for component in components {
				current_path_buf.push(component);

				if let Some(entry_ref) = vfs_map.get(&current_path_buf) {
					// Use the loaded map
					if let VfsEntry::Dir(_) = entry_ref.value() {
						// Directory already exists, continue
					} else {
						return Err(Error::Io(std::io::Error::new(
							std::io::ErrorKind::AlreadyExists,
							format!("Path already exists as a file: {}", current_path_buf.display()),
						)));
					}
				} else {
					vfs_map.insert(
						// Use the loaded map
						current_path_buf.clone(),
						VfsEntry::Dir(VfsDir {
							children: HashMap::new(),
							host: self.simulated_host.clone(),
							metadata: Metadata {
								size: None,
								modified: Some(SystemTime::now()),
								created: Some(SystemTime::now()),
								is_dir: true,
								is_file: false,
								extra: HashMap::new(),
							},
						}),
					);
				}
			}
			Ok(())
		})
	}
}

#[async_trait]
#[typetag::serde(name = "vfs")]
impl StorageProvider for VirtualFileSystem {
	fn kind(&self) -> BackendType {
		BackendType::Local // Simulate as a local backend for now
	}

	async fn home(&self) -> Result<PathBuf, Error> {
		Ok(PathBuf::from("/")) // Simulate root as home directory
	}

	fn prefix(&self) -> &'static str {
		"vfs"
	}

	// --- Phase 2: Fundamental StorageProvider Operations against VFS ---

	async fn try_exists(&self, path: &Path, ctx: &ExecutionContext) -> Result<bool, Error> {
		Ok(self.get_vfs_entry(path, ctx).await.is_some())
	}

	async fn metadata(&self, path: &Path, ctx: &ExecutionContext) -> Result<Metadata, Error> {
		match self.get_vfs_entry(path, ctx).await {
			Some(VfsEntry::File(file)) => Ok(file.metadata),
			Some(VfsEntry::Dir(dir)) => Ok(dir.metadata),
			None => Err(Error::Io(std::io::Error::new(
				std::io::ErrorKind::NotFound,
				format!("File could not be found: {}", path.display()),
			))),
		}
	}

	async fn write(&self, path: &Path, content: &[u8], ctx: &ExecutionContext) -> Result<(), Error> {
		self.ensure_vfs_parent_dirs_exist(path, ctx).await?;

		let now = SystemTime::now();
		let size = content.len() as u64;

		let new_metadata = Metadata {
			size: Some(size),
			modified: Some(now),
			created: Some(now), // For new files, created is 'now'
			is_dir: false,
			is_file: true,
			extra: HashMap::new(),
		};

		let vfs_file = VfsFile {
			content: Some(content.to_vec()),
			metadata: new_metadata,
			host: self.simulated_host.clone(),
		};
		self.insert_vfs_entry(path.to_path_buf(), VfsEntry::File(vfs_file), ctx).await;

		// Hash field for Resource will be uninitialized until retrieved, as decided.
		Ok(())
	}

	fn upload<'a>(
		&'a self,
		to: &'a Path,
		stream: BoxStream<'a, Result<Bytes, Error>>,
		ctx: &'a ExecutionContext,
	) -> BoxFuture<'a, Result<(), Error>> {
		Box::pin(async move {
			// Ensure the file is empty or created before appending.
			self.write(to, &[], ctx).await?;
			stream
				.try_for_each(|bytes_chunk| async move {
					self.append_to_vfs_file(to, &bytes_chunk, ctx).await?;
					Ok(())
				})
				.await?;
			Ok(())
		})
	}

	fn download<'a>(&'a self, path: &'a Path, ctx: &'a ExecutionContext) -> BoxStream<'a, Result<Bytes, Error>> {
		Box::pin(async_stream::try_stream! {
			let entry = self.get_vfs_entry(path, ctx).await;
			match entry {
				Some(VfsEntry::File(file)) => {
					if let Some(content_vec) = file.content {
						// For simplicity, yield content as a single chunk.
						// For large simulated files, you'd yield in smaller chunks.
						yield Bytes::from(content_vec);
					}
				}
				Some(VfsEntry::Dir(_)) => {
					// Path is a directory, not a file to download. Yield no content.
				}
				None => {
					// File not found. The download stream will just end.
					// If you want to signal "not found" via the stream, you'd need to yield an error here.
					// For now, an empty stream implies nothing to download.
				}
			}
		})
	}

	// --- Placeholders for methods to be implemented in later phases ---

	async fn read_dir(&self, path: &Path, ctx: &ExecutionContext) -> Result<Vec<PathBuf>, Error> {
		// 1. Check if the given path exists in the VFS and if it represents a directory.
		match self.get_vfs_entry(path, ctx).await {
			Some(VfsEntry::File(_)) => {
				// If the path points to a file, it's an error as you cannot read a directory's contents from a file path.
				return Err(Error::Io(std::io::Error::new(
					std::io::ErrorKind::NotADirectory,
					format!("Path is a file, not a directory: {}", path.display()),
				)));
			}
			Some(VfsEntry::Dir(_)) => {
				// The path exists and is a directory. Proceed to list its children.
			}
			None => {
				// The path does not exist in the VFS, so its contents cannot be read.
				return Err(Error::Io(std::io::Error::new(
					std::io::ErrorKind::NotFound,
					format!("File could not be found: {}", path.display()),
				)));
			}
		}

		// 2. Collect all direct children of the specified path from the `vfs_root` DashMap.
		// We iterate through all entries currently stored in the VFS and check if their parent path matches the `path` argument.
		let vfs_root = match self._get_populated_vfs_root(ctx).await {
			Ok(map) => map,
			Err(_) => return Ok(vec![]), // Or log the error
		};
		let mut children = Vec::new();
		for entry_ref in vfs_root.iter() {
			let (child_path, _) = entry_ref.pair(); // Get the (path, entry) pair from the DashMap iterator

			// Check if the current entry's parent matches the directory path we are trying to read.
			// This identifies direct children.
			if child_path.parent() == Some(path) {
				children.push(child_path.clone()); // Add the child's full path to the result vector
			}
		}
		Ok(children)
	}

	async fn read(&self, path: &Path, ctx: &ExecutionContext) -> Result<Vec<u8>, Error> {
		self.download(path, ctx)
			.try_fold(Vec::new(), |mut acc, bytes_chunk| async move {
				acc.extend_from_slice(&bytes_chunk);
				Ok(acc)
			})
			.await
	}

	async fn rename(&self, from: &Path, to: &Path, ctx: &ExecutionContext) -> Result<(), Error> {
		let entry_to_move = match self.get_vfs_entry(from, ctx).await {
			Some(entry) => entry,
			None => {
				return Err(Error::Io(std::io::Error::new(
					std::io::ErrorKind::NotFound,
					format!("File could not be found: {}", from.display()),
				)))
			}
		};

		// 2. Check if the destination path already exists.
		// If it does, a rename typically fails if the target exists, or overwrites.
		// For dry run, explicit AlreadyExists might be clearer.
		if self.get_vfs_entry(to, ctx).await.is_some() {
			return Err(Error::Io(std::io::Error::new(
				std::io::ErrorKind::AlreadyExists,
				format!("Destination path already exists: {}", to.display()),
			)));
		}

		// 3. Ensure all parent directories for the destination path exist in the VFS.
		self.ensure_vfs_parent_dirs_exist(to, ctx).await?;

		// 4. Remove the entry from its original location.
		self.remove_vfs_entry(from, ctx).await;

		// 5. Update metadata for the moved entry (e.g., modified timestamp).
		let now = SystemTime::now();
		let updated_entry = match entry_to_move {
			VfsEntry::File(mut file) => {
				file.metadata.modified = Some(now);
				VfsEntry::File(file)
			}
			VfsEntry::Dir(mut dir) => {
				// Directories might not have all metadata, ensure consistency
				dir.metadata.modified = Some(now);
				VfsEntry::Dir(dir)
			}
		};

		self.insert_vfs_entry(to.to_path_buf(), updated_entry, ctx).await;

		Ok(())
	}

	async fn copy(&self, from: &Path, to: &Path, ctx: &ExecutionContext) -> Result<(), Error> {
		// 1. Check if the source path exists and is a file in the VFS.
		let from_entry = match self.get_vfs_entry(from, ctx).await {
			Some(VfsEntry::File(file)) => file,
			Some(VfsEntry::Dir(_)) => {
				// Cannot copy a directory using file copy. For recursive copy, a different method is needed.
				return Err(Error::Io(std::io::Error::new(
					std::io::ErrorKind::IsADirectory,
					format!("Source for copy is a directory: {}", from.display()),
				)));
			}
			None => {
				// Source file not found.
				return Err(Error::Io(std::io::Error::new(
					std::io::ErrorKind::NotFound,
					format!("File could not be found: {}", from.display()),
				)));
			}
		};

		// 2. Ensure all parent directories for the destination path exist in the VFS.
		self.ensure_vfs_parent_dirs_exist(to, ctx).await?;

		// 3. Create a new VfsFile entry for the destination.
		// Copy the content and update metadata.
		let now = SystemTime::now();
		let mut new_metadata = from_entry.metadata.clone();
		new_metadata.modified = Some(now); // New modification time for the copy
		new_metadata.created = Some(now); // New creation time for the copy

		let new_vfs_file = VfsFile {
			content: from_entry.content.clone(), // Clone the content (Vec<u8> is deep-copied)
			metadata: new_metadata,
			host: self.simulated_host.clone(),
		};

		// 4. Insert the new VfsFile at the destination path.
		// This will overwrite if 'to' already exists, which is standard copy behavior.
		self.insert_vfs_entry(to.to_path_buf(), VfsEntry::File(new_vfs_file), ctx).await;

		Ok(())
	}

	async fn delete(&self, path: &Path, ctx: &ExecutionContext) -> Result<(), Error> {
		let vfs_map = self._get_populated_vfs_root(ctx).await?;
		let normalized_path = path.normalize();

		let entry = match self.get_vfs_entry(&normalized_path, ctx).await {
			Some(entry) => entry,
			None => {
				return Err(Error::Io(std::io::Error::new(
					std::io::ErrorKind::NotFound,
					format!("File could not be found: {}", normalized_path.display()),
				)));
			}
		};

		self.remove_vfs_entry(&normalized_path, ctx).await;
		Ok(())
	}

	async fn mk_parent(&self, path: &Path, ctx: &ExecutionContext) -> Result<(), Error> {
		// Delegate to the internal helper we already implemented.
		self.ensure_vfs_parent_dirs_exist(path, ctx).await
	}

	async fn hardlink(&self, from: &Path, to: &Path, ctx: &ExecutionContext) -> Result<(), Error> {
		// 1. Check if the source path exists and is a file in the VFS.
		let from_entry = match self.get_vfs_entry(from, ctx).await {
			Some(VfsEntry::File(file)) => file,
			Some(VfsEntry::Dir(_)) => {
				// Hardlinks typically cannot be created for directories.
				return Err(Error::Io(std::io::Error::new(
					std::io::ErrorKind::PermissionDenied, // Or InvalidInput
					format!("Source for hardlink is a directory: {}", from.display()),
				)));
			}
			None => {
				// Source file not found.
				return Err(Error::Io(std::io::Error::new(
					std::io::ErrorKind::NotFound,
					format!("File could not be found: {}", from.display()),
				)));
			}
		};

		// 2. Check if the destination path already exists in the VFS.
		if self.get_vfs_entry(to, ctx).await.is_some() {
			return Err(Error::Io(std::io::Error::new(
				std::io::ErrorKind::AlreadyExists,
				format!("Destination for hardlink already exists: {}", to.display()),
			)));
		}

		// 3. Ensure all parent directories for the destination path exist in the VFS.
		self.ensure_vfs_parent_dirs_exist(to, ctx).await?;

		// 4. Create a new VfsFile entry for the destination.
		// For simulation, we'll give it a clone of the source's content.
		// This is not a true inode-level hardlink, but functionally simulates the copy.
		let now = SystemTime::now();
		let mut new_metadata = from_entry.metadata.clone();
		new_metadata.modified = Some(now); // The link itself has a new modified time
		new_metadata.created = Some(now); // The link itself has a new creation time

		let new_vfs_file = VfsFile {
			content: from_entry.content.clone(), // Clone the content (Vec<u8> is deep-copied)
			metadata: new_metadata,
			host: self.simulated_host.clone(),
		};

		// 5. Insert the new VfsFile at the destination path.
		self.insert_vfs_entry(to.to_path_buf(), VfsEntry::File(new_vfs_file), ctx).await;

		Ok(())
	}

	async fn symlink(&self, from: &Path, to: &Path, ctx: &ExecutionContext) -> Result<(), Error> {
		// 1. Check if the source path exists in the VFS.
		// Symlinks can point to non-existent targets in real file systems,
		// but for a dry run simulation, we might enforce existence for clarity.
		// Here, we check for its existence in the VFS.
		if self.get_vfs_entry(from, ctx).await.is_none() {
			return Err(Error::Io(std::io::Error::new(
				std::io::ErrorKind::NotFound,
				format!("File could not be found: {}", from.display()),
			)));
		}

		// 2. Check if the destination path already exists in the VFS.
		if self.get_vfs_entry(to, ctx).await.is_some() {
			return Err(Error::Io(std::io::Error::new(
				std::io::ErrorKind::AlreadyExists,
				format!("Destination for symlink already exists: {}", to.display()),
			)));
		}

		// 3. Ensure all parent directories for the destination path exist in the VFS.
		self.ensure_vfs_parent_dirs_exist(to, ctx).await?;

		// 4. Create a new VfsFile entry for the destination (the symlink itself).
		// The content of the symlink file is the path it points to.
		let now = SystemTime::now();
		let from_str = from.to_string_lossy();
		let symlink_content = from_str.as_bytes();
		let symlink_size = symlink_content.len() as u64;

		let new_metadata = Metadata {
			size: Some(symlink_size), // Size of the path string it points to
			modified: Some(now),
			created: Some(now),
			is_dir: false, // The symlink itself is a file
			is_file: true, // The symlink itself is a file
			extra: {
				let mut map = HashMap::new();
				// Store the target path in extra metadata for later retrieval if needed.
				map.insert("target".into(), from.to_string_lossy().into());
				map
			},
		};

		let new_vfs_file = VfsFile {
			content: Some(symlink_content.to_vec()), // The content of the symlink is its target path
			metadata: new_metadata,
			host: self.simulated_host.clone(),
		};

		// 5. Insert the new VfsFile at the destination path.
		self.insert_vfs_entry(to.to_path_buf(), VfsEntry::File(new_vfs_file), ctx).await;

		Ok(())
	}

	async fn discover(&self, location: &Location, ctx: &ExecutionContext) -> Result<Vec<Arc<Resource>>, Error> {
		let mut discovered_resources = Vec::new();
		// The simulated_backend will be passed to Resource::new, ensuring
		// operations on discovered resources also use this DryRunFileSystem.
		let backend: Arc<dyn StorageProvider> = Arc::new(self.clone());
		let location = Arc::new(location.clone());

		// Start the recursive discovery process from the location's base path.
		self._discover_recursive(
			location.path.clone(),     // The starting path for discovery
			0,                         // Initial depth (root of discovery)
			location,                  // The compiled Options (filters)
			&mut discovered_resources, // Mutable vector to collect results
			backend,                   // Pass the Arc<dyn StorageProvider> clone
			ctx,
		)
		.await?;

		Ok(discovered_resources)
	}
}
