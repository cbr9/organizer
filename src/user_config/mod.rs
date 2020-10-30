use crate::{
    path::{Expandable, Update},
    user_config::rules::{actions::ConflictOption, rule::Rule},
    ARGS,
};
use clap::crate_name;
use dirs::{config_dir, home_dir};
use serde::Deserialize;
use std::{
    borrow::Cow,
    collections::HashMap,
    env,
    fs,
    path::{Path, PathBuf},
};

pub mod rules;

/// Represents the user's configuration file
/// ### Fields
/// * `path`: the path the user's config, either the default one or some other passed with the --with-config argument
/// * `rules`: a list of parsed rules defined by the user
#[derive(Deserialize, Clone, Debug)]
pub struct UserConfig {
    pub rules: Vec<Rule>,
    #[serde(skip)]
    pub path: PathBuf,
}

impl Default for UserConfig {
    fn default() -> Self {
        let path = UserConfig::path();
        Self::new(path)
    }
}

impl UserConfig {
    /// Creates a new UserConfig instance.
    /// It parses the configuration file
    /// and fills missing fields with either the defaults, in the case of global options,
    /// or with the global options, in the case of folder-level options.
    /// If the config file does not exist, it is created.
    /// ### Errors
    /// This constructor fails in the following cases:
    /// - The configuration file does not exist
    fn new(path: PathBuf) -> Self {
        if path == UserConfig::default_path() {
            match home_dir() {
                None => panic!("error: cannot determine home directory"),
                Some(home) => env::set_current_dir(&home).unwrap(),
            };
        } else {
            env::set_current_dir(&path.parent().unwrap()).unwrap();
        };

        if !path.exists() {
            Self::create(&path);
        }

        let content = fs::read_to_string(&path).unwrap();
        let mut config: UserConfig =
            serde_yaml::from_str(&content).expect("could not parse config file");
        config.path = path;
        config
    }

    pub fn create(path: &Path) {
        let path = if path.exists() {
            path.update(&ConflictOption::Rename, &Default::default())
                .unwrap() // safe unwrap (can only return an error if if_exists == Skip)
        } else {
            Cow::Borrowed(path)
        };

        match path.parent() {
            Some(parent) => {
                if !parent.exists() {
                    std::fs::create_dir_all(parent).unwrap_or_else(|_| {
                        panic!(
                            "error: could not create config directory ({})",
                            parent.display()
                        )
                    });
                }
                let output = include_str!("../../examples/config.yml");
                std::fs::write(&path, output).unwrap_or_else(|_| {
                    panic!("error: could not create config file ({})", path.display())
                });
                println!("New config file created at {}", path.display());
            }
            None => panic!("config file's parent folder should be defined"),
        }
    }

    pub fn path() -> PathBuf {
        match ARGS.value_of("config") {
            Some(path) => PathBuf::from(path)
                .expand_user()
                .expand_vars()
                .canonicalize()
                .unwrap(),
            None => Self::default_path(),
        }
    }

    pub fn dir() -> PathBuf {
        let dir = config_dir()
            .expect("ERROR: cannot determine config directory")
            .join(crate_name!());
        if !dir.exists() {
            fs::create_dir_all(&dir).expect("error: could not create config directory");
        }
        dir
    }

    pub fn default_path() -> PathBuf {
        Self::dir().join("config.yml")
    }

    /// returns a hashmap where the keys are paths and the values are tuples of rules
    /// and indices, which indicate the index of the key's corresponding folder in the rule's folders' list
    /// (i.e. the key is the ith folder in the corresponding rule)
    pub fn to_map(&self) -> HashMap<&Path, Vec<(&Rule, usize)>> {
        let mut map = HashMap::new();
        for rule in self.rules.iter() {
            for (i, folder) in rule.folders.iter().enumerate() {
                if !map.contains_key(folder.path.as_path()) {
                    map.insert(folder.path.as_path(), Vec::new());
                }
                map.get_mut(folder.path.as_path()).unwrap().push((rule, i));
            }
        }
        map
    }
}
