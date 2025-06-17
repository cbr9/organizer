use crate::{
	config::{
		context::{ExecutionContext, ExecutionScope, RunServices, RunSettings},
		Config,
		ConfigBuilder,
	},
	templates::TemplateEngine,
};
use anyhow::Result;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::path::PathBuf;

/// The main engine for the application.
/// It owns the compiled configuration and all run-wide services.
pub struct Engine {
	pub config: Config,
	pub services: RunServices,
}

impl Engine {
	pub fn new(path: Option<PathBuf>, tags: Option<Vec<String>>, ids: Option<Vec<String>>) -> Result<Self> {
		let config_builder = ConfigBuilder::new(path)?;
		let mut engine = TemplateEngine::from_config(&config_builder)?;
		let config = config_builder.build(&mut engine, tags, ids)?;

		let services = RunServices {
			template_engine: engine,
			credential_cache: Default::default(),
			content_cache: Default::default(),
		};
		Ok(Self { config, services })
	}

	/// Runs the organization process based on the loaded configuration and
	/// command-line arguments.
	pub fn run(&self, dry_run: bool) -> Result<()> {
		for (i, rule) in self.config.rules.iter().enumerate() {
			for folder in rule.folders.iter() {
				let context = ExecutionContext {
					services: &self.services,
					scope: ExecutionScope {
						config: &self.config,
						rule,
						folder,
					},
					settings: RunSettings { dry_run },
				};

				let entries = match folder.get_resources() {
					Ok(entries) => entries
						.into_par_iter()
						.filter(|res| rule.filters.iter().all(|f| f.filter(res, &context)))
						.collect::<Vec<_>>(),
					Err(e) => {
						tracing::error!(
							"Rule [number = {}, id = {}]: Could not read entries from folder '{}'. Error: {}",
							i,
							rule.id.as_deref().unwrap_or("untitled"),
							folder.path.display(),
							e
						);
						continue;
					}
				};

				rule.actions
					.iter()
					.fold(entries, |current_entries, action| action.run(current_entries, &context));
			}
		}
		Ok(())
	}
}
