use crate::cmd::Cmd;
use clap::Clap;
use colored::Colorize;
use organize_core::{
	data::{config::Config, options::Options, Data},
	utils::DefaultOpt,
};

#[derive(Clap, Debug)]
pub struct Info {
	#[clap(long, short = 'd')]
	defaults: bool,
	#[clap(long, short = 'p')]
	path: bool,
	#[clap(long, short = 'a', exclusive = true)]
	all: bool,
}

impl Cmd for Info {
	fn run(mut self) -> anyhow::Result<()> {
		if !self.defaults && !self.path {
			self.all = true;
		}
		if self.all {
			self.defaults = true;
			self.path = true;
		}

		if self.defaults {
			let Options {
				recursive,
				watch,
				ignored_dirs,
				hidden_files,
				r#match,
				apply,
			} = Options::default_some();
			println!("{}:", "Defaults".bold().underline());
			println!("  recursive = {}", recursive.enabled.unwrap().to_string().bright_purple());
			println!("  recursive.depth = {}", recursive.depth.unwrap().to_string().bright_purple());
			println!("  watch = {}", watch.unwrap().to_string().bright_purple());
			println!("  ignored_dirs = {:?}", ignored_dirs.unwrap());
			println!("  hidden_files = {}", hidden_files.unwrap().to_string().bright_purple());
			println!("  match = {}", r#match.unwrap().to_string().bright_green());
			println!("  apply.actions = {}", apply.actions.unwrap().to_string().bright_green());
			println!("  apply.filters = {}", apply.filters.unwrap().to_string().bright_green());
			println!()
		}
		if self.path {
			println!("{}: {}", "Data directory".bold().underline(), Data::dir()?.display());
			println!("{}: {}", "Config directory".bold().underline(), Config::default_dir()?.display());
			println!()
		}
		Ok(())
	}
}
