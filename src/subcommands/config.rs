use crate::{
    settings::Settings,
    user_config::{rules::options::Options, UserConfig},
    ARGS,
    CONFIG,
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
            apply,
        } = Settings::new().unwrap().defaults;
        println!(
            "recursive: {}",
            recursive.unwrap().to_string().bright_purple()
        );
        println!("watch: {}", watch.unwrap().to_string().bright_purple());
        println!(
            "hidden_files: {}",
            hidden_files.unwrap().to_string().bright_purple()
        );
        println!("ignored_directories: {:?}", ignore.unwrap());
        println!(
            "apply (actions): {}",
            apply
                .as_ref()
                .unwrap()
                .actions
                .as_ref()
                .unwrap()
                .to_string()
                .bright_purple()
        );
        println!(
            "apply (filters): {}",
            apply
                .as_ref()
                .unwrap()
                .filters
                .as_ref()
                .unwrap()
                .to_string()
                .bright_purple()
        );
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
    match env::var_os("EDITOR") {
        Some(prog) => prog,
        None => panic!("Could not find any EDITOR environment variable or it's not properly set"),
    }
}
