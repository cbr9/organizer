use async_trait::async_trait;
use dyn_clone::DynClone;
use dyn_eq::DynEq;
use std::fmt::Debug;

use crate::batch::Batch;

dyn_clone::clone_trait_object!(Grouper);
dyn_eq::eq_trait_object!(Grouper);

#[async_trait]
#[typetag::serde(tag = "type")]
pub trait Grouper: DynEq + DynClone + Sync + Send + Debug {
	fn name(&self) -> &str;
	async fn group(&self, batch: &Batch) -> Vec<Batch>;
}
