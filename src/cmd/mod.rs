use clap::Clap;

use libalfred::logger::Logger;

use crate::cmd::{edit::Edit, info::Info, logs::Logs, new::New, run::Run, stop::Stop, watch::Watch};

mod edit;
mod info;
pub(super) mod logs;
mod new;
mod run;
mod stop;
mod watch;
mod undo;

#[derive(Clap)]
#[clap(about, author, version)]
pub enum App {
	Watch(Watch),
	Run(Run),
	Logs(Logs),
	Stop(Stop),
	Edit(Edit),
	New(New),
	Info(Info),
}

pub trait Cmd {
	fn run(self) -> anyhow::Result<()>;
}

impl Cmd for App {
	fn run(self) -> anyhow::Result<()> {
		use App::*;
		match self {
			Watch(watch) => watch.run(),
			Run(run) => run.run(),
			Stop(stop) => stop.run(),
			Logs(logs) => {
				Logger::setup(logs.no_color)?;
				logs.run()
			}
			Edit(config) => config.run(),
			New(new) => new.run(),
			Info(info) => info.run(),
		}
	}
}
