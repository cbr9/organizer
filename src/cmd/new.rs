use crate::cmd::edit::Edit;
use crate::cmd::Cmd;
use clap::Clap;
use organize_core::data::config::Config;
use anyhow::Result;
use std::env;

#[derive(Clap, Debug)]
pub struct New {
	#[clap(skip)]
	inner: bool,
}

impl Cmd for New {
	fn run(self) -> Result<()> {
		let path = Config::create_in(env::current_dir()?)?;
		Edit::launch_editor(&path)?;
		println!("new config file created at {}", path.display());
		Ok(())
	}
}
