use std::{
	fmt::{Debug, Formatter},
	fs::Metadata,
	io::{Read, Write},
	net::TcpStream,
	path::{Path, PathBuf},
	sync::Arc,
};

use anyhow::Result;
use async_trait::async_trait;
use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use ssh2::{Session, Sftp as SftpClient};
use tempfile::NamedTempFile;

use organize_sdk::{
	context::ExecutionContext,
	error::Error,
	location::{options::Options, Location},
	plugins::storage::StorageProvider,
	resource::Resource,
	stdx::path::PathBufExt,
};
use tokio::sync::OnceCell;

#[derive(Serialize, Deserialize)]
pub struct Sftp {
	pub address: String,
	pub username: String,
	pub private_key: Option<PathBuf>,
	#[serde(skip)]
	sftp: OnceCell<SftpClient>,
}

impl Debug for Sftp {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Sftp")
			.field("address", &self.address)
			.field("username", &self.username)
			.finish()
	}
}

impl Clone for Sftp {
	fn clone(&self) -> Self {
		Sftp {
			address: self.address.clone(),
			username: self.username.clone(),
			private_key: self.private_key.clone(),
			sftp: OnceCell::new(),
		}
	}
}

impl Sftp {
	pub async fn get_sftp(&self) -> Result<&SftpClient> {
		self.sftp
			.get_or_try_init(|| async {
				let address = self.address.clone();
				let username = self.username.clone();
				let private_key = self.private_key.clone();
				let tcp = TcpStream::connect(address)?;
				let mut session = Session::new()?;
				session.set_tcp_stream(tcp);
				session.handshake()?;
				if let Some(private_key) = private_key {
					session.userauth_pubkey_file(&username, None, &private_key, None)?;
				} else {
					let mut agent = session.agent()?;
					agent.connect()?;
					agent.list_identities()?;
					for identity in agent.identities()? {
						if agent.userauth(&username, &identity).is_ok() {
							break;
						}
					}
				}
				Ok(session.sftp()?)
			})
			.await
	}
}

impl PartialEq for Sftp {
	fn eq(&self, other: &Self) -> bool {
		self.address == other.address && self.username == other.username && self.private_key == other.private_key
	}
}

impl Eq for Sftp {}

#[async_trait]
#[typetag::serde(name = "sftp")]
impl StorageProvider for Sftp {
	async fn home(&self) -> Result<PathBuf, Error> {
		let sftp = self.get_sftp().await?;
		let path = sftp.realpath(Path::new("."))?;
		Ok(path)
	}

	fn prefix(&self) -> &'static str {
		"sftp"
	}

	async fn metadata(&self, _path: &Path) -> Result<Metadata, Error> {
		Err(Error::ImpossibleOp("SFTP does not support std::fs::Metadata".to_string()))
	}

	async fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, Error> {
		let sftp = self.get_sftp().await?;
		let entries = sftp.readdir(path)?;
		let paths = entries.into_iter().map(|(path, _)| path).collect();
		Ok(paths)
	}

	async fn read(&self, path: &Path) -> Result<Vec<u8>, Error> {
		let sftp = self.get_sftp().await?;
		let mut file = sftp.open(path)?;
		let mut buf = Vec::new();
		file.read_to_end(&mut buf)?;
		Ok(buf)
	}

	async fn write(&self, path: &Path, content: &[u8]) -> Result<(), Error> {
		let sftp = self.get_sftp().await?;
		let mut file = sftp.create(path)?;
		file.write_all(content)?;
		Ok(())
	}

	async fn discover(&self, location: &Location, ctx: &ExecutionContext<'_>) -> Result<Vec<Arc<Resource>>, Error> {
		let mut files = Vec::new();
		let backend = ctx.services.fs.get_provider(&location.path)?;
		self.discover_recursive(ctx, location.path.clone(), 1, &location.options, &mut files, location, backend)
			.await?;
		Ok(files)
	}

	async fn mkdir(&self, path: &Path, _ctx: &ExecutionContext<'_>) -> Result<(), Error> {
		let sftp = self.get_sftp().await?;
		sftp.mkdir(path, 0o755)?;
		Ok(())
	}

	async fn r#move(&self, from: &Path, to: &Path, ctx: &ExecutionContext<'_>) -> Result<(), Error> {
		let sftp = self.get_sftp().await?;
		let rename_result = sftp.rename(from, to, None);
		if rename_result.is_err() {
			tracing::warn!(
				"Could not move {} to {}. Falling back to copy and delete. Original error: {:?}",
				from.display(),
				to.display(),
				rename_result.err()
			);
			self.copy(from, to, ctx).await?;
			self.delete(from).await?;
		}
		Ok(())
	}

	async fn copy(&self, from: &Path, to: &Path, _ctx: &ExecutionContext<'_>) -> Result<(), Error> {
		let content = self.read(from).await?;
		self.write(to, &content).await
	}

	async fn delete(&self, path: &Path) -> Result<(), Error> {
		let sftp = self.get_sftp().await?;
		let stat = sftp.stat(path)?;
		if stat.is_dir() {
			sftp.rmdir(path)?;
		} else {
			sftp.unlink(path)?;
		}
		Ok(())
	}

	async fn download(&self, from: &Path) -> Result<PathBuf, Error> {
		let content = self.read(from).await?;
		let temp_file = NamedTempFile::new().map_err(|e| Error::Io(e))?;
		tokio::fs::write(temp_file.path(), &content).await.map_err(|e| Error::Io(e))?;
		let path = temp_file.keep().map_err(|e| Error::Other(e.into()))?;
		Ok(path.1)
	}

	async fn upload(&self, from_local: &Path, to: &Path, _ctx: &ExecutionContext<'_>) -> Result<(), Error> {
		let content = tokio::fs::read(from_local).await.map_err(|e| Error::Io(e))?;
		self.write(to, &content).await
	}

	async fn hardlink(&self, _from: &Path, _to: &Path, _ctx: &ExecutionContext<'_>) -> Result<(), Error> {
		Err(Error::ImpossibleOp("SFTP does not support hardlinks".to_string()))
	}

	async fn symlink(&self, from: &Path, to: &Path, _ctx: &ExecutionContext<'_>) -> Result<(), Error> {
		let sftp = self.get_sftp().await?;
		sftp.symlink(from, to)?;
		Ok(())
	}
}

impl Sftp {
	fn discover_recursive<'a>(
		&'a self,
		ctx: &'a ExecutionContext<'a>,
		path: PathBuf,
		depth: usize,
		options: &'a Options,
		files: &'a mut Vec<Arc<Resource>>,
		location: &'a Location,
		backend: Arc<dyn StorageProvider>,
	) -> BoxFuture<'a, Result<(), Error>> {
		Box::pin(async move {
			if depth > options.max_depth {
				return Ok(());
			}

			let sftp = self.get_sftp().await?;
			let entries = sftp.readdir(&path)?;

			for (entry_path, stat) in entries {
				let entry_path = sftp.realpath(&entry_path)?;
				if options.exclude.contains(&entry_path) {
					continue;
				}

				if !options.hidden_files && entry_path.file_name().unwrap().to_str().unwrap().starts_with('.') {
					continue;
				}

				if stat.is_dir() {
					if depth >= options.min_depth {
						if let organize_sdk::location::options::Target::Folders = options.target {
							let resource = entry_path
								.clone()
								.as_resource(ctx, Some(Arc::new(location.clone())), backend.clone())
								.await;
							files.push(resource);
						}
					}
					self.discover_recursive(ctx, entry_path, depth + 1, options, files, location, backend.clone())
						.await?;
				} else {
					if depth >= options.min_depth {
						if options.target.is_files() {
							let resource = entry_path
								.as_resource(ctx, Some(Arc::new(location.clone())), backend.clone())
								.await;
							files.push(resource);
						}
					}
				}
			}
			Ok(())
		})
	}
}
