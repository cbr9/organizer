use clap::Clap;

use organize_core::logger::Logger;

use crate::cmd::{edit::Edit, logs::Logs, run::Run, stop::Stop, watch::Watch};
use crate::cmd::info::Info;
use crate::cmd::new::New;

mod edit;
mod info;
pub(super) mod logs;
mod new;
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
			Watch(mut watch) => {
				Logger::setup(watch.no_color)?;
				watch.config = watch.config.canonicalize()?;
				watch.run()
			}
			Run(mut run) => {
				Logger::setup(run.no_color)?;
				run.config = run.config.canonicalize()?;
				run.run()
			}
			Stop(mut stop) => {
				Logger::setup(stop.no_color)?;
				stop.config = stop.config.canonicalize()?;
				stop.run()
			}
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
