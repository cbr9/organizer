use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{
    batch::Batch,
    errors::Error,
    grouper::Grouper,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionGrouper;

#[async_trait]
#[typetag::serde(name = "extension")]
impl Grouper for ExtensionGrouper {
    fn name(&self) -> &str {
        self.typetag_name()
    }

    async fn group(&self, batch: &Batch) -> Result<HashMap<String, Batch>, Error> {
        let mut groups: HashMap<String, Batch> = HashMap::new();
        for resource in &batch.files {
            let extension = resource
                .path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("no_extension")
                .to_string();
            groups
                .entry(extension.clone())
                .or_insert_with(Batch::new)
                .files
                .push(resource.clone());
        }
        Ok(groups)
    }
}
