use std::{
    fs,
    io::{
        Error,
        ErrorKind,
    },
};

use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    cli::{
        config_path,
        default_config,
        Cli,
    },
    user_config::rules::rule::Rule,
    utils,
    utils::expand_env_vars,
};
use std::path::PathBuf;

/// Represents the user's configuration file
/// ### Fields
/// * `path`: the path the user's config, either the default one or some other passed with the --with-config argument
/// * `rules`: a list of parsed rules defined by the user
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct UserConfig {
    #[serde(default = "default_config")]
    pub path: PathBuf,
    pub rules: Vec<Rule>,
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
    pub fn new(cli: &Cli) -> Result<Self, Error> {
        let path = config_path(cli);

        if !path.exists() {
            utils::create_config_file(&path)?;
        }

        let content = fs::read_to_string(&path)?;
        let mut config: Self = serde_yaml::from_str(&content).expect("could not parse config file");
        // config.path = path;

        for rule in config.rules.iter_mut() {
            for folder in rule.folders.iter_mut() {
                folder.path = expand_env_vars(&folder.path);
            }
        }

        Ok(config)
    }
}