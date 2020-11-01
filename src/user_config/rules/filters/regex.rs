use crate::user_config::rules::filters::{extension::Extension, AsFilter};
use serde::{Deserialize, Deserializer};
use std::{ops::Deref, path::Path, str::FromStr};

#[derive(Debug, Clone)]
pub struct Regex(pub Vec<regex::Regex>);

impl Deref for Regex {
    type Target = Vec<regex::Regex>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsFilter for Regex {
    fn matches(&self, path: &Path) -> bool {
        for regex in self.iter() {
            if regex.is_match(path.to_str().unwrap()) {
                return true;
            }
        }
        false
    }
}

impl Regex {}

impl From<Vec<&str>> for Regex {
    fn from(vec: Vec<&str>) -> Self {
        let vec = vec
            .iter()
            .map(|str| regex::Regex::new(str).unwrap())
            .collect::<Vec<_>>();
        Self(vec)
    }
}

impl FromStr for Regex {
    type Err = regex::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match regex::Regex::new(s) {
            Ok(regex) => Ok(Regex(vec![regex])),
            Err(e) => Err(e),
        }
    }
}

impl<'de> Deserialize<'de> for Regex {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        let vec = Extension::deserialize(deserializer)? // the Extension deserializer is a plain String or Vec deserializer
            .iter()
            .map(|str| regex::Regex::new(str).unwrap())
            .collect::<Vec<_>>();
        Ok(Regex(vec))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::Error as YamlError;
    #[test]
    fn deserialize_single() -> Result<(), YamlError> {
        // only needs to test the deserialize implementation, because it's just a wrapper around a struct from a different crate
        let regex: Result<Regex, YamlError> = serde_yaml::from_str(".*");
        regex.and_then(|_| Ok(()))
    }

    #[test]
    fn deserialize_mult() -> Result<(), YamlError> {
        // only needs to test the deserialize implementation, because it's just a wrapper around a struct from a different crate
        let regex: Result<Regex, YamlError> = serde_yaml::from_str("[.*]");
        regex.and_then(|_| Ok(()))
    }
}
