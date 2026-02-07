use log::Level;
use std::collections::HashMap;
use std::sync::Arc;
use wgui::Wgui;
use wgui::WuiModel;
use wgui::wui::runtime::Ctx;

mod components;
mod context;
mod generated;

#[derive(Debug, Clone, WuiModel)]
pub struct Message {
	id: u32,
	author: String,
	body: String,
	time: String,
}

#[derive(Debug, Clone, WuiModel)]
pub struct Channel {
	id: u32,
	name: String,
	display_name: String,
	messages: Vec<Message>,
}

#[derive(Debug, Clone, WuiModel)]
pub struct DirectMessage {
	id: u32,
	name: String,
	display_name: String,
	online: bool,
	messages: Vec<Message>,
}

#[derive(Debug, Clone)]
pub struct ChatState {
	channels: Vec<Channel>,
	directs: Vec<DirectMessage>,
	dm_threads: HashMap<String, Vec<Message>>,
}

#[derive(Debug, Clone)]
pub struct SessionState {
	pub user_name: String,
	pub login_name: String,
	pub new_message: String,
	pub new_channel_name: String,
	pub show_create_channel: bool,
	pub active_kind: String,
	pub active_id: u32,
	pub active_name: String,
}

impl SessionState {
	fn new(shared: &ChatState) -> Self {
		let (active_kind, active_id, active_name) = if let Some(first) = shared.channels.first() {
			("channel".to_string(), first.id, first.display_name.clone())
		} else {
			("".to_string(), 0, "".to_string())
		};
		Self {
			user_name: String::new(),
			login_name: String::new(),
			new_message: String::new(),
			new_channel_name: String::new(),
			show_create_channel: false,
			active_kind,
			active_id,
			active_name,
		}
	}
}

#[derive(Debug, Clone, WuiModel)]
pub struct ChatViewState {
	user_name: String,
	login_name: String,
	new_message: String,
	new_channel_name: String,
	show_create_channel: bool,
	active_kind: String,
	active_id: u32,
	active_name: String,
	channels: Vec<Channel>,
	directs: Vec<DirectMessage>,
}

impl Default for ChatState {
	fn default() -> Self {
		let channels = vec![Channel {
			id: 1,
			name: "general".to_string(),
			display_name: "# general".to_string(),
			messages: Vec::new(),
		}];
		let directs = Vec::new();
		Self {
			channels,
			directs,
			dm_threads: HashMap::new(),
		}
	}
}

#[tokio::main]
async fn main() {
	simple_logger::init_with_level(Level::Info).unwrap();

	let ctx = Arc::new(Ctx::new(context::SharedContext::default()));
	let mut wgui = Wgui::new("0.0.0.0:5545".parse().unwrap());
	wgui.set_ctx(ctx.clone());
	wgui.add_component::<components::puppychat::Puppychat>("/");
	wgui.run().await;
}
