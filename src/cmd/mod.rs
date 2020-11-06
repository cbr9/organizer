use crate::cmd::{config::Config, logs::Logs, run::Run, stop::Stop, watch::Watch};
use anyhow::Result;
use clap::Clap;

pub mod config;
pub mod logs;
pub mod run;
pub mod stop;
pub mod watch;

#[derive(Clap)]
pub enum App {
	Watch(Watch),
	Run(Run),
	Logs(Logs),
	Stop(Stop),
	Config(Config),
}

pub trait Cmd {
	fn run(self) -> Result<()>;
}

impl Cmd for App {
	fn run(self) -> Result<()> {
		match self {
			App::Watch(watch) => watch.run(),
			App::Run(run) => run.run(),
			App::Logs(logs) => logs.run(),
			App::Stop(stop) => stop.run(),
			App::Config(config) => config.run(),
		}
	}
}
