use crate::user_config::{rules::rule::Rule, PathToRules};
use std::path::{Path, PathBuf};
use std::collections::HashMap;

pub trait GetRules {
    fn get_rules<'a>(&self, path2rules: &'a HashMap<&'a Path, Vec<(&'a Rule, usize)>>) -> &'a Vec<(&'a Rule, usize)>;
}

impl GetRules for Path {
    fn get_rules<'a>(&self, path2rules: &'a HashMap<&'a Path, Vec<(&'a Rule, usize)>>) -> &'a Vec<(&'a Rule, usize)> {
        path2rules.get(self).unwrap_or_else(|| {
            // if the path is some subdirectory not represented in the hashmap
            let components = self.components().collect::<Vec<_>>();
            let mut paths = Vec::new();
            for i in 0..components.len() {
                let slice = components[0..i]
                    .iter()
                    .map(|comp| comp.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>();
                let str: String = slice.join(&std::path::MAIN_SEPARATOR.to_string());
                paths.push(PathBuf::from(str.replace("//", "/")))
            }
            let path = paths
                .iter()
                .rev()
                .find_map(|path| {
                    if path2rules.contains_key(path.as_path()) {
                        Some(path)
                    } else {
                        None
                    }
                })
                .unwrap();
            path2rules.get(path.as_path()).unwrap()
        })
    }
}
