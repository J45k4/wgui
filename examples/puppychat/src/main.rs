use log::Level;
use wgui::{Wgui, WguiModel};

mod components;
mod context;
mod db;

pub use db::{Channel, DirectMessage, Message, PuppyDB as PuppyDb, User};

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
pub struct ChannelView {
	id: u32,
	name: String,
	display_name: String,
	messages: Vec<Message>,
}

#[derive(Debug, Clone, WguiModel)]
pub struct DirectMessageView {
	id: u32,
	name: String,
	display_name: String,
	online: bool,
	messages: Vec<Message>,
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
	channels: Vec<ChannelView>,
	directs: Vec<DirectMessageView>,
}

pub(crate) fn puppy_db_with_defaults() -> PuppyDb {
	let db = PuppyDb::new();
	if db.channels.snapshot().is_empty() {
		db.channels.replace(vec![Channel {
			id: 1,
			name: "general".to_string(),
			display_name: "# general".to_string(),
			messages: "[]".to_string(),
		}]);
	}
	db
}

fn ensure_db_url_from_local_env() {
	#[cfg(feature = "sqlite")]
	let project_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	#[cfg(feature = "sqlite")]
	wgui::configure_sqlite_env_for_project(&project_dir);
}

#[tokio::main]
async fn main() {
	simple_logger::init_with_level(Level::Info).unwrap();
	ensure_db_url_from_local_env();

	let db = puppy_db_with_defaults();
	let mut wgui = Wgui::new("0.0.0.0:5545".parse().unwrap()).with_db(db);
	wgui.set_ctx_state(context::SharedContext::default());
	wgui.add_component::<components::puppychat::Puppychat>("/");
	wgui.run().await;
}
