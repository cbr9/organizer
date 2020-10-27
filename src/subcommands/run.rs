use crate::CONFIG;
use std::{fs, io::Result, path::Path};

use dialoguer::{theme::ColorfulTheme, Select};

use crate::{path::IsHidden, user_config::rules::actions::ConflictOption};

pub fn run() -> Result<()> {
    let path2rules = CONFIG.to_map();

    for (path, rules) in path2rules.iter() {
        let files = fs::read_dir(&path)?;
        'files: for file in files {
            let path = file.unwrap().path();
            if path.is_file() {
                'rules: for (rule, index) in rules.iter() {
                    let folder = rule.folders.get(*index).unwrap();
                    let options = &folder.options;
                    if path.is_hidden() && !options.hidden_files {
                        continue 'rules;
                    }
                    let filters = &rule.filters;
                    if filters.r#match(&path) {
                        rule.actions.run(path);
                        continue 'files;
                    }
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
