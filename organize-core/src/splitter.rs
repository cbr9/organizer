use crate::{batch::Batch, errors::Error};
use async_trait::async_trait;
use dyn_clone::DynClone;
use dyn_eq::DynEq;
use std::{collections::HashMap, fmt::Debug};

dyn_clone::clone_trait_object!(Splitter);
dyn_eq::eq_trait_object!(Splitter);

#[async_trait]
#[typetag::serde(tag = "type")]
pub trait Splitter: DynEq + DynClone + Sync + Send + Debug {
	async fn split(&self, batch: &Batch) -> Result<HashMap<String, Batch>, Error>;
}
