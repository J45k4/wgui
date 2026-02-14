use log::Level;
use wgui::{HasId, Table, Wgui, WguiModel};

mod components;
mod context;

#[derive(Debug, Clone, WguiModel)]
pub struct Message {
	id: u32,
	author: String,
	body: String,
	image_url: String,
	time: String,
	channel_id: Option<u32>,
	dm_thread_key: Option<String>,
}

impl HasId for Message {
	fn id(&self) -> u32 {
		self.id
	}

	fn set_id(&mut self, id: u32) {
		self.id = id;
	}
}

impl Message {
	pub async fn save(self, db: &std::sync::Arc<PuppyDb>) -> Self {
		db.messages.save(self).await
	}
}

#[derive(Debug, Clone, WguiModel)]
pub struct Channel {
	id: u32,
	name: String,
	display_name: String,
	messages: Vec<Message>,
}

impl HasId for Channel {
	fn id(&self) -> u32 {
		self.id
	}

	fn set_id(&mut self, id: u32) {
		self.id = id;
	}
}

impl Channel {
	pub async fn save(mut self, db: &std::sync::Arc<PuppyDb>) -> Self {
		if self.display_name.is_empty() {
			self.display_name = if self.name.starts_with('#') {
				self.name.clone()
			} else {
				format!("# {}", self.name)
			};
		}
		db.channels.save(self).await
	}
}

#[derive(Debug, Clone, WguiModel)]
pub struct DirectMessage {
	id: u32,
	name: String,
	display_name: String,
	online: bool,
	messages: Vec<Message>,
}

impl HasId for DirectMessage {
	fn id(&self) -> u32 {
		self.id
	}

	fn set_id(&mut self, id: u32) {
		self.id = id;
	}
}

#[derive(Debug, Clone)]
pub struct User {
	name: String,
	password: String,
}

impl User {
	pub async fn save(self, db: &std::sync::Arc<PuppyDb>) -> Self {
		db.users.insert(self.clone()).await;
		self
	}

	pub async fn find(name: &str, db: &std::sync::Arc<PuppyDb>) -> Option<Self> {
		db.users
			.snapshot()
			.into_iter()
			.find(|user| user.name == name)
	}
}

#[derive(Debug, Clone)]
pub struct SessionState {
	pub user_name: String,
	pub login_name: String,
	pub login_password: String,
	pub auth_error: String,
	pub new_message: String,
	pub new_picture_url: String,
	pub new_channel_name: String,
	pub show_create_channel: bool,
	pub show_attach_menu: bool,
	pub show_image_modal: bool,
	pub selected_image_url: String,
	pub active_kind: String,
	pub active_id: u32,
	pub active_name: String,
}

impl SessionState {
	fn new(default_channel: Option<(u32, String)>) -> Self {
		let (active_kind, active_id, active_name) =
			if let Some((id, display_name)) = default_channel {
				("channel".to_string(), id, display_name)
			} else {
				("".to_string(), 0, "".to_string())
			};
		Self {
			user_name: String::new(),
			login_name: String::new(),
			login_password: String::new(),
			auth_error: String::new(),
			new_message: String::new(),
			new_picture_url: String::new(),
			new_channel_name: String::new(),
			show_create_channel: false,
			show_attach_menu: false,
			show_image_modal: false,
			selected_image_url: String::new(),
			active_kind,
			active_id,
			active_name,
		}
	}
}

#[derive(Debug, Clone, WguiModel)]
pub struct ChatViewState {
	user_name: String,
	login_name: String,
	login_password: String,
	auth_error: String,
	new_message: String,
	new_picture_url: String,
	new_channel_name: String,
	show_create_channel: bool,
	show_attach_menu: bool,
	show_image_modal: bool,
	selected_image_url: String,
	active_kind: String,
	active_id: u32,
	active_name: String,
	channels: Vec<Channel>,
	directs: Vec<DirectMessage>,
}

#[derive(Debug)]
pub struct PuppyDb {
	pub channels: Table<Channel>,
	pub directs: Table<DirectMessage>,
	pub messages: Table<Message>,
	pub users: Table<User>,
}

impl PuppyDb {
	fn dm_thread_key(left: &str, right: &str) -> String {
		if left <= right {
			format!("{}|{}", left, right)
		} else {
			format!("{}|{}", right, left)
		}
	}

	pub fn new() -> Self {
		let channels = vec![Channel {
			id: 1,
			name: "general".to_string(),
			display_name: "# general".to_string(),
			messages: Vec::new(),
		}];
		Self {
			channels: Table::with_ids(channels),
			directs: Table::with_ids(Vec::new()),
			messages: Table::with_ids(Vec::new()),
			users: Table::new(Vec::new()),
		}
	}

	pub fn ensure_direct_entry(&self, user_name: &str) {
		let mut directs = self.directs.snapshot();
		if directs.iter().any(|dm| dm.name == user_name) {
			return;
		}
		directs.push(DirectMessage {
			id: self.directs.next_id(),
			name: user_name.to_string(),
			display_name: format!("@ {}", user_name),
			online: true,
			messages: Vec::new(),
		});
		self.directs.replace(directs);
	}

	pub fn dm_thread_key_for_session(&self, session: &SessionState) -> Option<String> {
		if session.active_kind != "dm" {
			return None;
		}
		let directs = self.directs.snapshot();
		let other_name = directs
			.iter()
			.find(|dm| dm.id == session.active_id)
			.map(|dm| dm.name.clone())?;
		Some(Self::dm_thread_key(&session.user_name, &other_name))
	}
}

#[tokio::main]
async fn main() {
	simple_logger::init_with_level(Level::Info).unwrap();

	let db = PuppyDb::new();
	let mut wgui = Wgui::new("0.0.0.0:5545".parse().unwrap()).with_db(db);
	wgui.set_ctx_state(context::SharedContext::default());
	wgui.add_component::<components::puppychat::Puppychat>("/");
	wgui.run().await;
}
