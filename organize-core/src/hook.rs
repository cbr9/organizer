use crate::{config::Config, context::RunSettings, resource::Resource, rule::Rule};
use anyhow::Result;

/// A trait for plugins to listen and react to events in the engine's lifecycle.
///
/// Implementors only need to define the methods for the events they care about,
/// as each method has a default empty implementation.
pub trait Hook {
	/// Called once at the very beginning of a run, before any rules are processed.
	fn on_run_start(&self, _ctx: &RunContext) -> Result<()> {
		Ok(())
	}

	/// Called once at the very end of a run, with final statistics.
	fn on_run_end(&self, _stats: &RunStatistics) -> Result<()> {
		Ok(())
	}

	/// Called when a new rule begins processing.
	fn on_rule_start(&self, _rule: &Rule) -> Result<()> {
		Ok(())
	}

	/// Called when a rule has finished processing all of its folders.
	fn on_rule_end(&self, _rule: &Rule) -> Result<()> {
		Ok(())
	}

	/// Called on a resource *after* it has been evaluated by the filter chain.
	/// The `passed` parameter indicates whether the resource will be acted upon.
	/// This hook is purely for observation and cannot change the filter outcome.
	fn after_filters(&self, _resource: &Resource, _passed: bool) -> Result<()> {
		Ok(())
	}

	/// Called on a resource that has passed filters, just before the action chain begins.
	fn before_actions(&self, _resource: &Resource) -> Result<()> {
		Ok(())
	}

	/// Called on a resource after all actions have been executed on it.
	/// The `Resource` passed to this hook will reflect any changes (e.g., a new path).
	fn after_actions(&self, _resource: &Resource) -> Result<()> {
		Ok(())
	}
}

/// Contains read-only information about the entire run.
pub struct RunContext<'a> {
	pub settings: &'a RunSettings,
	pub config: &'a Config,
}

/// Contains final statistics compiled after a run is complete.
pub struct RunStatistics {
	pub files_processed: u64,
	pub actions_taken: u64,
	pub errors: u64,
}
