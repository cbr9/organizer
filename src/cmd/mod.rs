use crate::cmd::{edit::Edit, logs::Logs, run::Run, stop::Stop, watch::Watch};
use clap::Clap;
use crate::cmd::new::New;
use crate::cmd::info::Info;

mod edit;
pub(super) mod logs;
mod run;
mod stop;
mod watch;
mod new;
mod info;

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
