use crate::user_config::rules::filters::AsFilter;
use serde::{Deserialize, Deserializer};
use std::{ops::Deref, path::Path};

#[derive(Debug, Clone)]
pub struct Regex(regex::Regex);

impl Deref for Regex {
    type Target = regex::Regex;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsFilter for Regex {
    fn matches(&self, path: &Path) -> bool {
        if self.is_match(path.to_str().unwrap()) {
            return true;
        }
        false
    }
}

impl<'de> Deserialize<'de> for Regex {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        let buf = String::deserialize(deserializer)?;
        let regex = regex::Regex::new(&buf).expect("error: could not parse config file (invalid regex)");
        Ok(Self(regex))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::Error as YamlError;

    #[test]
    fn deserialize() -> Result<(), YamlError> {
        let regex: Result<Regex, YamlError> = serde_yaml::from_str(".*");
        regex.and_then(|_| Ok(()))
    }
}
