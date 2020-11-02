use rayon::prelude::*;
use std::{fs, io::Result};

use crate::{subcommands::watch::process_file, user_config::PathToRules, CONFIG};
use std::{borrow::Cow, ops::Deref};

pub fn run() -> Result<()> {
    let cow = Cow::Borrowed(CONFIG.deref());
    let path2rules = PathToRules::from(&cow);

    path2rules
        .par_iter()
        .map(|(path, _)| fs::read_dir(path).unwrap())
        .into_par_iter()
        .for_each(|dir| {
            dir.collect::<Vec<_>>().into_par_iter().for_each(|file| {
                let path = file.unwrap().path();
                process_file(&path, &path2rules, false)
            });
        });

    Ok(())
}
