use crate::cmd::edit::Edit;
use crate::cmd::Cmd;
use clap::Clap;
use organize_core::data::config::Config;
use anyhow::Result;
use std::path::PathBuf;
use organize_core::logger::Logger;

#[derive(Clap, Debug)]
pub struct New {
	#[clap(skip)]
	inner: bool,
    #[clap(long = "in")]
    folder: Option<PathBuf>
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
