use serde::{Serialize, Serializer};
use crate::data::options::recursive::Recursive;
use notify::RecursiveMode;
use serde::ser::SerializeStruct;

impl Serialize for Recursive {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("Recursive", 2)?;
        if let Some(enabled) = self.enabled {
            let as_bool = enabled == RecursiveMode::Recursive;
            state.serialize_field("enabled", &as_bool)?;
        }
        if let Some(depth) = self.depth {
            state.serialize_field("depth", &depth)?;
        }
        state.end()
    }
}