#[cfg(feature = "hyper")]
use server::Server;
use std::collections::HashMap;
use std::future::Future;
#[cfg(feature = "hyper")]
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::RwLock;

pub mod diff;
pub mod dist;
pub mod edit_distance;
pub mod gui;
pub mod pubsub;
#[cfg(feature = "hyper")]
mod server;
pub mod ssr;
pub mod types;
mod ui_client;
pub mod ws;
pub mod wui;

pub use pubsub::PubSub;
pub use wui::runtime::WuiModel;
pub use wui_derive::{wgui_controller, WuiModel};

use crate::ui_client::UiWsWorker;

pub use dist::*;
pub use gui::*;
pub use types::*;
#[cfg(feature = "hyper")]
pub use ws::TungsteniteWs;
pub use ws::{next_client_id, WsMessage, WsStream};

pub(crate) type Sessions = Arc<RwLock<HashMap<usize, Option<String>>>>;
type BoxedController = Box<dyn crate::wui::runtime::WuiController + Send>;
type ControllerFuture = Pin<Box<dyn Future<Output = BoxedController> + Send>>;
type ControllerFactory = Arc<dyn Fn() -> ControllerFuture + Send + Sync>;

#[derive(Clone)]
pub struct WguiHandle {
	event_tx: mpsc::UnboundedSender<ClientMessage>,
	clients: Clients,
	sessions: Sessions,
}

impl WguiHandle {
	pub(crate) fn new(
		event_tx: mpsc::UnboundedSender<ClientMessage>,
		clients: Clients,
		sessions: Sessions,
	) -> Self {
		Self {
			event_tx,
			clients,
			sessions,
		}
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

		let mut sessions = self.sessions.write().await;
		sessions.insert(id, None);

		id
	}

	pub async fn handle_ws_with_session<S>(&self, ws: S, session: Option<String>) -> usize
	where
		S: WsStream + 'static,
	{
		let id = self.handle_ws(ws).await;
		let mut sessions = self.sessions.write().await;
		sessions.insert(id, session);
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

	pub async fn session_for_client(&self, client_id: usize) -> Option<String> {
		let sessions = self.sessions.read().await;
		sessions.get(&client_id).cloned().flatten()
	}

	pub async fn clear_session(&self, client_id: usize) {
		let mut sessions = self.sessions.write().await;
		sessions.remove(&client_id);
	}
}

struct ComponentRegistration {
	route_path: String,
	factory: ControllerFactory,
	controllers: HashMap<usize, BoxedController>,
}

impl ComponentRegistration {
	fn new(route_path: String, factory: ControllerFactory) -> Self {
		Self {
			route_path,
			factory,
			controllers: HashMap::new(),
		}
	}
}

pub struct Wgui {
	events_rx: mpsc::UnboundedReceiver<ClientMessage>,
	handle: WguiHandle,
	components: Vec<ComponentRegistration>,
}

impl Wgui {
	fn path_matches(route_path: &str, current_path: &str) -> bool {
		if route_path == "/" {
			return true;
		}
		if current_path == route_path {
			return true;
		}
		let prefix = format!("{}/", route_path.trim_end_matches('/'));
		current_path.starts_with(&prefix)
	}

	#[cfg(feature = "hyper")]
	pub fn new(addr: SocketAddr) -> Self {
		let (events_tx, events_rx) = mpsc::unbounded_channel();
		let clients: Clients = Arc::new(RwLock::new(HashMap::new()));
		let sessions: Sessions = Arc::new(RwLock::new(HashMap::new()));

		{
			let clients = clients.clone();
			let event_tx = events_tx.clone();
			tokio::spawn(async move {
				Server::new(addr, event_tx, clients, None).await.run().await;
			});
		}

		Self {
			events_rx,
			handle: WguiHandle::new(events_tx, clients, sessions),
			components: Vec::new(),
		}
	}

	#[cfg(feature = "hyper")]
	pub fn new_with_ssr(
		addr: SocketAddr,
		renderer: std::sync::Arc<dyn Fn() -> Item + Send + Sync>,
	) -> Self {
		let (events_tx, events_rx) = mpsc::unbounded_channel();
		let clients: Clients = Arc::new(RwLock::new(HashMap::new()));
		let sessions: Sessions = Arc::new(RwLock::new(HashMap::new()));

		{
			let clients = clients.clone();
			let event_tx = events_tx.clone();
			let ssr = Some(renderer);
			tokio::spawn(async move {
				Server::new(addr, event_tx, clients, ssr).await.run().await;
			});
		}

		Self {
			events_rx,
			handle: WguiHandle::new(events_tx, clients, sessions),
			components: Vec::new(),
		}
	}

	pub fn new_without_server() -> Self {
		let (events_tx, events_rx) = mpsc::unbounded_channel();
		let clients: Clients = Arc::new(RwLock::new(HashMap::new()));
		let sessions: Sessions = Arc::new(RwLock::new(HashMap::new()));

		Self {
			events_rx,
			handle: WguiHandle::new(events_tx, clients, sessions),
			components: Vec::new(),
		}
	}

	pub fn handle(&self) -> WguiHandle {
		self.handle.clone()
	}

	pub async fn next(&mut self) -> Option<ClientMessage> {
		self.events_rx.recv().await
	}

	pub async fn render(&self, client_id: usize, item: Item) {
		self.handle.render(client_id, item).await
	}

	pub async fn set_title(&self, client_id: usize, title: &str) {
		self.handle.set_title(client_id, title).await
	}

	pub async fn session_for_client(&self, client_id: usize) -> Option<String> {
		self.handle.session_for_client(client_id).await
	}

	pub async fn clear_session(&self, client_id: usize) {
		self.handle.clear_session(client_id).await
	}

	pub fn add_component<C, F, Fut>(&mut self, path: &str, controller: F)
	where
		C: crate::wui::runtime::WuiController + Send + 'static,
		F: Fn() -> Fut + Send + Sync + 'static,
		Fut: Future<Output = C> + Send + 'static,
	{
		let factory: ControllerFactory = Arc::new(move || {
			let fut = controller();
			Box::pin(async move { Box::new(fut.await) as BoxedController })
		});
		self.components
			.push(ComponentRegistration::new(path.to_string(), factory));
	}

	pub async fn run(&mut self) {
		let handle = self.handle();
		let mut paths: HashMap<usize, String> = HashMap::new();

		while let Some(message) = self.next().await {
			let client_id = message.client_id;
			match &message.event {
				ClientEvent::Connected { id: _ } => {
					let current_path = paths
						.get(&client_id)
						.cloned()
						.unwrap_or_else(|| "/".to_string());

					for component in self.components.iter_mut() {
						if !Self::path_matches(&component.route_path, &current_path) {
							continue;
						}
						let controller = (component.factory)().await;
						let item = controller.render_with_path(&current_path);
						let title = controller.route_title(&current_path);
						if let Some(title) = title {
							handle.set_title(client_id, &title).await;
						}
						handle.render(client_id, item).await;
						component.controllers.insert(client_id, controller);
					}
				}
				ClientEvent::Disconnected { id: _ } => {
					for component in self.components.iter_mut() {
						component.controllers.remove(&client_id);
					}
					paths.remove(&client_id);
					handle.clear_session(client_id).await;
				}
				ClientEvent::PathChanged(change) => {
					paths.insert(client_id, change.path.clone());

					for component in self.components.iter_mut() {
						if !Self::path_matches(&component.route_path, &change.path) {
							component.controllers.remove(&client_id);
							continue;
						}

						if let Some(controller) = component.controllers.get_mut(&client_id) {
							let item = controller.render_with_path(&change.path);
							let title = controller.route_title(&change.path);
							if let Some(title) = title {
								handle.set_title(client_id, &title).await;
							}
							handle.render(client_id, item).await;
						} else {
							let controller = (component.factory)().await;
							let item = controller.render_with_path(&change.path);
							let title = controller.route_title(&change.path);
							if let Some(title) = title {
								handle.set_title(client_id, &title).await;
							}
							handle.render(client_id, item).await;
							component.controllers.insert(client_id, controller);
						}
					}
				}
				ClientEvent::Input(_) => {}
				_ => {
					for component in self.components.iter_mut() {
						let handled = component
							.controllers
							.get_mut(&client_id)
							.map(|controller| controller.handle(&message.event))
							.unwrap_or(false);

						if handled {
							let mut updates: Vec<(usize, Item, Option<String>)> = Vec::new();
							for (mounted_client_id, mounted_controller) in
								component.controllers.iter_mut()
							{
								let current_path = paths
									.get(mounted_client_id)
									.cloned()
									.unwrap_or_else(|| "/".to_string());
								let item = mounted_controller.render_with_path(&current_path);
								let title = mounted_controller.route_title(&current_path);
								updates.push((*mounted_client_id, item, title));
							}

							for (mounted_client_id, item, title) in updates {
								if let Some(title) = title {
									handle.set_title(mounted_client_id, &title).await;
								}
								handle.render(mounted_client_id, item).await;
							}
						}
					}
				}
			}
		}
	}
}
