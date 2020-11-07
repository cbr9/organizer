use crate::cmd::{config::Config, logs::Logs, run::Run, stop::Stop, watch::Watch};
use clap::Clap;

mod config;
pub(super) mod logs;
mod run;
mod stop;
mod watch;

#[derive(Clap)]
#[clap(about, author, version)]
pub enum App {
	Watch(Watch),
	Run(Run),
	Logs(Logs),
	Stop(Stop),
	Config(Config),
}

pub trait Cmd {
	fn run(self) -> anyhow::Result<()>;
}

impl Cmd for App {
	fn run(self) -> anyhow::Result<()> {
		match self {
			App::Watch(watch) => watch.run(),
			App::Run(run) => run.run(),
			App::Logs(logs) => logs.run(),
			App::Stop(stop) => stop.run(),
			App::Config(config) => config.run(),
		}
	}
}
