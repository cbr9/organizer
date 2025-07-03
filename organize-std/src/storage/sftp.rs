use std::{
	fmt::{Debug, Formatter},
	fs::Metadata,
	net::{IpAddr, SocketAddr},
	path::{Path, PathBuf},
	sync::Arc,
};

use deadpool::managed::{self, Metrics, Object, Pool, RecycleResult};
use russh_sftp::client::SftpSession;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::future::BoxFuture;
use russh::{
	client::{self, Handle},
	keys::{agent::client::AgentClient, Algorithm},
	Channel,
};
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;
use tokio::sync::OnceCell;

use organize_sdk::{
	context::ExecutionContext,
	error::Error,
	location::{options::Options, Location},
	plugins::storage::StorageProvider,
	resource::Resource,
	stdx::path::PathBufExt,
};

#[derive(Serialize, Deserialize)]
pub struct Sftp {
	pub address: IpAddr,
	pub port: u16,
	pub username: String,
	pub private_key: Option<PathBuf>,
	#[serde(skip)]
	pub pool: OnceCell<SftpPool>,
}

impl PartialEq for Sftp {
	fn eq(&self, other: &Self) -> bool {
		self.address == other.address && self.port == other.port && self.username == other.username
	}
}

impl Eq for Sftp {}

impl Debug for Sftp {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Sftp")
			.field("address", &self.address)
			.field("port", &self.port)
			.field("username", &self.username)
			.finish()
	}
}

impl Clone for Sftp {
	fn clone(&self) -> Self {
		Self {
			address: self.address.clone(),
			port: self.port.clone(),
			username: self.username.clone(),
			private_key: self.private_key.clone(),
			pool: self.pool.clone(),
		}
	}
}

pub struct Client;

impl client::Handler for Client {
	type Error = anyhow::Error;

	async fn check_server_key(&mut self, _server_public_key: &russh::keys::PublicKey) -> Result<bool, Self::Error> {
		Ok(true)
	}
}

impl managed::Manager for Sftp {
	type Error = Error;
	type Type = SftpSession;

	/// Creates a new, authenticated SftpSession.
	async fn create(&self) -> Result<SftpSession, Self::Error> {
		let session = self.connect().await?;
		// 3. Open a channel and request the SFTP subsystem
		let channel: Channel<client::Msg> = session.channel_open_session().await.context("Failed to open session channel")?;
		channel
			.request_subsystem(true, "sftp")
			.await
			.context("Failed to request SFTP subsystem")?;

		// 4. Create the SftpSession
		SftpSession::new(channel.into_stream()).await.map_err(|e| Error::SFTP(e))
	}

	/// Checks if a connection is still valid before lending it out.
	async fn recycle(&self, session: &mut Self::Type, _metrics: &Metrics) -> RecycleResult<Self::Error> {
		// A simple, low-cost operation to check if the session is alive.
		match session.canonicalize(".").await {
			Ok(_) => Ok(()),
			Err(e) => {
				tracing::warn!("Recycling SFTP session failed, discarding. Error: {}", e);
				// The error indicates the connection is broken.
				Err(managed::RecycleError::Message(e.to_string().into()))
			}
		}
	}
}

/// The main runtime struct which holds the connection pool.
/// This struct is NOT serializable directly. It is created from an SftpConfig.
pub type SftpPool = Arc<managed::Pool<Sftp>>;

impl Sftp {
	/// Creates a new Sftp provider with a connection pool.
	pub async fn pool(&self) -> Result<&Arc<Pool<Sftp>>, Error> {
		let pool = self
			.pool
			.get_or_try_init(|| async {
				let pool = managed::Pool::builder(self.clone())
					.max_size(5) // Max 16 concurrent connections
					.build()
					.map(|pool| Arc::new(pool))
					.map_err(|e| Error::Other(e.into()))?;
				Ok::<Arc<Pool<Sftp>>, Error>(pool)
			})
			.await?;
		Ok(pool)
	}
}
impl Sftp {
	/// Establishes a full connection, authenticates, and creates an SftpSession.
	/// This is the main change: each public-facing operation will create and tear down
	/// a connection. This is necessary for compatibility with servers that only allow
	/// one SFTP subsystem per connection (e.g., using `ForceCommand`).
	async fn connect(&self) -> Result<Handle<Client>, Error> {
		// 1. Establish the underlying SSH session
		let config = Arc::new(russh::client::Config::default());
		let socket = SocketAddr::new(self.address, self.port);
		let mut session = client::connect(config, socket, Client)
			.await
			.context("Could not establish SSH connection")?;

		// 2. Authenticate using the SSH agent
		let client_pipe = tokio::net::windows::named_pipe::ClientOptions::new()
			.open(r"\\.\pipe\openssh-ssh-agent")
			.context("Could not connect to the SSH agent pipe. Is 1Password or another agent running?")?;

		let hash_alg = session.best_supported_rsa_hash().await?.flatten();

		let mut authenticated = false;
		let mut agent = AgentClient::connect(client_pipe);
		let identities = agent.request_identities().await.unwrap();
		for identity in identities {
			let alg = match identity.algorithm() {
				Algorithm::Dsa | Algorithm::Rsa { .. } => hash_alg,
				_ => None,
			};

			let auth_result = session
				.authenticate_publickey_with(&self.username, identity, alg, &mut agent)
				.await
				.unwrap();
			if auth_result.success() {
				tracing::debug!("Authenticated successfully with SSH agent.");
				authenticated = true;
				break;
			}
		}

		if !authenticated {
			return Err(Error::Other(anyhow::anyhow!(
				"Authentication failed: No valid keys found in the agent for the given user."
			)));
		}
		Ok(session)
	}
}

#[async_trait]
#[typetag::serde(name = "sftp")]
impl StorageProvider for Sftp {
	async fn home(&self) -> Result<PathBuf, Error> {
		let session = self.pool().await?.get().await.map_err(|e| Error::Other(e.into()))?;
		let path = session.canonicalize(".").await?;
		Ok(path.into())
	}

	fn prefix(&self) -> &'static str {
		"sftp"
	}

	async fn metadata(&self, _path: &Path) -> Result<Metadata, Error> {
		Err(Error::ImpossibleOp("SFTP does not support std::fs::Metadata".to_string()))
	}

	async fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, Error> {
		let session = self.pool().await?.get().await.map_err(|e| Error::Other(e.into()))?;
		let entries = session.read_dir(path.to_string_lossy()).await?;
		let paths: Vec<PathBuf> = entries.into_iter().map(|p| p.file_name().into()).collect();
		Ok(paths)
	}

	async fn read(&self, path: &Path) -> Result<Vec<u8>, Error> {
		println!("READING: {}", path.display());
		let session = self.pool().await?.get().await.map_err(|e| Error::Other(e.into()))?;
		let mut file = session.open(path.to_str().unwrap()).await?;
		let mut buf = Vec::new();
		file.read_to_end(&mut buf).await?;
		Ok(buf)
	}

	async fn write(&self, path: &Path, content: &[u8]) -> Result<(), Error> {
		let session = self.pool().await?.get().await.map_err(|e| Error::Other(e.into()))?;
		let mut file = session.create(path.to_str().unwrap()).await?;
		file.write_all(content).await?;
		Ok(())
	}

	async fn discover(&self, location: &Location, ctx: &ExecutionContext<'_>) -> Result<Vec<Arc<Resource>>, Error> {
		let mut files = Vec::new();
		let session = self.pool().await?.get().await.map_err(|e| Error::Other(e.into()))?;
		let backend = ctx.services.fs.get_provider(&location.host)?;
		self.discover_recursive(&session, ctx, location.path.clone(), 1, &location.options, &mut files, location, backend)
			.await?;
		Ok(files)
	}

	async fn mkdir(&self, path: &Path) -> Result<(), Error> {
		let session = self.pool().await?.get().await.map_err(|e| Error::Other(e.into()))?;
		session.create_dir(path.to_string_lossy()).await?;
		Ok(())
	}

	async fn r#move(&self, from: &Path, to: &Path) -> Result<(), Error> {
		let session = self.pool().await?.get().await.map_err(|e| Error::Other(e.into()))?;
		let rename_result = session.rename(from.to_string_lossy(), to.to_string_lossy()).await;
		if rename_result.is_err() {
			tracing::warn!(
				"Could not move {} to {}. Falling back to copy and delete. Original error: {:?}",
				from.display(),
				to.display(),
				rename_result.err()
			);
			self.copy(from, to).await?;
			self.delete(from).await?;
		}
		Ok(())
	}

	async fn copy(&self, from: &Path, to: &Path) -> Result<(), Error> {
		let content = self.read(from).await?;
		self.write(to, &content).await
	}

	async fn delete(&self, path: &Path) -> Result<(), Error> {
		let path = path.to_string_lossy().to_string();
		let session = self.pool().await?.get().await.map_err(|e| Error::Other(e.into()))?;
		let stat = session.metadata(&path).await?;
		if stat.is_dir() {
			session.remove_dir(&path).await?;
		} else {
			session.remove_file(&path).await?;
		}
		Ok(())
	}

	async fn download(&self, from: &Path) -> Result<PathBuf, Error> {
		tracing::info!("Downloading {}", from.display());
		let content = self.read(from).await?;
		let temp_file = NamedTempFile::new().map_err(|e| Error::Io(e))?;
		tokio::fs::write(temp_file.path(), &content).await.map_err(|e| Error::Io(e))?;
		tracing::info!("Downloaded {}", from.display());
		let path = temp_file.keep().map_err(|e| Error::Other(e.into()))?;
		Ok(path.1)
	}

	async fn download_many(&self, from: &[PathBuf]) -> Result<Vec<PathBuf>, Error> {
		let mut futures = Vec::with_capacity(from.len());
		for path in from {
			let sftp_clone = self.clone();
			let path_clone = path.clone();
			futures.push(tokio::spawn(async move { sftp_clone.download(&path_clone).await }));
		}
		let results: Result<Vec<PathBuf>, Error> = futures::future::try_join_all(futures)
			.await
			.map_err(|e| Error::Other(e.into()))?
			.into_iter()
			.collect();
		results
	}

	async fn upload(&self, from_local: &Path, to: &Path) -> Result<(), Error> {
		let content = tokio::fs::read(from_local).await.map_err(|e| Error::Io(e))?;
		self.write(to, &content).await
	}

	async fn upload_many(&self, from_local: &[PathBuf], to: &[PathBuf]) -> Result<(), Error> {
		if from_local.len() != to.len() {
			return Err(Error::Other(anyhow::anyhow!(
				"Mismatched number of source and destination paths for upload_many"
			)));
		}

		let mut futures = Vec::with_capacity(from_local.len());
		for (from, to) in from_local.iter().zip(to.iter()) {
			let sftp_clone = self.clone();
			let from_clone = from.clone();
			let to_clone = to.clone();
			futures.push(tokio::spawn(async move { sftp_clone.upload(&from_clone, &to_clone).await }));
		}

		futures::future::try_join_all(futures)
			.await
			.map_err(|e| Error::Other(e.into()))?
			.into_iter()
			.for_each(drop);
		Ok(())
	}

	async fn hardlink(&self, from: &Path, to: &Path) -> Result<(), Error> {
		let session = self.pool().await?.get().await.map_err(|e| Error::Other(e.into()))?;
		session.hardlink(from.to_string_lossy(), to.to_string_lossy()).await?;
		Ok(())
	}

	async fn symlink(&self, from: &Path, to: &Path) -> Result<(), Error> {
		let session = self.pool().await?.get().await.map_err(|e| Error::Other(e.into()))?;
		session.symlink(from.to_string_lossy(), to.to_string_lossy()).await?;
		Ok(())
	}
}

impl Sftp {
	fn discover_recursive<'a>(
		&'a self,
		sftp: &'a Object<Self>,
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

			let entries = sftp.read_dir(path.to_string_lossy()).await?;
			let parent_components: Vec<String> = path
				.components()
				.enumerate()
				.map(|(i, component)| {
					if i == 0 {
						"/".to_string()
					} else {
						component.as_os_str().to_string_lossy().to_string()
					}
				})
				.collect();

			for entry in entries {
				let mut components = parent_components.clone();
				components.push(entry.file_name());
				let entry = components.join("/").replace("//", "/");
				let pathbuf = PathBuf::from(&entry);

				if options.exclude.contains(&pathbuf) {
					continue;
				}

				if !options.hidden_files && entry.starts_with('.') {
					continue;
				}

				let metadata = sftp.metadata(entry).await?;

				if metadata.is_dir() {
					if depth >= options.min_depth {
						if let organize_sdk::location::options::Target::Folders = options.target {
							let resource = pathbuf
								.clone()
								.as_resource(ctx, Some(Arc::new(location.clone())), backend.clone())
								.await;
							files.push(resource);
						}
					}
					self.discover_recursive(sftp, ctx, pathbuf, depth + 1, options, files, location, backend.clone())
						.await?;
				} else {
					if depth >= options.min_depth {
						if options.target.is_files() {
							let resource = pathbuf
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
