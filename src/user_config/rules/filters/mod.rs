pub mod extension;
pub mod filename;
pub mod regex;

use crate::user_config::rules::{actions::script::Script, filters::regex::Regex};
use extension::Extension;
use filename::Filename;
use serde::Deserialize;
use std::{ops::Deref, path::Path};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all(deserialize = "lowercase", serialize = "lowercase"))]
pub enum Filter {
    Regex(Regex),
    Filename(Filename),
    Extension(Extension),
    Script(Script),
}

pub trait AsFilter {
    fn matches(&self, path: &Path) -> bool;
}

impl AsFilter for Filter {
    fn matches(&self, path: &Path) -> bool {
        match self {
            Filter::Regex(regex) => regex.matches(path),
            Filter::Filename(filename) => filename.matches(path),
            Filter::Extension(extension) => extension.matches(path),
            Filter::Script(script) => script.matches(path),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Filters(Vec<Filter>);

impl Deref for Filters {
    type Target = Vec<Filter>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Filters {
    pub fn r#match(&self, path: &Path) -> bool {
        let mut matches = true;
        for filter in self.iter() {
            matches = matches && filter.matches(path)
        }
        matches
    }
}
