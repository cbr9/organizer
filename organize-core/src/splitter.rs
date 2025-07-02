use crate::{batch::Batch, errors::Error};
use async_trait::async_trait;
use dyn_clone::DynClone;
use dyn_eq::DynEq;
use std::fmt::Debug;

dyn_clone::clone_trait_object!(Splitter);
dyn_eq::eq_trait_object!(Splitter);

#[async_trait]
#[typetag::serde(tag = "type")]
pub trait Splitter: DynEq + DynClone + Sync + Send + Debug {
	/// Splits a single batch into multiple named batches.
	/// The implementation should set the `name` field on each output `Batch`.
	async fn split(&self, batch: &Batch) -> Result<Vec<Batch>, Error>;
}