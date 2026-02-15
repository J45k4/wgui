#[cfg(feature = "hyper")]
use server::Server;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::future::Future;
#[cfg(feature = "hyper")]
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::RwLock;

pub mod db_table;
pub mod diff;
pub mod dist;
pub mod edit_distance;
pub mod gui;
pub mod pubsub;
pub mod schema_diff;
#[cfg(feature = "hyper")]
mod server;
#[cfg(feature = "sqlite")]
pub mod sqlite;
pub mod ssr;
pub mod table;
pub mod types;
mod ui_client;
pub mod wdb;
pub mod ws;
pub mod wui;

pub use pubsub::PubSub;
pub use wui::runtime::{WdbModel, WdbSchema, WguiModel};
pub use wui_derive::{wgui_controller, Wdb, WguiModel};

use crate::ui_client::UiWsWorker;

pub use db_table::{Db, DbTable};
pub use dist::*;
pub use gui::*;
#[cfg(feature = "sqlite")]
pub use sqlite::{
	default_db_path_for_schema, schema_diff_sql, schema_diff_sql_from_schema_file,
	write_schema_migration, write_schema_migration_from_schema_file, SQLLiteDB, SQLiteDB,
	SchemaMigrations, SqliteDb, SqliteTable,
};
pub use table::{HasId, Table};
pub use types::*;
#[cfg(feature = "hyper")]
pub use ws::TungsteniteWs;
pub use ws::{next_client_id, WsMessage, WsStream};

pub(crate) type Sessions = Arc<RwLock<HashMap<usize, Option<String>>>>;
type BoxedController = Box<dyn crate::wui::runtime::WuiController + Send>;
type ControllerFuture = Pin<Box<dyn Future<Output = BoxedController> + Send>>;
type ControllerFactory = Arc<dyn Fn() -> ControllerFuture + Send + Sync>;
type SsrRenderer = Arc<dyn Fn(&str) -> Option<Item> + Send + Sync>;
type SsrComponentFactories = Arc<std::sync::RwLock<Vec<(String, ControllerFactory)>>>;

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

	pub async fn push_state(&self, client_id: usize, url: &str) {
		let clients = self.clients.read().await;
		let sender = match clients.get(&client_id) {
			Some(sender) => sender,
			None => {
				println!("client not found");
				return;
			}
		};
		sender.send(Command::PushState(url.to_string())).unwrap();
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

struct ContextAwareController<C, T, DB>
where
	C: crate::wui::runtime::WuiController + Send + 'static,
	T: Send + Sync + 'static,
	DB: Send + Sync + 'static,
{
	inner: C,
	ctx: Arc<crate::wui::runtime::Ctx<T, DB>>,
}

#[crate::wui::runtime::async_trait]
impl<C, T, DB> crate::wui::runtime::WuiController for ContextAwareController<C, T, DB>
where
	C: crate::wui::runtime::WuiController + Send + 'static,
	T: Send + Sync + 'static,
	DB: Send + Sync + 'static,
{
	fn render(&self) -> Item {
		self.inner.render()
	}

	fn render_with_path(&self, path: &str) -> Item {
		self.inner.render_with_path(path)
	}

	fn route_title(&self, path: &str) -> Option<String> {
		self.inner.route_title(path)
	}

	fn set_runtime_context(&mut self, client_id: Option<usize>, session: Option<String>) {
		self.ctx.set_current_client(client_id);
		self.ctx.set_current_session(session.clone());
		self.inner.set_runtime_context(client_id, session);
	}

	async fn handle(&mut self, event: &crate::types::ClientEvent) -> bool {
		self.inner.handle(event).await
	}
}

pub struct Wgui<DB = ()> {
	events_rx: mpsc::UnboundedReceiver<ClientMessage>,
	handle: WguiHandle,
	components: Vec<ComponentRegistration>,
	ssr_components: SsrComponentFactories,
	db: Arc<DB>,
	contexts: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl Wgui<()> {
	#[cfg(feature = "hyper")]
	pub fn new(addr: SocketAddr) -> Self {
		let (events_tx, events_rx) = mpsc::unbounded_channel();
		let clients: Clients = Arc::new(RwLock::new(HashMap::new()));
		let sessions: Sessions = Arc::new(RwLock::new(HashMap::new()));
		let ssr_components: SsrComponentFactories = Arc::new(std::sync::RwLock::new(Vec::new()));

		{
			let clients = clients.clone();
			let event_tx = events_tx.clone();
			let sessions = sessions.clone();
			let ssr_components = ssr_components.clone();
			let ssr: Option<SsrRenderer> = Some(Arc::new(move |path: &str| {
				let factories = ssr_components.read().unwrap();
				for (route_path, factory) in factories.iter() {
					if !path_matches(route_path, path) {
						continue;
					}
					let controller = tokio::task::block_in_place(|| {
						tokio::runtime::Handle::current().block_on((factory)())
					});
					return Some(controller.render_with_path(path));
				}
				None
			}));
			tokio::spawn(async move {
				Server::new(addr, event_tx, clients, sessions, ssr)
					.await
					.run()
					.await;
			});
		}

		Self {
			events_rx,
			handle: WguiHandle::new(events_tx, clients, sessions),
			components: Vec::new(),
			ssr_components,
			db: Arc::new(()),
			contexts: HashMap::new(),
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
		let ssr_components: SsrComponentFactories = Arc::new(std::sync::RwLock::new(Vec::new()));

		{
			let clients = clients.clone();
			let event_tx = events_tx.clone();
			let sessions = sessions.clone();
			let ssr: Option<SsrRenderer> = Some(Arc::new(move |_path: &str| Some((renderer)())));
			tokio::spawn(async move {
				Server::new(addr, event_tx, clients, sessions, ssr)
					.await
					.run()
					.await;
			});
		}

		Self {
			events_rx,
			handle: WguiHandle::new(events_tx, clients, sessions),
			components: Vec::new(),
			ssr_components,
			db: Arc::new(()),
			contexts: HashMap::new(),
		}
	}

	pub fn new_without_server() -> Self {
		let (events_tx, events_rx) = mpsc::unbounded_channel();
		let clients: Clients = Arc::new(RwLock::new(HashMap::new()));
		let sessions: Sessions = Arc::new(RwLock::new(HashMap::new()));
		let ssr_components: SsrComponentFactories = Arc::new(std::sync::RwLock::new(Vec::new()));

		Self {
			events_rx,
			handle: WguiHandle::new(events_tx, clients, sessions),
			components: Vec::new(),
			ssr_components,
			db: Arc::new(()),
			contexts: HashMap::new(),
		}
	}
}

impl<DB> Wgui<DB>
where
	DB: Send + Sync + 'static,
{
	pub fn with_db<NewDB>(self, db: NewDB) -> Wgui<NewDB>
	where
		NewDB: Send + Sync + 'static,
	{
		Wgui {
			events_rx: self.events_rx,
			handle: self.handle,
			components: self.components,
			ssr_components: self.ssr_components,
			db: Arc::new(db),
			contexts: HashMap::new(),
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

	pub fn set_ctx<T>(&mut self, ctx: Arc<crate::wui::runtime::Ctx<T, DB>>)
	where
		T: Send + Sync + 'static,
	{
		let mut command_rx = ctx.take_command_rx();
		let handle = self.handle();
		tokio::spawn(async move {
			while let Some(command) = command_rx.recv().await {
				match command {
					crate::wui::runtime::RuntimeCommand::SetTitle { client_id, title } => {
						handle.set_title(client_id, &title).await;
					}
					crate::wui::runtime::RuntimeCommand::PushState { client_id, url } => {
						handle.push_state(client_id, &url).await;
					}
				}
			}
		});

		let erased: Arc<dyn Any + Send + Sync> = ctx;
		self.contexts.insert(TypeId::of::<T>(), erased);
	}

	pub fn set_ctx_state<T>(&mut self, state: T)
	where
		T: Send + Sync + 'static,
	{
		let ctx = Arc::new(crate::wui::runtime::Ctx::new_with_db(
			state,
			self.db.clone(),
		));
		self.set_ctx(ctx);
	}

	pub fn add_component<C>(&mut self, path: &str)
	where
		C: crate::wui::runtime::Component<Db = DB>
			+ crate::wui::runtime::WuiController
			+ Send
			+ 'static,
		<C as crate::wui::runtime::Component>::Context: Send + Sync + 'static,
	{
		let Some(ctx_any) = self
			.contexts
			.get(&TypeId::of::<<C as crate::wui::runtime::Component>::Context>())
			.cloned()
		else {
			panic!("missing context for component; call wgui.set_ctx(...) first");
		};
		let Ok(ctx) = ctx_any
			.downcast::<crate::wui::runtime::Ctx<<C as crate::wui::runtime::Component>::Context, DB>>(
			)
		else {
			panic!("invalid context type for component");
		};

		self.add_component_with(path, move || {
			let ctx = ctx.clone();
			async move {
				ContextAwareController {
					inner: C::mount(ctx.clone()).await,
					ctx,
				}
			}
		});
	}

	pub fn add_component_with<C, F, Fut>(&mut self, path: &str, controller: F)
	where
		C: crate::wui::runtime::WuiController + Send + 'static,
		F: Fn() -> Fut + Send + Sync + 'static,
		Fut: Future<Output = C> + Send + 'static,
	{
		let factory: ControllerFactory = Arc::new(move || {
			let fut = controller();
			Box::pin(async move { Box::new(fut.await) as BoxedController })
		});
		self.ssr_components
			.write()
			.unwrap()
			.push((path.to_string(), factory.clone()));
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
					let session = handle.session_for_client(client_id).await;

					for component in self.components.iter_mut() {
						if !path_matches(&component.route_path, &current_path) {
							continue;
						}
						let mut controller = (component.factory)().await;
						controller.set_runtime_context(Some(client_id), session.clone());
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
					let session = handle.session_for_client(client_id).await;

					for component in self.components.iter_mut() {
						if !path_matches(&component.route_path, &change.path) {
							component.controllers.remove(&client_id);
							continue;
						}

						if let Some(controller) = component.controllers.get_mut(&client_id) {
							controller.set_runtime_context(Some(client_id), session.clone());
							let item = controller.render_with_path(&change.path);
							let title = controller.route_title(&change.path);
							if let Some(title) = title {
								handle.set_title(client_id, &title).await;
							}
							handle.render(client_id, item).await;
						} else {
							let mut controller = (component.factory)().await;
							controller.set_runtime_context(Some(client_id), session.clone());
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
					let session = handle.session_for_client(client_id).await;
					for component in self.components.iter_mut() {
						let handled =
							if let Some(controller) = component.controllers.get_mut(&client_id) {
								controller.set_runtime_context(Some(client_id), session.clone());
								controller.handle(&message.event).await
							} else {
								false
							};

						if handled {
							let mut updates: Vec<(usize, Item, Option<String>)> = Vec::new();
							for (mounted_client_id, mounted_controller) in
								component.controllers.iter_mut()
							{
								let mounted_session =
									handle.session_for_client(*mounted_client_id).await;
								mounted_controller
									.set_runtime_context(Some(*mounted_client_id), mounted_session);
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
