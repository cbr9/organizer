use crate::{engine::batch::Batch, error::Error};
use anyhow::Result;
use async_trait::async_trait;
use dyn_clone::DynClone;
use dyn_eq::DynEq;
use std::fmt::Debug;

dyn_clone::clone_trait_object!(Selector);
dyn_eq::eq_trait_object!(Selector);

/// A trait for any component that selects a subset of files from a batch based on
/// positional or quantitative criteria (e.g., first, last, random sample).
#[async_trait]
#[typetag::serde(tag = "type")]
pub trait Selector: DynEq + DynClone + Sync + Send + Debug {
	async fn select(&self, batch: &Batch) -> Result<Batch, Error>;
}