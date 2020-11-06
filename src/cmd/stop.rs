use crate::{
	cmd::Cmd,
	lock_file::{GetProcessBy, LockFile},
	DEFAULT_CONFIG_STR,
};
use anyhow::Result;
use clap::{crate_name, Clap};
use std::path::PathBuf;
use sysinfo::{ProcessExt, RefreshKind, Signal, System, SystemExt};

#[derive(Clap, Debug)]
pub struct Stop {
	#[clap(long)]
	all: bool,
	#[clap(long, default_value = &DEFAULT_CONFIG_STR)]
	config: PathBuf,
}

impl Cmd for Stop {
	fn run(self) -> Result<()> {
		let lock_file = LockFile::new();
		let watchers = lock_file.get_running_watchers();
		if watchers.is_empty() {
			println!("No instance was found running.");
		} else {
			let sys = System::new_with_specifics(RefreshKind::with_processes(RefreshKind::new()));
			if self.all {
				for process in sys.get_process_by_name(crate_name!()) {
					process.kill(Signal::Kill);
				}
			} else {
				match lock_file.get_process_by(self.config.as_path()) {
					Some((pid, _)) => {
						sys.get_process(pid).unwrap().kill(Signal::Kill);
					}
					None => {
						println!("No instance was found running with configuration: {}", self.config.display());
					}
				}
			}
		}
		Ok(())
	}
}
