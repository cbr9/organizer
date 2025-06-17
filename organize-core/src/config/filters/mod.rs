use dyn_clone::DynClone;
use dyn_eq::DynEq;
use itertools::Itertools;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub mod empty;
pub mod extension;
pub mod filename;
pub mod mime;
pub mod regex;

use crate::{config::context::ExecutionContext, resource::Resource, templates::template::Template};

dyn_clone::clone_trait_object!(Filter);
dyn_eq::eq_trait_object!(Filter);

#[typetag::serde(tag = "type")]
pub trait Filter: DynClone + DynEq + Debug + Send + Sync {
	fn filter(&self, res: &Resource, ctx: &ExecutionContext) -> bool;
	fn templates(&self) -> Vec<&Template>;
}

#[derive(Eq, PartialEq, Deserialize, Serialize, Debug, Clone)]
struct Not {
	filter: Box<dyn Filter>,
}

#[typetag::serde(name = "not")]
impl Filter for Not {
	#[tracing::instrument(ret, level = "debug", skip(ctx))]
	fn filter(&self, res: &Resource, ctx: &ExecutionContext) -> bool {
		!self.filter.filter(res, ctx)
	}

	fn templates(&self) -> Vec<&Template> {
		self.filter.templates()
	}
}

#[derive(Eq, PartialEq, Deserialize, Serialize, Debug, Clone)]
struct AnyOf {
	filters: Vec<Box<dyn Filter>>,
}

#[typetag::serde(name = "any_of")]
impl Filter for AnyOf {
	#[tracing::instrument(ret, level = "debug", skip(ctx))]
	fn filter(&self, res: &Resource, ctx: &ExecutionContext) -> bool {
		self.filters.par_iter().any(|f| f.filter(res, ctx))
	}

	fn templates(&self) -> Vec<&Template> {
		self.filters.iter().flat_map(|f| f.templates()).collect_vec()
	}
}

#[derive(Eq, PartialEq, Deserialize, Serialize, Debug, Clone)]
struct AllOf {
	filters: Vec<Box<dyn Filter>>,
}

#[typetag::serde(name = "all_of")]
impl Filter for AllOf {
	#[tracing::instrument(ret, level = "debug", skip(ctx))]
	fn filter(&self, res: &Resource, ctx: &ExecutionContext) -> bool {
		self.filters.par_iter().all(|f| f.filter(res, ctx))
	}

	fn templates(&self) -> Vec<&Template> {
		self.filters.iter().flat_map(|f| f.templates()).collect_vec()
	}
}

#[derive(Eq, PartialEq, Deserialize, Serialize, Debug, Clone)]
struct NoneOf {
	filters: Vec<Box<dyn Filter>>,
}

#[typetag::serde(name = "none_of")]
impl Filter for NoneOf {
	#[tracing::instrument(ret, level = "debug", skip(ctx))]
	fn filter(&self, res: &Resource, ctx: &ExecutionContext) -> bool {
		!self.filters.par_iter().any(|f| f.filter(res, ctx))
	}

	fn templates(&self) -> Vec<&Template> {
		self.filters.iter().flat_map(|f| f.templates()).collect_vec()
	}
}
