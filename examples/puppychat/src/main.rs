use log::Level;
use std::collections::HashMap;
use std::sync::Arc;
use wgui::WuiModel;
use wgui::wui::runtime::{Component, Ctx, WuiController};
use wgui::Wgui;

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
	let ssr_ctx = ctx.clone();
	let mut wgui = Wgui::new_with_ssr(
		"0.0.0.0:5545".parse().unwrap(),
		Arc::new(move || {
			let controller = tokio::task::block_in_place(|| {
				tokio::runtime::Handle::current()
					.block_on(components::puppychat::Puppychat::mount(ssr_ctx.clone()))
			});
			WuiController::render(&controller)
		}),
	);
	let mut controllers: HashMap<usize, components::puppychat::Puppychat> = HashMap::new();
	let mut paths: HashMap<usize, String> = HashMap::new();
	let mut pubsub_rx = ctx.pubsub().subscribe("rerender");

	while let Some(message) = wgui.next().await {
		let client_id = message.client_id;
		match message.event {
			wgui::ClientEvent::Connected { id: _ } => {
				let path = paths
					.get(&client_id)
					.cloned()
					.unwrap_or_else(|| "/".to_string());
				let controller = components::puppychat::Puppychat::mount(ctx.clone()).await;
				let item = WuiController::render_with_path(&controller, &path);
				if let Some(title) = WuiController::route_title(&controller, &path) {
					wgui.set_title(client_id, &title).await;
				}
				wgui.render(client_id, item).await;
				controllers.insert(client_id, controller);
			}
			wgui::ClientEvent::Disconnected { id: _ } => {
				if let Some(controller) = controllers.remove(&client_id) {
					controller.unmount(ctx.clone());
				}
				paths.remove(&client_id);
				wgui.clear_session(client_id).await;
			}
			wgui::ClientEvent::PathChanged(change) => {
				paths.insert(client_id, change.path.clone());
				if let Some(controller) = controllers.get_mut(&client_id) {
					let item = WuiController::render_with_path(controller, &change.path);
					if let Some(title) = WuiController::route_title(controller, &change.path) {
						wgui.set_title(client_id, &title).await;
					}
					wgui.render(client_id, item).await;
				}
			}
			wgui::ClientEvent::Input(_) => {}
			_ => {
				let path = paths
					.get(&client_id)
					.cloned()
					.unwrap_or_else(|| "/".to_string());
				if let Some(controller) = controllers.get_mut(&client_id) {
					if WuiController::handle(controller, &message.event) {
						let item = WuiController::render_with_path(controller, &path);
						if let Some(title) = WuiController::route_title(controller, &path) {
							wgui.set_title(client_id, &title).await;
						}
						wgui.render(client_id, item).await;
					}
				}
			}
		}

		while pubsub_rx.try_recv().is_ok() {
			for (client_id, controller) in controllers.iter_mut() {
				let path = paths
					.get(client_id)
					.cloned()
					.unwrap_or_else(|| "/".to_string());
				let item = WuiController::render_with_path(controller, &path);
				if let Some(title) = WuiController::route_title(controller, &path) {
					wgui.set_title(*client_id, &title).await;
				}
				wgui.render(*client_id, item).await;
			}
		}
	}
}
