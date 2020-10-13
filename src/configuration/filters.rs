use serde::{
    Deserialize,
    Serialize,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct Filters {
    pub regex: String,
    pub filename: String,
    pub extensions: Vec<String>,
}

impl Default for Filters {
    fn default() -> Self {
        Filters {
            regex: String::new(),
            filename: String::new(),
            extensions: Vec::new(),
        }
    }
}

#[allow(dead_code)]
struct Filename {
    startswith: Option<String>,
    endswith: Option<String>,
    contains: Option<String>,
    case_sensitive: Option<bool>,
}
