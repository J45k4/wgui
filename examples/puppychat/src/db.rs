use wgui::{Db, DbTable, HasId, Wdb, WguiModel};

#[derive(Debug, Clone, WguiModel, serde::Serialize, serde::Deserialize)]
pub struct Message {
	pub id: u32,
	pub author: String,
	pub body: String,
	pub image_url: String,
	pub time: String,
	pub channel_id: Option<u32>,
	pub dm_thread_key: Option<String>,
}

impl HasId for Message {
	fn id(&self) -> u32 {
		self.id
	}

	fn set_id(&mut self, id: u32) {
		self.id = id;
	}
}

#[derive(Debug, Clone, WguiModel, serde::Serialize, serde::Deserialize)]
pub struct Channel {
	pub id: u32,
	pub name: String,
	pub display_name: String,
	pub messages: String,
}

impl HasId for Channel {
	fn id(&self) -> u32 {
		self.id
	}

	fn set_id(&mut self, id: u32) {
		self.id = id;
	}
}

#[derive(Debug, Clone, WguiModel, serde::Serialize, serde::Deserialize)]
pub struct DirectMessage {
	pub id: u32,
	pub name: String,
	pub display_name: String,
	pub online: bool,
	pub messages: String,
}

impl HasId for DirectMessage {
	fn id(&self) -> u32 {
		self.id
	}

	fn set_id(&mut self, id: u32) {
		self.id = id;
	}
}

#[derive(Debug, Clone, WguiModel, serde::Serialize, serde::Deserialize)]
pub struct User {
	pub name: String,
	pub password: String,
}

#[derive(Debug, Wdb)]
pub struct PuppyDB {
	pub messages: DbTable<Message>,
	pub channels: DbTable<Channel>,
	pub direct_messages: DbTable<DirectMessage>,
	pub users: DbTable<User>,
}

impl PuppyDB {
	pub fn new() -> Self {
		let db = Db::<PuppyDB>::new();
		Self {
			messages: db.table(),
			channels: db.table(),
			direct_messages: db.table(),
			users: db.table(),
		}
	}
}
