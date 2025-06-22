use crate::config::{
	context::{ExecutionContext, VariableCacheKey},
	variables::Variable,
};
use serde::{
	ser::{Error, Serializer},
	Serialize,
};

#[allow(clippy::borrowed_box)]
pub struct LazyVariable<'a> {
	pub variable: &'a Box<dyn Variable>,
	pub context: &'a ExecutionContext<'a>,
}

impl<'a> Serialize for LazyVariable<'a> {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let var_cache = &self.context.services.blackboard.variables;
		let cache_key = VariableCacheKey {
			variable: self.variable.name().to_string(),
			rule_index: self.context.scope.rule.index,
			resource: self.context.scope.resource.clone(),
		};

		if let Some(cached_value) = var_cache.get(&cache_key) {
			return cached_value.serialize(serializer);
		}

		println!("computing!!");
		let computed_value = match self.variable.compute(self.context) {
			Ok(val) => val,
			Err(e) => return Err(S::Error::custom(e.to_string())),
		};
		dbg!(&computed_value);

		var_cache.insert(cache_key, computed_value.clone());
		computed_value.serialize(serializer)
	}
}

// // NEW LAZY WRAPPER FOR FILE METADATA
// pub struct LazyMetadata<'a> {
// 	pub resource: &'a Resource,
// 	pub blackboard: &'a Arc<Blackboard>,
// }

// impl<'a> Serialize for LazyMetadata<'a> {
// 	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
// 	where
// 		S: Serializer,
// 	{
// 		// This is where you would perform the expensive I/O to read metadata.
// 		// For this example, we'll create a simple map. A real implementation
// 		// would use a library like `exif` and cache the result on the blackboard.
// 		let mut metadata_map = tera::Map::new();

// 		if let Ok(fs_meta) = self.resource.path().metadata() {
// 			metadata_map.insert("size".to_string(), tera::to_value(fs_meta.len()).unwrap());
// 			// ... add other filesystem metadata ...
// 		}

// 		// You could add a section for EXIF data here
// 		let mut exif_map = tera::Map::new();
// 		exif_map.insert("iso".to_string(), tera::to_value(800).unwrap()); // Dummy data
// 		exif_map.insert("f_number".to_string(), tera::to_value("f/2.8").unwrap()); // Dummy data
// 		metadata_map.insert("exif".to_string(), tera::to_value(exif_map).unwrap());

// 		metadata_map.serialize(serializer)
// 	}
// }
