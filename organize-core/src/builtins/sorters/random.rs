use crate::{resource::Resource, sorter::Sorter};
use async_trait::async_trait;
use rand::{rng, seq::SliceRandom};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RandomSorter;

#[async_trait]
#[typetag::serde(name = "random")]
impl Sorter for RandomSorter {
	async fn sort(&self, files: &mut [Arc<Resource>]) {
		let mut rng = rng();
		files.shuffle(&mut rng);
	}
}
