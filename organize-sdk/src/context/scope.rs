use crate::{
    engine::{batch::Batch, rule::RuleMetadata},
    location::Location,
    resource::Resource,
};
use anyhow::Result;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Debug, Clone)]
pub enum ExecutionScope<'a> {
	Rule(RuleScope),
	Search(SearchScope<'a>),
	Resource(ResourceScope),
	Batch(BatchScope<'a>),
	Build(BuildScope),
	Blank,
}

impl<'a> ExecutionScope<'a> {
	pub fn new_rule_scope(rule: Arc<RuleMetadata>) -> ExecutionScope<'a> {
		ExecutionScope::Rule(RuleScope { rule })
	}

	pub fn new_location_scope(rule: Arc<RuleMetadata>, location: &'a Location) -> ExecutionScope<'a> {
		ExecutionScope::Search(SearchScope { rule, location })
	}

	pub fn new_resource_scope(rule: Arc<RuleMetadata>, resource: Arc<Resource>) -> ExecutionScope<'a> {
		ExecutionScope::Resource(ResourceScope { rule, resource })
	}

	pub fn new_batch_scope(rule: Arc<RuleMetadata>, batch: &'a Batch) -> ExecutionScope<'a> {
		ExecutionScope::Batch(BatchScope { rule, batch })
	}

	pub fn new_build_scope(root: &Path) -> ExecutionScope<'a> {
		ExecutionScope::Build(BuildScope { root: root.to_path_buf() })
	}

	pub fn rule(&self) -> Result<Arc<RuleMetadata>> {
		match self {
			ExecutionScope::Rule(scope) => Ok(scope.rule.clone()),
			ExecutionScope::Resource(scope) => Ok(scope.rule.clone()),
			ExecutionScope::Batch(scope) => Ok(scope.rule.clone()),
			ExecutionScope::Search(scope) => Ok(scope.rule.clone()),
			_ => anyhow::bail!("Rule not in scope"),
		}
	}

	pub fn resource(&self) -> Result<Arc<Resource>> {
		match self {
			ExecutionScope::Resource(scope) => Ok(scope.resource.clone()),
			_ => anyhow::bail!("Resource not in scope"),
		}
	}

	pub fn batch(&self) -> Result<&'a Batch> {
		match self {
			ExecutionScope::Batch(scope) => Ok(scope.batch),
			_ => anyhow::bail!("Batch not in scope"),
		}
	}

	pub fn root(&self) -> Result<PathBuf> {
		match self {
			ExecutionScope::Search(scope) => Ok(scope.location.path.clone()),
			ExecutionScope::Resource(scope) => Ok(scope.resource.location.path.clone()),
			ExecutionScope::Build(path) => Ok(path.root.clone()), // <-- ADD THIS CASE
			_ => anyhow::bail!("Root not in scope"),
		}
	}
}

#[derive(Debug, Clone)]
pub struct RuleScope {
	pub rule: Arc<RuleMetadata>,
}
#[derive(Debug, Clone)]
pub struct SearchScope<'a> {
	pub rule: Arc<RuleMetadata>,
	pub location: &'a Location,
}
#[derive(Debug, Clone)]
pub struct ResourceScope {
	pub rule: Arc<RuleMetadata>,
	pub resource: Arc<Resource>,
}
#[derive(Debug, Clone)]
pub struct BatchScope<'a> {
	pub rule: Arc<RuleMetadata>,
	pub batch: &'a Batch,
}

#[derive(Debug, Clone)]
pub struct BuildScope {
	pub root: PathBuf,
}
