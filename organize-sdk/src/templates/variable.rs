use crate::templates::schema::Property;

/// The plugin interface for a static variable provider.
///
/// Any struct implementing this trait can be automatically discovered and
/// integrated into the template engine's schema. This trait is intended for
/// built-in variables with fixed schemas, like `file` or `env`.
pub trait StatelessVariable: Sync + Send {
	/// Returns the canonical name of the root variable (e.g., "file", "env").
	fn name(&self) -> &'static str;

	/// Returns the schema for this variable, defining its properties and accessors.
	fn schema(&self) -> Property;
}

/// The collectible struct for the `inventory` crate.
/// It holds a static reference to an object that implements our `Variable` trait.
pub struct VariableInventory {
	pub provider: &'static (dyn StatelessVariable + Sync),
}

// Declare the global collection for automatic registration of static variable providers.
inventory::collect!(VariableInventory);
