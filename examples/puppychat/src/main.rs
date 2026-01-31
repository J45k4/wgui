use ::axum::Router;
use log::Level;
use std::net::SocketAddr;
use std::sync::Arc;
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

#[derive(Debug, Clone, WuiModel)]
pub struct ChatState {
	new_message: String,
	active_kind: String,
	active_id: u32,
	active_name: String,
	channels: Vec<Channel>,
	directs: Vec<DirectMessage>,
}

impl Default for ChatState {
	fn default() -> Self {
		let channels = vec![
			Channel {
				id: 1,
				name: "general".to_string(),
				display_name: "# general".to_string(),
				messages: vec![
					Message {
						id: 1,
						author: "Nova".to_string(),
						body: "Welcome to PuppyChat!".to_string(),
						time: "9:00".to_string(),
					},
					Message {
						id: 2,
						author: "You".to_string(),
						body: "Let’s build the Slack clone UI.".to_string(),
						time: "9:02".to_string(),
					},
				],
			},
			Channel {
				id: 2,
				name: "design".to_string(),
				display_name: "# design".to_string(),
				messages: vec![Message {
					id: 3,
					author: "Luna".to_string(),
					body: "Left sidebar needs some structure.".to_string(),
					time: "9:10".to_string(),
				}],
			},
			Channel {
				id: 3,
				name: "ship-it".to_string(),
				display_name: "# ship-it".to_string(),
				messages: vec![Message {
					id: 4,
					author: "Piper".to_string(),
					body: "We ship today.".to_string(),
					time: "9:18".to_string(),
				}],
			},
		];
		let directs = vec![
			DirectMessage {
				id: 10,
				name: "Avery".to_string(),
				display_name: "@ Avery".to_string(),
				online: true,
				messages: vec![Message {
					id: 5,
					author: "Avery".to_string(),
					body: "Do we have the layout ready?".to_string(),
					time: "9:20".to_string(),
				}],
			},
			DirectMessage {
				id: 11,
				name: "Milo".to_string(),
				display_name: "@ Milo".to_string(),
				online: false,
				messages: vec![Message {
					id: 6,
					author: "Milo".to_string(),
					body: "Ping me when it’s live.".to_string(),
					time: "9:22".to_string(),
				}],
			},
		];
		Self {
			new_message: String::new(),
			active_kind: "channel".to_string(),
			active_id: channels[0].id,
			active_name: channels[0].display_name.clone(),
			channels,
			directs,
		}
	}
}

#[tokio::main]
async fn main() {
	simple_logger::init_with_level(Level::Info).unwrap();

	let ctx = Arc::new(Ctx::new(context::SharedContext::default()));
	let router = generated::routes::router(ctx);
	let app = Router::new().merge(router);

	let addr: SocketAddr = "0.0.0.0:5545".parse().unwrap();
	log::info!("listening on http://localhost:5545");
	let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
	::axum::serve(listener, app).await.unwrap();
}
