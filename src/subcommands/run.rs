use crate::CONFIG;
use std::{fs, io::Result, path::Path};

use dialoguer::{theme::ColorfulTheme, Select};

use crate::{
    path::IsHidden,
    user_config::rules::{actions::ConflictOption, folder::Options},
};

pub fn run() -> Result<()> {
    let path2rules = CONFIG.to_map();
    let mut files = Vec::new();
    for (path, _) in path2rules.iter() {
        files.extend(fs::read_dir(path)?)
    }

    for file in files {
        let path = file.unwrap().path();
        if path.is_file() {
            let parent = path.parent().unwrap();

            // FIXME: if using recursive = true, this will panic, because the parent won't be a key in path2rules
            'rules: for (rule, i) in path2rules.get(parent).unwrap() {
                let folder = rule.folders.get(*i).unwrap();
                let Options {
                    ignore,
                    hidden_files,
                    ..
                } = &folder.options;
                if ignore.contains(&parent.to_path_buf()) {
                    continue 'rules;
                }
                if path.is_hidden() && !*hidden_files {
                    continue 'rules;
                }
                if rule.filters.r#match(&path) {
                    rule.actions.run(path);
                    break 'rules;
                }
            }
        }
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
