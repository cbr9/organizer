use crate::{engine::batch::Batch, error::Error};
use async_trait::async_trait;
use dyn_clone::DynClone;
use dyn_eq::DynEq;
use std::{collections::HashMap, fmt::Debug};

dyn_clone::clone_trait_object!(Partitioner);
dyn_eq::eq_trait_object!(Partitioner);

#[async_trait]
#[typetag::serde(tag = "type")]
pub trait Partitioner: DynEq + DynClone + Sync + Send + Debug {
	fn name(&self) -> &str;
	async fn partition(&self, batch: &Batch) -> Result<HashMap<String, Batch>, Error>;
}