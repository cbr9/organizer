use async_trait::async_trait;
use dyn_clone::DynClone;
use dyn_eq::DynEq;
use futures::future;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub mod content;
pub mod empty;
pub mod extension;
pub mod filename;
pub mod mime;
pub mod regex;

use crate::{config::context::ExecutionContext, templates::template::Template};

dyn_clone::clone_trait_object!(Filter);
dyn_eq::eq_trait_object!(Filter);

#[async_trait]
#[typetag::serde(tag = "type")]
pub trait Filter: DynClone + DynEq + Debug + Send + Sync {
	async fn filter(&self, ctx: &ExecutionContext) -> bool;
	fn templates(&self) -> Vec<&Template>;
}

#[derive(Eq, PartialEq, Deserialize, Serialize, Debug, Clone)]
struct Not {
	filter: Box<dyn Filter>,
}

#[async_trait]
#[typetag::serde(name = "not")]
impl Filter for Not {
	async fn filter(&self, ctx: &ExecutionContext) -> bool {
		!self.filter.filter(ctx).await
	}

	fn templates(&self) -> Vec<&Template> {
		self.filter.templates()
	}
}

#[derive(Eq, PartialEq, Deserialize, Serialize, Debug, Clone)]
struct AnyOf {
	filters: Vec<Box<dyn Filter>>,
}

#[async_trait]
#[typetag::serde(name = "any_of")]
impl Filter for AnyOf {
	async fn filter(&self, ctx: &ExecutionContext) -> bool {
		let filter_futures = self.filters.iter().map(|f| f.filter(ctx));
		let results: Vec<bool> = future::join_all(filter_futures).await;
		results.iter().any(|&result| result)
	}

	fn templates(&self) -> Vec<&Template> {
		self.filters.iter().flat_map(|f| f.templates()).collect_vec()
	}
}

#[derive(Eq, PartialEq, Deserialize, Serialize, Debug, Clone)]
struct AllOf {
	filters: Vec<Box<dyn Filter>>,
}

#[async_trait]
#[typetag::serde(name = "all_of")]
impl Filter for AllOf {
	async fn filter(&self, ctx: &ExecutionContext) -> bool {
		let filter_futures = self.filters.iter().map(|f| f.filter(ctx));
		let results: Vec<bool> = future::join_all(filter_futures).await;
		results.iter().all(|&result| result)
	}

	fn templates(&self) -> Vec<&Template> {
		self.filters.iter().flat_map(|f| f.templates()).collect_vec()
	}
}

#[derive(Eq, PartialEq, Deserialize, Serialize, Debug, Clone)]
struct NoneOf {
	filters: Vec<Box<dyn Filter>>,
}

#[async_trait]
#[typetag::serde(name = "none_of")]
impl Filter for NoneOf {
	async fn filter(&self, ctx: &ExecutionContext) -> bool {
		let filter_futures = self.filters.iter().map(|f| f.filter(ctx));
		let results: Vec<bool> = future::join_all(filter_futures).await;
		!results.iter().any(|&result| result)
	}

	fn templates(&self) -> Vec<&Template> {
		self.filters.iter().flat_map(|f| f.templates()).collect_vec()
	}
}
