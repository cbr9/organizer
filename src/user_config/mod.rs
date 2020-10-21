use crate::{path::Expandable, user_config::rules::rule::Rule};
use clap::{crate_name, load_yaml, ArgMatches};
use dirs::home_dir;
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs,
    io::{Error, ErrorKind, Result},
    path::{Path, PathBuf},
};
use yaml_rust::YamlEmitter;

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

impl UserConfig {
    /// Creates a new UserConfig instance.
    /// It parses the configuration file
    /// and fills missing fields with either the defaults, in the case of global options,
    /// or with the global options, in the case of folder-level options.
    /// If the config file does not exist, it is created.
    /// ### Errors
    /// This constructor fails in the following cases:
    /// - The configuration file does not exist
    pub fn new(args: &ArgMatches) -> Result<Self> {
        let path = UserConfig::path(args);

        if !path.exists() {
            Self::create(&path)?;
        }

        let content = fs::read_to_string(&path)?;
        let mut config: UserConfig = serde_yaml::from_str(&content).expect("could not parse config file");
        config.path = path;
        for (i, rule) in config.rules.iter().enumerate() {
            let action = &rule.actions;
            if action.r#move.is_some() && action.rename.is_some() {
                panic!(
                    "error: tried declaring both a `move` and `rename` action, which are incompatible (rule no. {})",
                    i
                )
            }
        }

        Ok(config)
    }

    pub fn create(path: &Path) -> Result<()> {
        // safe unwrap, dir is created at $HOME or $UserProfile%,
        // so it exists and the user must have permissions
        if path.exists() {
            return Err(Error::new(
                ErrorKind::AlreadyExists,
                format!(
                    "{} already exists in this directory",
                    path.file_name().unwrap().to_str().unwrap()
                ),
            ));
        }
        match path.parent() {
            Some(parent) => {
                if !parent.exists() {
                    std::fs::create_dir_all(path.parent().unwrap())?;
                }
                let config = load_yaml!("../../examples/config.yml");
                let mut output = String::new();
                let mut emitter = YamlEmitter::new(&mut output);
                emitter.dump(config).expect("ERROR: could not create starter config");
                std::fs::write(path, output)?;
            }
            None => panic!("home directory's parent folder should be defined"),
        }
        Ok(())
    }

    pub fn path(args: &ArgMatches) -> PathBuf {
        match args.subcommand().unwrap().1.value_of("config") {
            Some(path) => PathBuf::from(path).expand_user().expand_vars().canonicalize().unwrap(),
            None => Self::default_path(),
        }
    }

    pub fn dir() -> PathBuf {
        home_dir()
            .expect("ERROR: cannot determine home directory")
            .join(format!(".{}", crate_name!()))
    }

    pub fn default_path() -> PathBuf {
        Self::dir().join("config.yml")
    }

    /// returns a hashmap where the keys are paths and the values are tuples of rules
    /// and indices, which indicate the index of the key's corresponding folder in the rule's folders' list
    /// (i.e. the key is the ith folder in the corresponding rule)
    pub fn to_map(&self) -> HashMap<&PathBuf, Vec<(&Rule, usize)>> {
        let mut map = HashMap::new();
        for rule in self.rules.iter() {
            for (i, folder) in rule.folders.iter().enumerate() {
                if !map.contains_key(&folder.path) {
                    map.insert(&folder.path, Vec::new());
                }
                map.get_mut(&folder.path).unwrap().push((rule, i));
            }
        }
        map
    }
}
