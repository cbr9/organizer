use async_trait::async_trait;
use dyn_clone::DynClone;
use dyn_eq::DynEq;
use std::{fmt::Debug, sync::Arc};

// pub mod content;
// pub mod empty;
// pub mod extension;
// pub mod filename;
// pub mod mime;
// pub mod regex;

use crate::{
	context::ExecutionContext,
	engine::ExecutionModel,
	error::Error,
	resource::Resource,
};

dyn_clone::clone_trait_object!(Filter);
dyn_eq::eq_trait_object!(Filter);

#[typetag::serde(tag = "type")]
#[async_trait]
pub trait Filter: DynClone + DynEq + Debug + Send + Sync {
	fn execution_model(&self) -> ExecutionModel {
		ExecutionModel::Single
	}

	/// Takes the execution context, which contains the appropriate scope,
	/// and returns a Result containing the list of files that passed.
	async fn filter(&self, ctx: &ExecutionContext) -> Result<Vec<Arc<Resource>>, Error>;
}

// #[derive(Eq, PartialEq, Deserialize, Serialize, Debug, Clone)]
// struct Not(Box<dyn Filter>);

// impl std::ops::Deref for Not {
// 	type Target = Box<dyn Filter>;

// 	fn deref(&self) -> &Self::Target {
// 		&self.0
// 	}
// }

// #[async_trait]
// #[typetag::serde(name = "not")]
// impl Filter for Not {
// 	async fn filter(&self, ctx: &ExecutionContext) -> bool {
// 		!self.filter(ctx).await
// 	}
// }

// #[derive(Eq, PartialEq, Deserialize, Serialize, Debug, Clone)]
// struct AnyOf(Vec<Box<dyn Filter>>);

// impl std::ops::Deref for AnyOf {
// 	type Target = Vec<Box<dyn Filter>>;

// 	fn deref(&self) -> &Self::Target {
// 		&self.0
// 	}
// }

// #[async_trait]
// #[typetag::serde(name = "any_of")]
// impl Filter for AnyOf {
// 	async fn filter(&self, ctx: &ExecutionContext) -> bool {
// 		let filter_futures = self.iter().map(|f| f.filter(ctx));
// 		let results: Vec<bool> = future::join_all(filter_futures).await;
// 		results.iter().any(|&result| result)
// 	}
// }

// #[derive(Eq, PartialEq, Deserialize, Serialize, Debug, Clone)]
// struct AllOf(Vec<Box<dyn Filter>>);

// impl std::ops::Deref for AllOf {
// 	type Target = Vec<Box<dyn Filter>>;

// 	fn deref(&self) -> &Self::Target {
// 		&self.0
// 	}
// }

// #[async_trait]
// #[typetag::serde(name = "all_of")]
// impl Filter for AllOf {
// 	async fn filter(&self, ctx: &ExecutionContext) -> bool {
// 		let filter_futures = self.iter().map(|f| f.filter(ctx));
// 		let results: Vec<bool> = future::join_all(filter_futures).await;
// 		results.iter().all(|&result| result)
// 	}
// }

// #[derive(Eq, PartialEq, Deserialize, Serialize, Debug, Clone)]
// struct NoneOf(Vec<Box<dyn Filter>>);

// impl std::ops::Deref for NoneOf {
// 	type Target = Vec<Box<dyn Filter>>;

// 	fn deref(&self) -> &Self::Target {
// 		&self.0
// 	}
// }

// #[async_trait]
// #[typetag::serde(name = "none_of")]
// impl Filter for NoneOf {
// 	async fn filter(&self, ctx: &ExecutionContext) -> bool {
// 		let filter_futures = self.iter().map(|f| f.filter(ctx));
// 		let results: Vec<bool> = future::join_all(filter_futures).await;
// 		!results.iter().any(|&result| result)
// 	}
// }