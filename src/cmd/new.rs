use clap::Clap;
use crate::cmd::Cmd;
use organize_core::data::config::UserConfig;
use std::env;
use clap::crate_name;
use crate::cmd::edit::Edit;

#[derive(Clap, Debug)]
pub struct New {
    #[clap(skip)]
    inner: bool,
}

impl Cmd for New {
    fn run(self) -> anyhow::Result<()> {
        let config_file = env::current_dir()?.join(format!("{}.yml", crate_name!()));
        UserConfig::create(&config_file)?;
        Edit::launch_editor(config_file).map(|_| ())
    }
}