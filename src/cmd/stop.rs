use crate::{cmd::Cmd, DEFAULT_CONFIG_STR};
use anyhow::Result;
use clap::Clap;
use lib::register::Register;
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
		let register = Register::new()?;
		if register.is_empty() {
			println!("No instance was found running.");
		} else {
			let sys = System::new_with_specifics(RefreshKind::with_processes(RefreshKind::new()));
			if self.all {
				for section in register.iter() {
					sys.get_process(section.pid).unwrap().kill(Signal::Kill);
				}
			} else {
				match register.iter().find(|section| section.path == self.config) {
					Some(section) => {
						sys.get_process(section.pid).unwrap().kill(Signal::Kill);
					}
					None => println!("No instance was found running with configuration: {}", self.config.display()),
				}
			}
		}
		Ok(())
	}
}
