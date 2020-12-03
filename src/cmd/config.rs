use crate::cmd::Cmd;
use log::error;
use anyhow::{Result, Context};
use clap::{crate_name, Clap};
use colored::Colorize;
use organize_core::{
	data::{
		config::UserConfig,
		options::{apply::wrapper::ApplyWrapper, Options},
	},
	utils::DefaultOpt,
};
use std::{env, process};
use std::thread::spawn;
use std::process::{Command, ExitStatus};

#[derive(Clap, Debug)]
pub struct Config {
	#[clap(long, exclusive = true)]
	show_path: bool,
	#[clap(long, exclusive = true)]
	show_defaults: bool,
	#[clap(long, exclusive = true)]
	new: bool,
}

impl Cmd for Config {
	fn run(self) -> Result<()> {
		if self.show_path {
			println!("{}", UserConfig::default_path().display());
		} else if self.show_defaults {
			if let Options {
				recursive: Some(recursive),
				watch: Some(watch),
				ignore: Some(ignore),
				hidden_files: Some(hidden_files),
				r#match: Some(r#match),
				apply,
			} = Options::default_some()
			{
				if let ApplyWrapper {
					actions: Some(actions),
					filters: Some(filters),
				} = apply
				{
					println!("recursive: {}", recursive.to_string().bright_purple());
					println!("watch: {}", watch.to_string().bright_purple());
					println!("hidden_files: {}", hidden_files.to_string().bright_purple());
					println!("ignored_directories: {:?}", ignore);
					println!("match: {:?}", r#match);
					println!("apply (actions): {}", actions.to_string().bright_purple());
					println!("apply (filters): {}", filters.to_string().bright_purple());
				}
			}
		} else if self.new {
			let config_file = env::current_dir()?.join(format!("{}.yml", crate_name!()));
			UserConfig::create(&config_file);
		} else {
			env::var("EDITOR").map(|editor| {
				let path = UserConfig::path();
				let mut command = Command::new(&editor);
				command.arg(path).spawn().context(format!("{}", &editor))?.wait().context("command wasn't running")
			}).context("invalid EDITOR variable")??;
		}
		Ok(())
	}
}
