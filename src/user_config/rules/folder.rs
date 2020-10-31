use crate::path::Expandable;
use serde::{
    de,
    de::{Error, MapAccess, Visitor},
    export,
    export::PhantomData,
    Deserialize,
    Deserializer,
    Serialize,
};
use std::{fmt, ops::Deref, path::PathBuf, result, str::FromStr};

fn deserialize_path<'de, D>(deserializer: D) -> Result<PathBuf, D::Error>
where
    D: Deserializer<'de>,
{
    let buf = String::deserialize(deserializer)?;
    let path = PathBuf::from(&buf).expand_user().expand_vars();
    match path.canonicalize() {
        Ok(path) => Ok(path),
        Err(e) => Err(D::Error::custom(e)),
    }
}

#[derive(Serialize, Debug, PartialEq, Eq, Clone)]
#[serde(deny_unknown_fields)]
pub struct Folder {
    #[serde(deserialize_with = "deserialize_path")]
    pub path: PathBuf,
    #[serde(default)]
    pub options: Options,
}

impl<'de> Deserialize<'de> for Folder {
    fn deserialize<D>(deserializer: D) -> result::Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        struct StringOrStruct(PhantomData<fn() -> Self>);

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
                    folder.options = options;
                }
                Ok(folder)
            }
        }
        deserializer.deserialize_any(StringOrStruct(PhantomData))
    }
}

impl FromStr for Folder {
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let path = PathBuf::from(s);
        match path.expand_user().expand_vars().canonicalize() {
            Ok(path) => Ok(Self {
                path,
                options: Default::default(),
            }),
            Err(e) => Err(e),
        }
    }
}

// #[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
// pub struct WrappedFolder(Folder);
//
// impl Deref for WrappedFolder {
//     type Target = Folder;
//
//     fn deref(&self) -> &Self::Target {
//         &self.0
//     }
// }
//
pub type Folders = Vec<Folder>;

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Options {
    /// defines whether or not subdirectories must be scanned
    #[serde(default)]
    pub recursive: bool,
    #[serde(default)]
    pub watch: bool,
    #[serde(default)]
    pub ignore: Vec<PathBuf>,
    #[serde(default)]
    pub hidden_files: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            recursive: false,
            watch: true,
            hidden_files: false,
            ignore: Vec::new(),
        }
    }
}
