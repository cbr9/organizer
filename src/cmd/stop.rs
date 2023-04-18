// use std::path::PathBuf;

// use anyhow::Result;
// use clap::Clap;
// use sysinfo::{ProcessExt, RefreshKind, Signal, System, SystemExt};

// use organize_core::{logger::Logger, register::Register};

// use crate::{cmd::Cmd, CONFIG_PATH_STR};

// #[derive(Clap, Debug)]
// pub struct Stop {
// 	#[clap(long)]
// 	all: bool,
// 	#[clap(long, default_value = & CONFIG_PATH_STR)]
// 	pub(crate) config: PathBuf,
// 	#[clap(long, about = "Do not print colored output")]
// 	pub(crate) no_color: bool,
// }

// impl Cmd for Stop {
// 	fn run(mut self) -> Result<()> {
// 		self.config = self.config.canonicalize()?;
// 		Logger::setup(self.no_color)?;

// 		let register = Register::new()?;
// 		if register.is_empty() {
// 			println!("No instance was found running.");
// 		} else {
// 			let sys = System::new_with_specifics(RefreshKind::with_processes(RefreshKind::new()));
// 			if self.all {
// 				for process in register.iter() {
// 					if let Some(process) = sys.get_process(process.pid) {
// 						process.kill(Signal::Term);
// 					}
// 				}
// 			} else {
// 				match register.iter().find(|section| section.path == self.config) {
// 					Some(process) => {
// 						if let Some(process) = sys.get_process(process.pid) {
// 							process.kill(Signal::Term);
// 						}
// 					}
// 					None => println!("No instance was found running with configuration: {}", self.config.display()),
// 				}
// 			}
// 		}
// 		Ok(())
// 	}
// }
