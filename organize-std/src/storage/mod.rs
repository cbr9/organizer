use organize_sdk::plugins::storage::Metadata;

pub mod local;
pub mod sftp;
pub mod vfs;

pub trait IntoMetadata {
	fn into_metadata(self) -> Metadata;
}
