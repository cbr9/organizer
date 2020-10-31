use crate::CONFIG;
use rayon::prelude::*;
use std::{fs, io::Result, path::Path};

use dialoguer::{theme::ColorfulTheme, Select};

use crate::{
    subcommands::watch::process_file,
    user_config::rules::actions::io_action::ConflictOption,
};

pub fn run() -> Result<()> {
    let path2rules = CONFIG.to_map();

    let dirs: Vec<_> = path2rules
        .par_iter()
        .map(|(path, _)| {
            let path = path.to_path_buf();
            fs::read_dir(path).unwrap()
        })
        .collect();

    dirs.into_par_iter().for_each(|dir| {
        dir.collect::<Vec<_>>().into_par_iter().for_each(|file| {
            let path = file.unwrap().path();
            process_file(path, &path2rules, false)
        });
    });

    Ok(())
}

pub fn resolve_conflict(path: &Path) -> ConflictOption {
    let selections = ["Overwrite", "Rename", "Skip"];
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(format!(
            "A file named {} already exists in {}.\nSelect an option and press Enter to resolve this issue:",
            path.file_name().unwrap().to_str().unwrap(),
            if path.is_dir() {
                path.display()
            } else {
                path.parent().unwrap().display()
            }
        ))
        .default(0)
        .items(&selections[..])
        .interact()
        .unwrap();

    match selection {
        0 => ConflictOption::Overwrite,
        1 => ConflictOption::Rename,
        2 => ConflictOption::Skip,
        _ => panic!("no option selected"),
    }
}
