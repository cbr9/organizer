use organize_sdk::plugins::storage::Metadata;

pub mod local;
#[cfg(windows)]
pub mod sftp;
pub mod vfs;

pub trait IntoMetadata {
	fn into_metadata(self) -> Metadata;
}
