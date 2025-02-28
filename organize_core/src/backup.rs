use anyhow::{bail, Context, Result};
use bson::{doc, Bson, Document};
use chrono::{DateTime, Utc};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::File;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::{
	fs,
	path::PathBuf,
	sync::{Arc, LazyLock, Mutex},
};
use tracing::debug;
use uuid::Uuid;

use crate::PROJECT_NAME;

pub static LOCAL: LazyLock<PathBuf> = LazyLock::new(|| {
	let path = dirs::data_local_dir().unwrap().join(PROJECT_NAME);
	std::fs::create_dir_all(&path).unwrap();
	path
});
pub static DATABASE: LazyLock<Arc<Mutex<Database>>> = LazyLock::new(|| Arc::new(Mutex::new(Database::new().unwrap())));
pub static BACKUP_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
	let path = LOCAL.join(".backup");
	std::fs::create_dir_all(&path).unwrap();
	path
});

pub struct Database {
	path: PathBuf,
	data: Vec<Event>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Event {
	id: i32,
	timestamp: String,
	next_event_id: Option<i32>,
	last_event_id: Option<i32>,
	data: Value,
}

impl Event {
	fn get_id() -> i32 {
		let uuid = Uuid::new_v4();

		let mut hasher = DefaultHasher::new();
		uuid.hash(&mut hasher);
		let hash = hasher.finish();

		// Truncate the hash to fit an i32
		(hash % i32::MAX as u64) as i32
	}

	fn from_value(data: Value, last_event: Option<i32>) -> Self {
		Event {
			id: Self::get_id(),
			timestamp: Utc::now().to_string(),
			next_event_id: None,
			last_event_id: last_event,
			data,
		}
	}
}

impl Database {
	fn new() -> Result<Self> {
		let path = LOCAL.join("history.json");
		let file = match File::open(&path) {
			Ok(file) => file,
			Err(_) => {
				let _ = File::create(&path);
				File::open(&path).unwrap()
			}
		};

		let data: Vec<Event> = serde_json::from_reader(file)
			.context("cannot deserialize history")
			.unwrap_or_default();
		Ok(Self { path, data })
	}

	pub fn insert(&mut self, entry: Value, last_event_id: Option<i32>) -> Result<i32> {
		let event = Event::from_value(entry, last_event_id);

		if let Some(last_event_id) = last_event_id {
			let last_event_index = self
				.data
				.iter()
				.position(|e| e.id == last_event_id)
				.ok_or(format!("Event with ID {} not found", last_event_id))
				.unwrap();

			// Update the next_event_id
			self.data[last_event_index].next_event_id = Some(event.id);
		}
		self.data.push(event.clone());
		self.persist()?;
		Ok(event.id)
	}

	fn find_event_index_by_id(&self, id: i32) -> Option<usize> {
		self.data.iter().position(|row| &row.id == &id)
	}

	pub fn add_backup_path(&mut self, id: i32, path: PathBuf) -> Result<()> {
		let event_index = self.find_event_index_by_id(id).unwrap();
		if let Some(obj) = self.data[event_index].data.as_object_mut() {
			obj.insert("backup".to_string(), path.to_string_lossy().into());
			self.persist()?;
		}

		Ok(())
	}

	fn drop(&mut self, id: i32) {
		if let Some(index) = self.find_event_index_by_id(id) {
			self.data.remove(index);
			self.persist().unwrap();
		}
	}

	fn get_next_event(&self, current_event_id: i32) -> Option<&Event> {
		let next_event_id = self
			.data
			.iter()
			.position(|event| event.id == current_event_id)
			.and_then(|index| self.data[index].next_event_id)?;

		self.data
			.iter()
			.position(|event| event.id == next_event_id)
			.and_then(|index| self.data.get(index))
	}

	fn persist(&self) -> Result<()> {
		// Serialize the data to JSON
		let json_str = serde_json::to_string_pretty(&self.data)?;
		fs::write(&self.path, json_str).context("could not write history")
	}
}
