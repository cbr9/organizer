use crate::{
    user_config::{rules::folder::Options, UserConfig},
    ARGS, CONFIG,
};
use clap::crate_name;
use colored::Colorize;
use std::{env, ffi::OsString, io::Result, path::PathBuf, process};

pub fn config() -> Result<()> {
    if ARGS.is_present("show_path") {
        println!("{}", CONFIG.path.display());
    } else if ARGS.is_present("show_defaults") {
        let Options {
            recursive,
            watch,
            ignore,
            hidden_files,
        } = Options::default();
        println!("recursive: {}", recursive.to_string().purple());
        println!("watch: {}", watch.to_string().purple());
        println!("hidden_files: {}", hidden_files.to_string().purple());
        println!("ignored_directories: {:?}", ignore);
    } else if ARGS.is_present("new") {
        let config_file = env::current_dir()?.join(format!("{}.yml", crate_name!()));
        UserConfig::create(&config_file);
    } else {
        edit(UserConfig::path())?;
    }
    Ok(())
}

/// Launches an editor to modify the default config.
/// This function represents the `config` subcommand without any arguments.
/// ### Errors
/// This functions returns an error in the following cases:
/// - There is no $EDITOR environment variable.
/// ### Panics
/// This functions panics in the following cases:
/// - The $EDITOR env. variable was found but its process could not be started.
fn edit(path: PathBuf) -> Result<()> {
    let editor = get_default_editor();
    process::Command::new(&editor).arg(path).spawn()?.wait()?;
    Ok(())
}

fn get_default_editor() -> OsString {
    if let Some(prog) = env::var_os("VISUAL") {
        return prog;
    }
    if let Some(prog) = env::var_os("EDITOR") {
        return prog;
    }
    if cfg!(windows) {
        "notepad.exe".into()
    } else {
        "vi".into()
    }
}
