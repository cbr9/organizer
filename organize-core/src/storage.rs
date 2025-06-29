use crate::{
	context::ExecutionContext,
	errors::Error,
	options::{Options, OptionsBuilder},
	resource::Resource,
	templates::prelude::Template,
};
use anyhow::Result;
use async_trait::async_trait;
use dyn_clone::DynClone;
use dyn_eq::DynEq;
use futures::{stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::{
	fmt::Debug,
	path::{Path, PathBuf},
	sync::Arc,
};

dyn_clone::clone_trait_object!(Location);
dyn_eq::eq_trait_object!(Location);

#[async_trait]
#[typetag::serde(tag = "type")]
/// A trait for any component that can provide a list of files to be processed.
/// This could be a local folder, an S3 bucket, an SFTP connection, etc.
pub trait Location: DynEq + DynClone + Sync + Send + Debug {
	fn partial_options(&self) -> &OptionsBuilder;
	fn initialize_options(&self, options: Options);
	fn options(&self) -> &Options;
	fn partial_path(&self) -> &Template;
	fn initialize_path(&self, path: PathBuf);
	fn path(&self) -> &PathBuf;
	async fn get_resources(&self, ctx: &ExecutionContext<'_>) -> Result<Vec<Arc<Resource>>, Error>;

	async fn get_excluded_paths(&self, ctx: &ExecutionContext<'_>) -> Vec<PathBuf> {
		let mut excluded_paths = Vec::new();

		// Use a simple, sequential loop. This is clear and avoids all lifetime issues.
		for template in &self.options().exclude {
			// We await each future one by one.
			if let Ok(rendered_path_str) = template.render(ctx).await {
				excluded_paths.push(PathBuf::from(rendered_path_str));
			}
		}

		excluded_paths
	}
}
