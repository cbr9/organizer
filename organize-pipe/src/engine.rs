use std::collections::HashMap;

use crate::variable::Variable;

pub struct Engine {
	variables: HashMap<String, Box<dyn Variable>>,
}
