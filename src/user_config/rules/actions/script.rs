use crate::user_config::rules::actions::AsAction;
use crate::{
    string::Placeholder,
    user_config::{rules::filters::AsFilter, UserConfig},
};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::{
    fs,
    io::Result,
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
    fn act(&self, path: &mut Cow<Path>) -> Result<()> {
        match self.helper(path) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }
}

impl AsFilter for Script {
    fn matches(&self, path: &Path) -> bool {
        let output = self.helper(&mut Cow::from(path));
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
        let content = self.content.expand_placeholders(path)?;
        let dir = UserConfig::dir().join("scripts");
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }
        let script = dir.join("temp_script");
        fs::write(&script, content)?;
        Ok(script)
    }

    fn helper(&self, path: &mut Cow<Path>) -> Result<Output> {
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
