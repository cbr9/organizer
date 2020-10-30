use crate::CONFIG;
use std::{fs, io::Result, path::Path};

use dialoguer::{theme::ColorfulTheme, Select};

use crate::{
    subcommands::watch::process_file,
    user_config::rules::{actions::ConflictOption, folder::Options},
};
use std::borrow::Borrow;

pub fn run() -> Result<()> {
    let path2rules = CONFIG.to_map();
    let mut files = Vec::new();
    for (path, _) in path2rules.iter() {
        files.extend(fs::read_dir(path)?)
    }

    for file in files {
        let path = file.unwrap().path();
        process_file(path, path2rules.borrow(), false);
    }
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
