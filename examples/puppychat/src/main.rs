use log::Level;
use wgui::{Wgui, WguiModel};

mod context;
mod db;
mod routes;

pub use db::{Channel, DirectMessage, Message, PuppyDB as PuppyDb, Session, User};

const PUPPYCHAT_CSS: &str = r#"
.puppychat-shell {
  height: 100dvh;
  min-height: 0;
  overflow: hidden;
}

.puppychat-sidebar,
.puppychat-main,
.puppychat-messages {
  min-height: 0;
}

@media (max-width: 640px) {
  .puppychat-shell {
    flex-direction: column !important;
    flex-wrap: nowrap !important;
    width: 100vw !important;
    box-sizing: border-box !important;
  }

  .puppychat-sidebar {
    box-sizing: border-box !important;
    width: 100% !important;
    min-width: 0 !important;
    max-width: none !important;
    max-height: 140px;
    flex: 0 0 auto !important;
    resize: none !important;
  }

  .puppychat-sidebar > .wgui-resize-handle {
    display: none !important;
  }

  .puppychat-list-page .puppychat-sidebar {
    max-height: none;
    flex: 1 1 auto !important;
  }

  .puppychat-list-page .puppychat-main,
  .puppychat-conversation-page .puppychat-sidebar {
    display: none !important;
  }

  .puppychat-main {
    box-sizing: border-box !important;
    width: 100% !important;
    min-width: 0 !important;
    flex: 1 1 0 !important;
    min-height: 0 !important;
    overflow: hidden !important;
  }

  .puppychat-header,
  .puppychat-call-controls,
  .puppychat-composer {
    flex-wrap: wrap !important;
  }

  .puppychat-composer input {
    min-width: 0;
    max-width: 100%;
  }

  .puppychat-back {
    display: inline !important;
  }
}

.puppychat-back {
  display: none;
}

.puppychat-create-channel-modal {
  position: relative;
  min-height: 200px;
  box-sizing: border-box;
}

.puppychat-create-submit-row {
  position: absolute;
  right: 16px;
  bottom: 16px;
  width: calc(100% - 32px);
  justify-content: flex-end;
}

.puppychat-create-cancel-row {
  position: absolute;
  left: 16px;
  bottom: 16px;
}

.puppychat-upload-modal {
  position: relative;
  min-height: 200px;
  box-sizing: border-box;
}

.puppychat-upload-submit-row {
  position: absolute;
  right: 16px;
  bottom: 16px;
}

.puppychat-upload-cancel-row {
  position: absolute;
  left: 16px;
  bottom: 16px;
  transform: translateY(20px);
}
"#;

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
	pub call_active: bool,
	pub call_with_video: bool,
	pub push_status: String,
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
			call_active: false,
			call_with_video: true,
			push_status: String::new(),
		}
	}
}

#[derive(Debug, Clone, WguiModel)]
pub struct ChannelView {
	id: u32,
	name: String,
	display_name: String,
	href: String,
	messages: Vec<Message>,
}

#[derive(Debug, Clone, WguiModel)]
pub struct DirectMessageView {
	id: u32,
	name: String,
	display_name: String,
	href: String,
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
	call_active: bool,
	call_with_video: bool,
	call_room: String,
	push_status: String,
	shell_class: String,
	message_partial_addr: String,
	channels: Vec<ChannelView>,
	directs: Vec<DirectMessageView>,
}

impl ChatViewState {
	pub(crate) fn with_message_target(mut self, kind: String, id: u32) -> Self {
		self.active_kind = kind.clone();
		self.active_id = id;
		self.message_partial_addr = format!("/chat/messages/{kind}/{id}");
		self
	}
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
	let address = std::env::var("PUPPYCHAT_ADDR")
		.unwrap_or_else(|_| "0.0.0.0:5545".to_string())
		.parse()
		.expect("PUPPYCHAT_ADDR must be a valid socket address");
	let mut wgui = Wgui::new(address).with_db(db);
	wgui.set_css(PUPPYCHAT_CSS);
	wgui.set_ctx_state(context::SharedContext::default());
	wgui.add_route(routes::page_login_route);
	wgui.add_route(routes::login_route);
	wgui.add_route(routes::logout_route);
	wgui.add_route(routes::page_register_route);
	wgui.add_route(routes::register_route);
	wgui.add_route(routes::send_message_route);
	wgui.add_route(routes::open_create_channel_route);
	wgui.add_route(routes::close_create_channel_route);
	wgui.add_route(routes::create_channel_route);
	wgui.add_route(routes::start_audio_call_route);
	wgui.add_route(routes::start_video_call_route);
	wgui.add_route(routes::end_call_route);
	wgui.add_route(routes::open_attach_menu_route);
	wgui.add_route(routes::close_attach_menu_route);
	wgui.add_route(routes::send_picture_route);
	wgui.add_route(routes::open_message_image_route);
	wgui.add_route(routes::close_image_modal_route);
	wgui.add_route(routes::page_chat_route);
	wgui.add_route(routes::page_channel_route);
	wgui.add_route(routes::page_direct_route);
	wgui.add_partial(routes::message_list_partial);
	wgui.run().await;
}
