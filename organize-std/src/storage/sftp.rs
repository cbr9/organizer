use std::{
	collections::HashMap,
	fmt::{Debug, Formatter},
	net::{IpAddr, SocketAddr},
	path::{Path, PathBuf},
	sync::Arc,
	time::{Duration, UNIX_EPOCH},
};
use tokio_stream::StreamExt;
use tokio_util::codec::{BytesCodec, FramedRead};

use bytes::Bytes;
use deadpool::managed::{self, Metrics, Object, Pool, RecycleResult};
use russh_sftp::{client::SftpSession, protocol::FileAttributes};
use tokio::{
	io::{AsyncReadExt, AsyncWriteExt},
	net::windows::named_pipe::NamedPipeClient,
};

use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::{future::BoxFuture, stream::BoxStream};
use russh::{
	client::{self, Handle},
	keys::{agent::client::AgentClient, Algorithm},
	Channel,
};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, OnceCell};

use organize_sdk::{
	context::{services::fs::resource::Resource, ExecutionContext},
	error::Error,
	location::{options::Options, Location},
	plugins::storage::{Metadata, StorageProvider},
};

use super::IntoMetadata;

#[derive(Serialize, Deserialize)]
pub struct Sftp {
	pub address: IpAddr,
	pub port: u16,
	pub username: String,
	pub private_key: Option<PathBuf>,
	#[serde(skip)]
	pub pool: OnceCell<SftpPool>,
	#[serde(skip)]
	agent: OnceCell<Mutex<AgentClient<NamedPipeClient>>>,
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
			agent: OnceCell::new(),
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
					.max_size(5) // Max 5 concurrent connections
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
		let agent_client = self
			.agent
			.get_or_try_init(|| async {
				let client_pipe = tokio::net::windows::named_pipe::ClientOptions::new()
					.open(r"\\.\pipe\openssh-ssh-agent")
					.context("Could not connect to the SSH agent pipe. Is 1Password or another agent running?")?;
				Ok::<Mutex<AgentClient<NamedPipeClient>>, Error>(Mutex::new(AgentClient::connect(client_pipe)))
			})
			.await?;

		let mut agent = agent_client.lock().await;

		let hash_alg = session.best_supported_rsa_hash().await?.flatten();

		let mut authenticated = false;
		let identities = agent.request_identities().await.unwrap();
		for identity in identities {
			let alg = match identity.algorithm() {
				Algorithm::Dsa | Algorithm::Rsa { .. } => hash_alg,
				_ => None,
			};

			let auth_result = session
				.authenticate_publickey_with(&self.username, identity, alg, &mut *agent)
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

impl IntoMetadata for FileAttributes {
	fn into_metadata(self) -> Metadata {
		let mut extra = HashMap::new();
		if let Some(uid) = self.uid {
			extra.insert("uid".to_string(), uid.to_string());
		}
		if let Some(gid) = self.gid {
			extra.insert("gid".to_string(), gid.to_string());
		}
		if let Some(permissions) = self.permissions {
			extra.insert("permissions".to_string(), format!("{:#o}", permissions));
		}

		Metadata {
			size: self.size,
			modified: self.mtime.map(|t| UNIX_EPOCH + Duration::from_secs(t as u64)),
			created: None, // The SFTP protocol doesn't provide creation time.
			is_dir: self.is_dir(),
			is_file: !self.is_dir(),
			extra,
		}
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

	async fn metadata(&self, path: &Path) -> Result<Metadata, Error> {
		let session = self.pool().await?.get().await.map_err(|e| Error::Other(e.into()))?;
		let sftp_attrs = session.metadata(path.to_string_lossy()).await?;
		Ok(sftp_attrs.into_metadata())
	}

	async fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, Error> {
		let session = self.pool().await?.get().await.map_err(|e| Error::Other(e.into()))?;
		let entries = session.read_dir(path.to_string_lossy()).await?;
		let paths: Vec<PathBuf> = entries.into_iter().map(|p| p.file_name().into()).collect();
		Ok(paths)
	}

	async fn read(&self, path: &Path) -> Result<Vec<u8>, Error> {
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

	async fn discover(&self, location: &Location, ctx: &ExecutionContext) -> Result<Vec<Arc<Resource>>, Error> {
		let mut files = Vec::new();
		let session = self.pool().await?.get().await.map_err(|e| Error::Other(e.into()))?;
		let backend = ctx.services.fs.get_provider(&location.host)?;
		self.discover_recursive(&session, ctx, location.path.clone(), 1, &location.options, &mut files, location, backend)
			.await?;
		Ok(files)
	}

	async fn mk_parent(&self, path: &Path) -> Result<(), Error> {
		let session = self.pool().await?.get().await.map_err(|e| Error::Other(e.into()))?;
		session.create_dir(path.to_string_lossy()).await?;
		Ok(())
	}

	async fn rename(&self, from: &Path, to: &Path) -> Result<(), Error> {
		let session = self.pool().await?.get().await.map_err(|e| Error::Other(e.into()))?;
		session.rename(from.to_string_lossy(), to.to_string_lossy()).await?;
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

	fn download<'a>(&'a self, path: &'a Path) -> BoxStream<'a, Result<Bytes, Error>> {
		let stream = async_stream::try_stream! {
			let session = self.pool().await?.get().await.map_err(|e| Error::Other(e.into()))?;
			let remote_file = session.open(path.to_string_lossy()).await?;
			let mut reader = FramedRead::new(remote_file, BytesCodec::new());
			while let Some(chunk_result) = reader.next().await {
				yield chunk_result?.freeze();
			}
		};

		Box::pin(stream)
	}

	fn upload<'a>(&'a self, to: &'a Path, mut stream: BoxStream<'a, Result<Bytes, Error>>) -> BoxFuture<'a, Result<(), Error>> {
		Box::pin(async move {
			let session = self.pool().await?.get().await.map_err(|e| Error::Other(e.into()))?;
			let mut remote_file = session.create(to.to_string_lossy()).await?;

			while let Some(chunk) = stream.try_next().await? {
				remote_file.write_all(&chunk).await?;
			}

			Ok(())
		})
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
		ctx: &'a ExecutionContext,
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
							let resource = ctx
								.services
								.fs
								.get_or_init_resource(pathbuf.clone(), Some(Arc::new(location.clone())), &location.host, backend.clone())
								.await;
							files.push(resource);
						}
					}
					self.discover_recursive(sftp, ctx, pathbuf, depth + 1, options, files, location, backend.clone())
						.await?;
				} else {
					if depth >= options.min_depth {
						if options.target.is_files() {
							let resource = ctx
								.services
								.fs
								.get_or_init_resource(pathbuf.clone(), Some(Arc::new(location.clone())), &location.host, backend.clone())
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
