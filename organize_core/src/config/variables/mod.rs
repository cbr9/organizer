use dyn_clone::DynClone;
use dyn_eq::DynEq;
use std::fmt::Debug;
use tera::Context;

pub mod regex;
pub mod simple;

dyn_clone::clone_trait_object!(Variable);
dyn_eq::eq_trait_object!(Variable);

#[typetag::serde(tag = "type")]
pub trait Variable: DynEq + DynClone + Sync + Send + Debug {
	fn register(&self, context: &mut Context);
}
