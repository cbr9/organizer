use crate::cmd::info::Info;
use crate::cmd::new::New;
use crate::cmd::{edit::Edit, logs::Logs, run::Run, stop::Stop, watch::Watch};
use clap::Clap;

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
		match self {
			App::Watch(mut watch) => {
				watch.config = watch.config.canonicalize()?;
				watch.run()
			}
			App::Run(mut run) => {
				run.config = run.config.canonicalize()?;
				run.run()
			}
			App::Stop(mut stop) => {
				stop.config = stop.config.canonicalize()?;
				stop.run()
			}
			App::Logs(logs) => logs.run(),
			App::Edit(config) => config.run(),
			App::New(new) => new.run(),
			App::Info(info) => info.run(),
		}
	}
}
