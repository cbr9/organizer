use crate::{
    path::expand::Expandable,
    settings::Settings,
    user_config::{
        rules::{options::Options, rule::Rule},
        UserConfig,
    },
};
use serde::{
    de,
    de::{MapAccess, Visitor},
    export,
    export::PhantomData,
    Deserialize,
    Deserializer,
    Serialize,
};
use std::{fmt, path::PathBuf, result, str::FromStr};

#[derive(Serialize, Debug, PartialEq, Eq, Clone)]
pub struct Folder {
    pub path: PathBuf,
    pub options: Option<Options>,
}

impl Folder {
    pub fn fill_options<S, C, R>(&self, settings: &S, config: &C, rule: &R) -> Option<Options>
    where
        S: AsRef<Settings>,
        C: AsRef<UserConfig>,
        R: AsRef<Rule>,
    {
        let mut options = settings.as_ref().defaults.clone();
        if let Some(config_defaults) = &config.as_ref().defaults {
            options = &options + config_defaults;
        }
        if let Some(rule_options) = &rule.as_ref().options {
            options = &options + rule_options;
        }
        if let Some(folder_options) = &self.options {
            options = &options + folder_options;
        }
        Some(options)
    }
}

impl<'de> Deserialize<'de> for Folder {
    fn deserialize<D>(deserializer: D) -> result::Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        struct StringOrStruct;

        impl<'de> Visitor<'de> for StringOrStruct {
            type Value = Folder;

            fn expecting(&self, formatter: &mut export::Formatter) -> fmt::Result {
                formatter.write_str("string or map")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Folder::from_str(v).unwrap())
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut path: Option<String> = None;
                let mut options: Option<Options> = None;
                while let Some(key) = map.next_key::<String>()? {
                    if key == "path" {
                        path = Some(map.next_value()?);
                    } else if key == "options" {
                        options = Some(map.next_value()?);
                    } else {
                        return Err(serde::de::Error::custom(&format!("Invalid key: {}", key)));
                    }
                }
                if path.is_none() {
                    return Err(serde::de::Error::custom("Missing path"));
                }

                let mut folder = match Folder::from_str(path.unwrap().as_str()) {
                    Ok(folder) => folder,
                    Err(e) => {
                        return Err(serde::de::Error::custom(&format!(
                            "Path does not exist: {}",
                            e
                        )))
                    }
                };
                if let Some(options) = options {
                    folder.options = Some(options);
                }
                Ok(folder)
            }
        }
        deserializer.deserialize_any(StringOrStruct)
    }
}

impl FromStr for Folder {
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let path = PathBuf::from(s);
        match path.expand_user().expand_vars().canonicalize() {
            Ok(path) => Ok(Self {
                path,
                options: None,
            }),
            Err(e) => Err(e),
        }
    }
}

pub type Folders = Vec<Folder>;
