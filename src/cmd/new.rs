use crate::cmd::{edit::Edit, Cmd};
use anyhow::Result;
use clap::Parser;
use organize_core::{data::config::Config, logger::Logger};
use std::path::PathBuf;

#[derive(Parser, Debug)]
pub struct New {
	#[arg(long = "in")]
	folder: Option<PathBuf>,
}

impl Cmd for New {
	fn run(self) -> Result<()> {
		Logger::setup(false)?;
		let path = match self.folder {
			None => Config::create_in_cwd()?,
			Some(folder) => Config::create_in(folder)?,
		};
		Edit::launch_editor(&path)?;
		println!("new config file created at {}", path.display());
		Ok(())
	}
}
