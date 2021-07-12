use crate::{Cmd, CONFIG_PATH_STR};
use anyhow::Result;
use clap::Clap;
use notify::RecursiveMode;
use organize_core::{
    data::{path_to_recursive::PathToRecursive, path_to_rules::PathToRules, Data},
    file::File,
};
use rayon::prelude::*;
use std::path::PathBuf;
use walkdir::{DirEntry, WalkDir};
use crate::cmd::logs::Logs;
use regex::Regex;
use colored::{ColoredString, Colorize};

#[derive(Clap, Debug)]
pub struct Undo;

impl Cmd for Undo {
    fn run(self) -> anyhow::Result<()> {
        let regex = Regex::new(r"/home/.*").unwrap();
        let logs = std::fs::read_to_string(Logs::path()).unwrap_or_default();
        logs.lines().into_iter().for_each(|line| {
            let changes = regex.find(line).unwrap().as_str().split("->").map(|str| str.trim()).collect_vec();
            let org = changes[0];
            let dst = changes[1];
            println!("{} {}", org, dst);
            std::fs::rename(dst, org).unwrap();
        });
        Ok(())
    }
}
