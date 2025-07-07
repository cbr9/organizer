use crate::{
	context::services::fs::resource::Resource,
	engine::{batch::Batch, rule::RuleMetadata},
	location::Location,
};
use anyhow::Result;
use std::{
	path::{Path, PathBuf},
	sync::Arc,
};

#[derive(Debug, Clone)]
pub enum ExecutionScope {
	Rule(RuleScope),
	Search(SearchScope),
	Resource(ResourceScope),
	Batch(BatchScope),
	Build(BuildScope),
	Blank,
}

impl ExecutionScope {
	pub fn new_rule_scope(rule: Arc<RuleMetadata>) -> ExecutionScope {
		ExecutionScope::Rule(RuleScope { rule })
	}

	pub fn new_location_scope(rule: Arc<RuleMetadata>, location: Arc<Location>) -> ExecutionScope {
		ExecutionScope::Search(SearchScope { rule, location })
	}

	pub fn new_resource_scope(rule: Arc<RuleMetadata>, resource: Arc<Resource>) -> ExecutionScope {
		ExecutionScope::Resource(ResourceScope { rule, resource })
	}

	pub fn new_batch_scope(rule: Arc<RuleMetadata>, batch: Arc<Batch>) -> ExecutionScope {
		ExecutionScope::Batch(BatchScope { rule, batch })
	}

	pub fn new_build_scope(root: &Path) -> ExecutionScope {
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

	pub fn batch(&self) -> Result<&Batch> {
		match self {
			ExecutionScope::Batch(scope) => Ok(&scope.batch),
			_ => anyhow::bail!("Batch not in scope"),
		}
	}
}

#[derive(Debug, Clone)]
pub struct RuleScope {
	pub rule: Arc<RuleMetadata>,
}
#[derive(Debug, Clone)]
pub struct SearchScope {
	pub rule: Arc<RuleMetadata>,
	pub location: Arc<Location>,
}
#[derive(Debug, Clone)]
pub struct ResourceScope {
	pub rule: Arc<RuleMetadata>,
	pub resource: Arc<Resource>,
}
#[derive(Debug, Clone)]
pub struct BatchScope {
	pub rule: Arc<RuleMetadata>,
	pub batch: Arc<Batch>,
}

#[derive(Debug, Clone)]
pub struct BuildScope {
	pub root: PathBuf,
}
