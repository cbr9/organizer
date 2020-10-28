use crate::{
    string::Placeholder,
    user_config::{
        rules::{
            actions::{ActionType, AsAction},
            filters::AsFilter,
        },
        UserConfig,
    },
};
use log::info;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    fs,
    io::Result,
    ops::Deref,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    str::FromStr,
};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Script {
    exec: String,
    content: String,
}

impl AsAction for Script {
    fn act<'a>(&self, path: Cow<'a, Path>) -> Result<Cow<'a, Path>> {
        match self.helper(&path) {
            Ok(_) => {
                info!("({}) run script on {}", self.exec, path.display());
                Ok(path)
            }
            Err(e) => Err(e),
        }
    }

    fn kind(&self) -> ActionType {
        ActionType::Script
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
    pub fn write(&self, path: &Path) -> Result<PathBuf> {
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
