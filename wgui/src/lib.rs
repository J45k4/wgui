#[cfg(feature = "hyper")]
#[cfg(feature = "hyper")]
use server::Server;
use std::collections::HashMap;
#[cfg(feature = "hyper")]
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::RwLock;

#[cfg(feature = "axum")]
pub mod axum;
pub mod diff;
pub mod dist;
pub mod edit_distance;
pub mod gui;
#[cfg(feature = "hyper")]
mod server;
pub mod types;
mod ui_client;
pub mod ws;

use crate::ui_client::UiWsWorker;

#[cfg(feature = "axum")]
use crate::axum::router as axum_router;
#[cfg(feature = "axum")]
use ::axum::Router;

pub use dist::*;
pub use gui::*;
pub use types::*;
#[cfg(feature = "hyper")]
pub use ws::TungsteniteWs;
pub use ws::{next_client_id, WsMessage, WsStream};

#[derive(Clone)]
pub struct WguiHandle {
	event_tx: mpsc::UnboundedSender<ClientEvent>,
	clients: Clients,
}

impl WguiHandle {
	pub(crate) fn new(event_tx: mpsc::UnboundedSender<ClientEvent>, clients: Clients) -> Self {
		Self { event_tx, clients }
	}

	pub async fn handle_ws<S>(&self, ws: S) -> usize
	where
		S: WsStream + 'static,
	{
		let id = next_client_id();
		let event_tx = self.event_tx.clone();
		let clients = self.clients.clone();
		log::info!("websocket worker created {}", id);
		tokio::spawn(async move {
			let worker = UiWsWorker::new(id, ws, event_tx, clients).await;
			worker.run().await;
		});

		id
	}

	pub async fn render(&self, client_id: usize, item: Item) {
		log::debug!("render {:?}", item);
		let clients = self.clients.read().await;
		let sender = match clients.get(&client_id) {
			Some(sender) => sender,
			None => {
				println!("client not found");
				return;
			}
		};
		sender.send(Command::Render(item)).unwrap();
	}

	pub async fn set_title(&self, client_id: usize, title: &str) {
		let clients = self.clients.read().await;
		let sender = match clients.get(&client_id) {
			Some(sender) => sender,
			None => {
				println!("client not found");
				return;
			}
		};
		sender.send(Command::SetTitle(title.to_string())).unwrap();
	}
}

pub struct Wgui {
	events_rx: mpsc::UnboundedReceiver<ClientEvent>,
	handle: WguiHandle,
}

impl Wgui {
	#[cfg(feature = "hyper")]
	pub fn new(addr: SocketAddr) -> Self {
		let (events_tx, events_rx) = mpsc::unbounded_channel();
		let clients: Clients = Arc::new(RwLock::new(HashMap::new()));

		{
			let clients = clients.clone();
			let event_tx = events_tx.clone();
			tokio::spawn(async move {
				Server::new(addr, event_tx, clients).await.run().await;
			});
		}

		Self {
			events_rx,
			handle: WguiHandle::new(events_tx, clients),
		}
	}

	pub fn new_without_server() -> Self {
		let (events_tx, events_rx) = mpsc::unbounded_channel();
		let clients: Clients = Arc::new(RwLock::new(HashMap::new()));

		Self {
			events_rx,
			handle: WguiHandle::new(events_tx, clients),
		}
	}

	pub fn handle(&self) -> WguiHandle {
		self.handle.clone()
	}

	#[cfg(feature = "axum")]
	pub fn router(&self) -> Router {
		axum_router(self.handle.clone())
	}

	pub async fn next(&mut self) -> Option<ClientEvent> {
		self.events_rx.recv().await
	}

	pub async fn render(&self, client_id: usize, item: Item) {
		self.handle.render(client_id, item).await
	}

	pub async fn set_title(&self, client_id: usize, title: &str) {
		self.handle.set_title(client_id, title).await
	}
}
