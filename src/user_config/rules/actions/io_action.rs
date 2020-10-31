use crate::{
    path::{Expandable, Update},
    string::Placeholder,
    user_config::rules::actions::{ActionType, AsAction},
};
use colored::Colorize;
use log::info;
use serde::{
    de,
    de::{MapAccess, Visitor},
    export,
    export::PhantomData,
    Deserialize,
    Deserializer,
    Serialize,
};
use std::{
    borrow::Cow,
    convert::Infallible,
    fmt,
    fs,
    io::{Error, ErrorKind, Result},
    ops::Deref,
    path::{Path, PathBuf},
    result,
    str::FromStr,
};

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub struct Sep(String);

impl Deref for Sep {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for Sep {
    fn default() -> Self {
        Self(" ".into())
    }
}

#[derive(Debug, Clone, Serialize, Eq, PartialEq, Default)]
pub struct IOAction {
    pub to: PathBuf,
    pub if_exists: ConflictOption,
    pub sep: Sep,
}

pub(super) struct Move;
pub(super) struct Rename;
pub(super) struct Copy;

impl AsAction<Move> for IOAction {
    fn act<'a>(&self, path: Cow<'a, Path>) -> Result<Cow<'a, Path>> {
        let to = Self::helper(&path, self, ActionType::Move)?;
        std::fs::rename(&path, &to)?;
        info!(
            "({}) {} -> {}",
            ActionType::Move.to_string().bold(),
            path.display(),
            to.display()
        );
        Ok(Cow::Owned(to))
    }
}

impl AsAction<Rename> for IOAction {
    fn act<'a>(&self, path: Cow<'a, Path>) -> Result<Cow<'a, Path>> {
        let to = IOAction::helper(&path, self, ActionType::Rename)?;
        fs::rename(&path, &to)?;
        info!(
            "({}) {} -> {}",
            ActionType::Rename.to_string().bold(),
            path.display(),
            to.display()
        );
        Ok(Cow::Owned(to))
    }
}

impl AsAction<Copy> for IOAction {
    fn act<'a>(&self, path: Cow<'a, Path>) -> Result<Cow<'a, Path>> {
        let to = IOAction::helper(&path, self, ActionType::Copy)?;
        std::fs::copy(&path, &to)?;
        info!(
            "({}) {} -> {}",
            ActionType::Copy.to_string().bold(),
            path.display(),
            to.display()
        );
        Ok(path)
    }
}

impl<'de> Deserialize<'de> for IOAction {
    fn deserialize<D>(deserializer: D) -> result::Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        struct StringOrStruct(PhantomData<fn() -> IOAction>);

        impl<'de> Visitor<'de> for StringOrStruct {
            type Value = IOAction;

            fn expecting(&self, formatter: &mut export::Formatter) -> fmt::Result {
                formatter.write_str("string or map")
            }

            fn visit_str<E>(self, value: &str) -> result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(IOAction::from_str(value).unwrap())
            }

            fn visit_map<M>(self, mut map: M) -> result::Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut to: Option<String> = None;
                let mut if_exists: Option<ConflictOption> = None;
                let mut sep: Option<Sep> = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "to" => to = Some(map.next_value()?),
                        "if_exists" => if_exists = Some(map.next_value()?),
                        "sep" => sep = Some(map.next_value()?),
                        _ => {
                            return Err(serde::de::Error::custom(&format!("Invalid key: {}", key)))
                        }
                    }
                }
                if to.is_none() {
                    return Err(serde::de::Error::custom("Missing path"));
                }
                let mut action = IOAction::from_str(to.unwrap().as_str()).unwrap();
                if let Some(if_exists) = if_exists {
                    action.if_exists = if_exists;
                }
                if let Some(sep) = sep {
                    action.sep = sep;
                }
                Ok(action)
            }
        }
        deserializer.deserialize_any(StringOrStruct(PhantomData))
    }
}

impl FromStr for IOAction {
    type Err = Infallible;

    fn from_str(s: &str) -> result::Result<Self, Self::Err> {
        let path = PathBuf::from(s);
        Ok(Self {
            to: path.expand_user().expand_vars(),
            if_exists: Default::default(),
            sep: Default::default(),
        })
    }
}

impl IOAction {
    fn helper(path: &Path, action: &IOAction, kind: ActionType) -> Result<PathBuf> {
        // TODO: refactor this mess
        #[cfg(debug_assertions)]
        debug_assert!([ActionType::Move, ActionType::Rename, ActionType::Copy].contains(&kind));

        let mut to: PathBuf = action
            .to
            .to_str()
            .unwrap()
            .expand_placeholders(path)?
            .deref()
            .into();
        if kind == ActionType::Copy || kind == ActionType::Move {
            if !to.exists() {
                fs::create_dir_all(&to)?;
            }
            to = to.canonicalize().unwrap();
            to.push(
                path.file_name()
                    .ok_or_else(|| Error::new(ErrorKind::Other, "path has no filename"))?,
            );
        }

        if to.exists() {
            match to.update(&action.if_exists, &action.sep) {
                // FIXME: avoid the into_owned() call
                Ok(new_path) => to = new_path.into_owned(),
                Err(e) => return Err(e),
            }
        }
        Ok(to)
    }
}

/// Defines the options available to resolve a naming conflict,
/// i.e. how the application should proceed when a file exists
/// but it should move/rename/copy some file to that existing path
#[derive(Eq, PartialEq, Debug, Clone, Copy, Deserialize, Serialize)]
// for the config schema to keep these options as lowercase (i.e. the user doesn't have to
// write `if_exists: Rename`), and not need a #[allow(non_camel_case_types)] flag, serde
// provides the option to modify the fields are deserialize/serialize time
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum ConflictOption {
    Overwrite,
    Skip,
    Rename,
    Ask, // not available when watching
}

impl Default for ConflictOption {
    fn default() -> Self {
        ConflictOption::Rename
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        path::lib::vars::test_file_or_dir,
        user_config::rules::actions::{io_action::IOAction, ActionType},
    };
    use std::{
        fs,
        io::{Error, ErrorKind, Result},
    };

    #[test]
    fn prepare_path_copy() -> Result<()> {
        let path = test_file_or_dir("test1.txt");
        let target = test_file_or_dir("test_dir");
        let expected = test_file_or_dir("test_dir").join("test1 (1).txt");
        if expected.exists() {
            fs::remove_file(&expected)?;
        }
        let action = IOAction {
            to: target,
            if_exists: Default::default(),
            sep: Default::default(),
        };
        let new_path = IOAction::helper(&path, &action, ActionType::Copy)?;
        if new_path == expected {
            Ok(())
        } else {
            Err(Error::from(ErrorKind::Other))
        }
    }

    #[test]
    fn prepare_path_move() -> Result<()> {
        let path = test_file_or_dir("test1.txt");
        let target = test_file_or_dir("test_dir");
        let expected = test_file_or_dir("test_dir").join("test1 (1).txt");
        if expected.exists() {
            fs::remove_file(&expected)?;
        }
        let action = IOAction {
            to: target,
            if_exists: Default::default(),
            sep: Default::default(),
        };
        let new_path = IOAction::helper(&path, &action, ActionType::Move)?;
        if new_path == expected {
            Ok(())
        } else {
            Err(Error::from(ErrorKind::Other))
        }
    }

    #[test]
    fn prepare_path_rename() -> Result<()> {
        let path = test_file_or_dir("test1.txt");
        let target = test_file_or_dir("test_dir").join("test1.txt");
        let expected = test_file_or_dir("test_dir").join("test1 (1).txt");
        if expected.exists() {
            fs::remove_file(&expected)?;
        }
        let action = IOAction {
            to: target,
            if_exists: Default::default(),
            sep: Default::default(),
        };
        let new_path = IOAction::helper(&path, &action, ActionType::Rename)?;
        if new_path == expected {
            Ok(())
        } else {
            Err(Error::from(ErrorKind::Other))
        }
    }
}
