use async_trait::async_trait;
use dyn_clone::DynClone;
use dyn_eq::DynEq;
use std::{fmt::Debug, sync::Arc};

use crate::context::services::fs::resource::Resource;

dyn_clone::clone_trait_object!(Sorter);
dyn_eq::eq_trait_object!(Sorter);

#[async_trait]
#[typetag::serde(tag = "type")]
pub trait Sorter: DynEq + DynClone + Sync + Send + Debug {
	/// Sorts a slice of resources in-place.
	async fn sort(&self, files: &mut [Arc<Resource>]);
}
