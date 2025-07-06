use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalSettings {
    pub copy_threshold: u64,
}

impl Default for GlobalSettings {
    fn default() -> Self {
        Self {
            copy_threshold: 1024 * 1024 * 1024, // Default to 1GB
        }
    }
}
