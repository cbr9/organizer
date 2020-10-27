use crate::string::Placeholder;
use crate::user_config::rules::actions::AsAction;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::{io::Result, ops::Deref, path::Path};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Echo(String);

impl Deref for Echo {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsAction for Echo {
    fn act(&self, path: &mut Cow<Path>) -> Result<()> {
        println!("{}", self.deref().expand_placeholders(path)?);
        Ok(())
    }
}
