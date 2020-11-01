use crate::{
    string::{Placeholder, PlaceholderStr},
    user_config::{
        rules::{actions::AsAction, filters::AsFilter},
        UserConfig,
    },
};
use colored::Colorize;
use log::info;
use serde::{
    de::{Error, MapAccess, Visitor},
    export::Formatter,
    Deserialize,
    Deserializer,
};
use std::{
    borrow::Cow,
    fmt,
    fs,
    io::Result,
    ops::Deref,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    result,
    str::FromStr,
};

#[derive(Debug, Clone, Default)]
pub struct Script {
    exec: String,
    content: PlaceholderStr,
}

impl AsAction<Self> for Script {
    fn act<'a>(&self, path: Cow<'a, Path>) -> Result<Cow<'a, Path>> {
        match self.helper(&path) {
            Ok(_output) => {
                // improve output
                info!("({}) run script on {}", self.exec.bold(), path.display());
                Ok(path)
            }
            Err(e) => Err(e),
        }
    }
}

impl<'de> Deserialize<'de> for Script {
    fn deserialize<D>(deserializer: D) -> result::Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        struct ScriptVisitor;
        impl<'de> Visitor<'de> for ScriptVisitor {
            type Value = Script;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter.write_str("map")
            }

            fn visit_map<A>(self, mut map: A) -> result::Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut exec: Option<String> = None;
                let mut content: Option<PlaceholderStr> = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "exec" => exec = Some(map.next_value()?),
                        "content" => content = Some(map.next_value()?),
                        _ => {
                            return Err(A::Error::custom(
                                "unexpected field, expected exec or content",
                            ))
                        }
                    }
                }

                match &exec {
                    None => return Err(A::Error::custom("missing field 'exec'")),
                    Some(exec) => {
                        let mut command = std::process::Command::new(exec);
                        match command.spawn() {
                            Ok(mut child) => child.kill().unwrap(),
                            Err(_) => {
                                return Err(A::Error::custom(format!(
                                    "interpreter '{}' could not be run",
                                    exec
                                )))
                            }
                        }
                    }
                }

                if content.is_none() {
                    return Err(A::Error::custom("missing field 'content'"));
                }

                Ok(Script {
                    exec: exec.unwrap(),
                    content: content.unwrap(),
                })
            }
        }
        deserializer.deserialize_map(ScriptVisitor)
    }
}

impl AsFilter for Script {
    fn matches(&self, path: &Path) -> bool {
        let output = self.helper(path);
        match output {
            Ok(output) => {
                let output = String::from_utf8_lossy(&output.stdout);
                let parsed = bool::from_str(&output.trim().to_lowercase());
                println!("{:?}", parsed);
                match parsed {
                    Ok(boolean) => boolean,
                    Err(_) => false,
                }
            }
            Err(_) => false,
        }
    }
}

impl Script {
    fn write(&self, path: &Path) -> Result<PathBuf> {
        let content = self.content.as_str();
        let content = content.expand_placeholders(path)?;
        let dir = UserConfig::dir().join("scripts");
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }
        let script = dir.join("temp_script");
        fs::write(&script, content.deref())?;
        Ok(script)
    }

    fn helper(&self, path: &Path) -> Result<Output> {
        let script = self.write(path)?;
        let output = Command::new(&self.exec)
            .arg(&script)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("could not run script")
            .wait_with_output()
            .expect("script terminated with an error");
        fs::remove_file(script)?;
        Ok(output)
    }
}
