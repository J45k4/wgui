#[cfg(feature = "hyper")]
use server::Server;
use std::any::{Any, TypeId};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::future::Future;
#[cfg(feature = "hyper")]
use std::net::SocketAddr;
#[cfg(feature = "hyper")]
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

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
pub use wui::route_handler::{
	DynRouteHandler, FromParam, HttpMethod, ParamError, PathParams, Redirect, RouteFormData,
	RouteFuture, RouteHandler, RouteResult, RuntimeContext, View,
};
pub use wui::runtime::{WdbModel, WdbSchema, WguiModel};
pub use wui_derive::{partial, route, view, wgui_controller, Wdb, WguiModel};

use crate::ui_client::UiWsWorker;
use crate::wui::routing::{best_route_index, RoutePattern};
use crate::wui::runtime::{MountResult, RouteContext};

pub use db_table::{Db, DbTable};
pub use dist::*;
pub use gui::*;
pub use serde_json;
#[cfg(feature = "hyper")]
pub use server::{
	FormData, FromHttpRequest, HttpCtx, HttpHandler, HttpRequest, HttpResponse, HttpRouteSpec,
	Json, StaticAsset,
};
#[cfg(feature = "sqlite")]
pub use sqlite::{
	apply_sqlite_migrations, configure_sqlite_env_for_project, default_db_path_for_schema,
	push_schema_from_schema_file, schema_diff_sql, schema_diff_sql_from_schema_file,
	write_schema_migration, write_schema_migration_from_schema_file, SQLLiteDB, SQLiteDB,
	SchemaMigrations, SchemaPushReport, SqliteDb, SqliteTable,
};
pub use table::{HasId, Table};
pub use types::*;
#[cfg(feature = "hyper")]
pub use ws::TungsteniteWs;
pub use ws::{next_client_id, WsMessage, WsStream};

pub(crate) type Sessions = Arc<RwLock<HashMap<usize, Option<String>>>>;
pub(crate) type SsrHydrationRoots = Arc<RwLock<HashMap<String, SsrHydrationRoot>>>;
type BoxedController = Box<dyn crate::wui::runtime::WuiController + Send>;
pub(crate) type SharedRouteHandler = Arc<dyn crate::wui::route_handler::DynRouteHandler>;
pub(crate) type SharedContexts =
	Arc<std::sync::RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>>;
type SharedRoutes = Arc<std::sync::RwLock<Vec<RouteEntry>>>;

struct RouteEntry {
	pattern: RoutePattern,
	handler: SharedRouteHandler,
	state_type_id: TypeId,
}

/// Result of matching a path against registered `#[route]` handlers.
struct RouteMatchResult {
	index: usize,
	params: crate::wui::route_handler::PathParams,
}

struct ClientSession {
	current_route: RouteContext,
	page_tree: Item,
	partials: HashMap<String, PartialCache>,
}

struct PartialCache {
	params: PathParams,
	tree: Item,
	last_acked_version: u64,
}
type ControllerFuture = Pin<Box<dyn Future<Output = BoxedController> + Send>>;
type ControllerFactory = Arc<dyn Fn() -> ControllerFuture + Send + Sync>;
type ControllerProcessFuture = Pin<Box<dyn Future<Output = ()> + Send>>;
type ControllerProcessFactory =
	Arc<dyn Fn(crate::wui::runtime::ControllerProcessCtx) -> ControllerProcessFuture + Send + Sync>;
type PageControllerFuture = Pin<Box<dyn Future<Output = PageMount> + Send>>;
type PageControllerFactory =
	Arc<dyn Fn(RouteContext, Option<usize>, Option<String>) -> PageControllerFuture + Send + Sync>;
#[cfg(feature = "hyper")]
type HttpControllerFuture = Pin<Box<dyn Future<Output = Option<HttpResponse>> + Send>>;
#[cfg(feature = "hyper")]
type HttpControllerFactory = Arc<
	dyn Fn(RouteContext, Option<String>, String, HttpRequest, HttpCtx) -> HttpControllerFuture
		+ Send
		+ Sync,
>;
pub(crate) type SsrRenderer = Arc<dyn Fn(RouteContext, Option<String>) -> Option<SsrResponse> + Send + Sync>;
type SsrComponentFactories = Arc<std::sync::RwLock<Vec<(String, ControllerFactory)>>>;
type SsrPageFactories = Arc<std::sync::RwLock<Vec<(RoutePattern, PageControllerFactory)>>>;
type BoxedCustomComponentController = Box<dyn CustomComponentController>;
type CustomComponentFactory = Arc<dyn Fn() -> BoxedCustomComponentController + Send + Sync>;
type CustomComponentEntries = HashMap<String, String>;

enum PageMount {
	Ready(BoxedController),
	Redirect(String),
}

pub(crate) struct SsrHydrationRoot {
	pub path: String,
	pub item: Item,
	pub title: Option<String>,
}

pub(crate) enum SsrResponse {
	Render { item: Item, title: Option<String> },
	Redirect(String),
}

pub fn custom_component_entry_for_path(path: &str) -> String {
	let trimmed = path.trim().trim_matches('/');
	format!("/fs/wgui-controllers/{trimmed}/controller.js")
}

pub fn custom_component_entry_for_asset(asset: &str) -> String {
	let trimmed = asset.trim().trim_matches('/');
	format!("/fs/wgui-controllers/{trimmed}/controller.js")
}

fn custom_component_asset_for_type<C>() -> String {
	let type_name = std::any::type_name::<C>()
		.rsplit("::")
		.next()
		.unwrap_or("custom-component");
	let base = type_name
		.strip_suffix("Component")
		.or_else(|| type_name.strip_suffix("Controller"))
		.or_else(|| type_name.strip_suffix("View"))
		.unwrap_or(type_name);
	let mut out = String::new();
	for (index, ch) in base.chars().enumerate() {
		if ch.is_ascii_uppercase() {
			if index > 0 {
				out.push('-');
			}
			out.push(ch.to_ascii_lowercase());
		} else if ch == '_' {
			out.push('-');
		} else {
			out.push(ch);
		}
	}
	out
}

fn resolve_custom_component_entries(item: &mut Item, entries: &CustomComponentEntries) {
	match &mut item.payload {
		ItemPayload::Custom { name, entry, .. } => {
			if entry.is_empty() {
				if let Some(resolved) = entries.get(name) {
					*entry = resolved.clone();
				}
			}
		}
		ItemPayload::Layout(layout) => {
			for child in &mut layout.body {
				resolve_custom_component_entries(child, entries);
			}
		}
		ItemPayload::Table { items }
		| ItemPayload::Tbody { items }
		| ItemPayload::Thead { items }
		| ItemPayload::Tr { items } => {
			for child in items {
				resolve_custom_component_entries(child, entries);
			}
		}
		ItemPayload::Th { item } | ItemPayload::Td { item } => {
			resolve_custom_component_entries(item, entries);
		}
		ItemPayload::Modal { body, .. } => {
			for child in body {
				resolve_custom_component_entries(child, entries);
			}
		}
		ItemPayload::ConnectionStatus { body, .. } => {
			for child in body {
				resolve_custom_component_entries(child, entries);
			}
		}
		_ => {}
	}
}

fn collect_rendered_custom_components(item: &Item, out: &mut Vec<RenderedCustomComponent>) {
	match &item.payload {
		ItemPayload::Custom { name, props, .. } => {
			out.push(RenderedCustomComponent {
				path: name.clone(),
				item_id: item.id,
				inx: if item.inx == 0 { None } else { Some(item.inx) },
				props: props.clone(),
			});
		}
		ItemPayload::Layout(layout) => {
			for child in &layout.body {
				collect_rendered_custom_components(child, out);
			}
		}
		ItemPayload::Table { items }
		| ItemPayload::Tbody { items }
		| ItemPayload::Thead { items }
		| ItemPayload::Tr { items } => {
			for child in items {
				collect_rendered_custom_components(child, out);
			}
		}
		ItemPayload::Th { item } | ItemPayload::Td { item } => {
			collect_rendered_custom_components(item, out);
		}
		ItemPayload::Modal { body, .. } => {
			for child in body {
				collect_rendered_custom_components(child, out);
			}
		}
		ItemPayload::ConnectionStatus { body, .. } => {
			for child in body {
				collect_rendered_custom_components(child, out);
			}
		}
		_ => {}
	}
}

fn collect_partial_regions(item: &Item, regions: &mut Vec<(String, Item)>) {
	if !item.partial_addr.is_empty() {
		regions.push((item.partial_addr.clone(), item.clone()));
	}
	match &item.payload {
		ItemPayload::Layout(layout) => {
			for child in &layout.body {
				collect_partial_regions(child, regions);
			}
		}
		ItemPayload::Form { body, .. }
		| ItemPayload::Table { items: body }
		| ItemPayload::Tbody { items: body }
		| ItemPayload::Thead { items: body }
		| ItemPayload::Tr { items: body }
		| ItemPayload::Modal { body, .. }
		| ItemPayload::ConnectionStatus { body, .. } => {
			for child in body {
				collect_partial_regions(child, regions);
			}
		}
		ItemPayload::Th { item } | ItemPayload::Td { item } => {
			collect_partial_regions(item, regions);
		}
		_ => {}
	}
}

fn replace_partial_region(item: &mut Item, addr: &str, replacement: &Item) -> bool {
	if item.partial_addr == addr {
		let mut replacement = replacement.clone();
		replacement.partial_addr = addr.to_string();
		*item = replacement;
		return true;
	}
	match &mut item.payload {
		ItemPayload::Layout(layout) => layout
			.body
			.iter_mut()
			.any(|child| replace_partial_region(child, addr, replacement)),
		ItemPayload::Form { body, .. }
		| ItemPayload::Table { items: body }
		| ItemPayload::Tbody { items: body }
		| ItemPayload::Thead { items: body }
		| ItemPayload::Tr { items: body }
		| ItemPayload::Modal { body, .. }
		| ItemPayload::ConnectionStatus { body, .. } => body
			.iter_mut()
			.any(|child| replace_partial_region(child, addr, replacement)),
		ItemPayload::Th { item } | ItemPayload::Td { item } => {
			replace_partial_region(item, addr, replacement)
		}
		_ => false,
	}
}

#[derive(Clone)]
pub struct CustomComponentCtx {
	handle: WguiHandle,
	pub client_id: usize,
	pub item_id: u32,
	pub inx: Option<u32>,
	pub path: String,
}

impl CustomComponentCtx {
	fn new(
		handle: WguiHandle,
		client_id: usize,
		item_id: u32,
		inx: Option<u32>,
		path: String,
	) -> Self {
		Self {
			handle,
			client_id,
			item_id,
			inx,
			path,
		}
	}

	pub async fn send_data(
		&self,
		name: impl Into<String>,
		payload: serde_json::Value,
	) -> anyhow::Result<()> {
		self.handle
			.send_actions(
				self.client_id,
				vec![ClientAction::CustomData(CustomData {
					id: self.item_id,
					inx: self.inx,
					name: name.into(),
					payload,
				})],
			)
			.await;
		Ok(())
	}
}

#[async_trait::async_trait]
pub trait CustomComponentController: Send + 'static {
	async fn mount(
		&mut self,
		_ctx: CustomComponentCtx,
		_props: serde_json::Value,
	) -> anyhow::Result<()> {
		Ok(())
	}

	async fn process(&mut self, _ctx: CustomComponentCtx) -> anyhow::Result<()> {
		Ok(())
	}

	async fn event(
		&mut self,
		_ctx: CustomComponentCtx,
		_name: String,
		_payload: serde_json::Value,
	) -> anyhow::Result<()> {
		Ok(())
	}

	async fn unmount(&mut self, _ctx: CustomComponentCtx) -> anyhow::Result<()> {
		Ok(())
	}
}

fn component_route_match_score(route_path: &str, current_path: &str) -> Option<usize> {
	if route_path == "/" {
		return Some(0);
	}
	if current_path == route_path {
		return Some(route_path.trim_end_matches('/').len());
	}
	let prefix = format!("{}/", route_path.trim_end_matches('/'));
	if current_path.starts_with(&prefix) {
		return Some(route_path.trim_end_matches('/').len());
	}
	None
}

fn best_component_route_index<T, F>(
	routes: &[T],
	current_path: &str,
	route_path: F,
) -> Option<usize>
where
	F: Fn(&T) -> &str,
{
	let mut best = None;
	for (index, route) in routes.iter().enumerate() {
		let Some(score) = component_route_match_score(route_path(route), current_path) else {
			continue;
		};
		if best
			.map(|(_, best_score)| score > best_score)
			.unwrap_or(true)
		{
			best = Some((index, score));
		}
	}
	best.map(|(index, _)| index)
}

fn page_route_context(
	pattern: &RoutePattern,
	path: &str,
	query: &HashMap<String, String>,
) -> Option<RouteContext> {
	let matched = pattern.match_path(path)?;
	Some(RouteContext {
		path: path.to_string(),
		params: matched.params,
		query: query.clone(),
	})
}

fn component_route_context(path: &str, query: &HashMap<String, String>) -> RouteContext {
	RouteContext {
		path: path.to_string(),
		params: HashMap::new(),
		query: query.clone(),
	}
}

fn route_target(url: &str) -> (String, HashMap<String, String>) {
	let (path, query) = url.split_once('?').unwrap_or((url, ""));
	let query = form_urlencoded::parse(query.as_bytes())
		.into_owned()
		.collect::<HashMap<_, _>>();
	(path.to_string(), query)
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

	pub fn refresh(&self, client_id: usize) {
		let _ = self.event_tx.send(ClientMessage {
			client_id,
			event: ClientEvent::Refresh,
		});
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

	pub async fn replace_root(&self, client_id: usize, item: Item) {
		log::debug!("replace root {:?}", item);
		let clients = self.clients.read().await;
		let sender = match clients.get(&client_id) {
			Some(sender) => sender,
			None => {
				println!("client not found");
				return;
			}
		};
		sender.send(Command::ReplaceRoot(item)).unwrap();
	}

	pub async fn hydrate_root(&self, client_id: usize, item: Item) {
		log::debug!("hydrate root {:?}", item);
		let clients = self.clients.read().await;
		let sender = match clients.get(&client_id) {
			Some(sender) => sender,
			None => {
				println!("client not found");
				return;
			}
		};
		sender.send(Command::HydrateRoot(item)).unwrap();
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

	pub async fn navigate(&self, client_id: usize, url: &str) {
		let clients = self.clients.read().await;
		let sender = match clients.get(&client_id) {
			Some(sender) => sender,
			None => {
				println!("client not found");
				return;
			}
		};
		sender.send(Command::Navigate(url.to_string())).unwrap();
	}

	pub async fn enable_web_push(
		&self,
		client_id: usize,
		service_worker_path: impl Into<String>,
		vapid_public_key: Option<String>,
	) {
		let service_worker_path = service_worker_path.into();
		if service_worker_path.trim().is_empty() {
			return;
		}
		self.send_actions(
			client_id,
			vec![ClientAction::WebPushEnable {
				service_worker_path,
				vapid_public_key,
			}],
		)
		.await;
	}

	pub async fn disable_web_push(&self, client_id: usize, service_worker_path: impl Into<String>) {
		let service_worker_path = service_worker_path.into();
		if service_worker_path.trim().is_empty() {
			return;
		}
		self.send_actions(
			client_id,
			vec![ClientAction::WebPushDisable {
				service_worker_path,
			}],
		)
		.await;
	}

	pub async fn send_actions(&self, client_id: usize, actions: Vec<ClientAction>) {
		if actions.is_empty() {
			return;
		}
		let clients = self.clients.read().await;
		let sender = match clients.get(&client_id) {
			Some(sender) => sender,
			None => {
				println!("client not found");
				return;
			}
		};
		sender.send(Command::Actions(actions)).unwrap();
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
	process_factory: ControllerProcessFactory,
	controllers: HashMap<usize, BoxedController>,
	processes: HashMap<usize, JoinHandle<()>>,
}

impl ComponentRegistration {
	fn new(
		route_path: String,
		factory: ControllerFactory,
		process_factory: ControllerProcessFactory,
	) -> Self {
		Self {
			route_path,
			factory,
			process_factory,
			controllers: HashMap::new(),
			processes: HashMap::new(),
		}
	}

	fn mount_process(&mut self, client_id: usize, event_tx: mpsc::UnboundedSender<ClientMessage>) {
		if let Some(process) = self.processes.remove(&client_id) {
			process.abort();
		}
		let ctx = crate::wui::runtime::ControllerProcessCtx::new(client_id, event_tx);
		self.processes
			.insert(client_id, tokio::spawn((self.process_factory)(ctx)));
	}

	fn unmount(&mut self, client_id: usize) {
		self.controllers.remove(&client_id);
		if let Some(process) = self.processes.remove(&client_id) {
			process.abort();
		}
	}
}

struct PageRegistration {
	pattern: RoutePattern,
	factory: PageControllerFactory,
	process_factory: ControllerProcessFactory,
	controllers: HashMap<usize, BoxedController>,
	processes: HashMap<usize, JoinHandle<()>>,
}

impl PageRegistration {
	fn new(
		pattern: RoutePattern,
		factory: PageControllerFactory,
		process_factory: ControllerProcessFactory,
	) -> Self {
		Self {
			pattern,
			factory,
			process_factory,
			controllers: HashMap::new(),
			processes: HashMap::new(),
		}
	}

	fn mount_process(&mut self, client_id: usize, event_tx: mpsc::UnboundedSender<ClientMessage>) {
		if let Some(process) = self.processes.remove(&client_id) {
			process.abort();
		}
		let ctx = crate::wui::runtime::ControllerProcessCtx::new(client_id, event_tx);
		self.processes
			.insert(client_id, tokio::spawn((self.process_factory)(ctx)));
	}

	fn unmount(&mut self, client_id: usize) {
		self.controllers.remove(&client_id);
		if let Some(process) = self.processes.remove(&client_id) {
			process.abort();
		}
	}
}

struct CustomComponentRegistration {
	path: String,
	entry: String,
	factory: CustomComponentFactory,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CustomComponentKey {
	client_id: usize,
	item_id: u32,
	inx: Option<u32>,
}

#[derive(Clone)]
struct RenderedCustomComponent {
	path: String,
	item_id: u32,
	inx: Option<u32>,
	props: serde_json::Value,
}

struct MountedCustomComponent {
	path: String,
	ctx: CustomComponentCtx,
	controller: Arc<Mutex<BoxedCustomComponentController>>,
	process: JoinHandle<()>,
}

impl MountedCustomComponent {
	async fn unmount(self) {
		self.process.abort();
		let mut controller = self.controller.lock().await;
		if let Err(err) = controller.unmount(self.ctx).await {
			log::warn!("custom component unmount failed: {err}");
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

	fn render_with_route(&self, route: &RouteContext) -> Item {
		self.inner.render_with_route(route)
	}

	fn title(&self) -> Option<String> {
		self.inner.title()
	}

	fn route_title(&self, path: &str) -> Option<String> {
		self.inner.route_title(path)
	}

	fn set_runtime_context(&mut self, client_id: Option<usize>, session: Option<String>) {
		self.ctx.set_current_client(client_id);
		self.ctx.set_current_session(session.clone());
		self.inner.set_runtime_context(client_id, session);
	}

	fn set_route_context(&mut self, route: Option<RouteContext>) {
		self.ctx.set_current_route(route.clone());
		self.inner.set_route_context(route);
	}

	async fn handle(&mut self, event: &crate::types::ClientEvent) -> bool {
		self.inner.handle(event).await
	}
}

pub struct Wgui<DB = ()> {
	events_rx: mpsc::UnboundedReceiver<ClientMessage>,
	handle: WguiHandle,
	components: Vec<ComponentRegistration>,
	pages: Vec<PageRegistration>,
	custom_components: Vec<CustomComponentRegistration>,
	mounted_custom_components: HashMap<CustomComponentKey, MountedCustomComponent>,
	ssr_components: SsrComponentFactories,
	ssr_pages: SsrPageFactories,
	routes: SharedRoutes,
	partials: SharedRoutes,
	db: Arc<DB>,
	contexts: SharedContexts,
	#[cfg(feature = "hyper")]
	http_handler: server::SharedHttpHandler,
	#[cfg(feature = "hyper")]
	http_routes: server::SharedHttpRoutes,
	#[cfg(feature = "hyper")]
	app_css: server::SharedAppCss,
	#[cfg(feature = "hyper")]
	static_mounts: server::SharedStaticMounts,
	#[cfg(feature = "hyper")]
	ssr_hydration_roots: SsrHydrationRoots,
}

impl Wgui<()> {
	#[cfg(feature = "hyper")]
	pub fn new(addr: SocketAddr) -> Self {
		let (events_tx, events_rx) = mpsc::unbounded_channel();
		let clients: Clients = Arc::new(RwLock::new(HashMap::new()));
		let sessions: Sessions = Arc::new(RwLock::new(HashMap::new()));
		let ssr_components: SsrComponentFactories = Arc::new(std::sync::RwLock::new(Vec::new()));
		let ssr_pages: SsrPageFactories = Arc::new(std::sync::RwLock::new(Vec::new()));
		let http_handler = Arc::new(std::sync::RwLock::new(None));
		let http_routes = Arc::new(std::sync::RwLock::new(Vec::new()));
		let app_css = Arc::new(std::sync::RwLock::new(None));
		let static_mounts = Arc::new(std::sync::RwLock::new(Vec::new()));
		let ssr_hydration_roots = Arc::new(RwLock::new(HashMap::new()));
		let routes: SharedRoutes = Arc::new(std::sync::RwLock::new(Vec::new()));
		let partials: SharedRoutes = Arc::new(std::sync::RwLock::new(Vec::new()));
		let contexts: SharedContexts = Arc::new(std::sync::RwLock::new(HashMap::new()));

		{
			let clients = clients.clone();
			let event_tx = events_tx.clone();
			let sessions = sessions.clone();
			let ssr_components = ssr_components.clone();
			let ssr_pages = ssr_pages.clone();
			let http_handler = http_handler.clone();
			let http_routes = http_routes.clone();
			let app_css = app_css.clone();
			let static_mounts = static_mounts.clone();
			let ssr_hydration_roots = ssr_hydration_roots.clone();
			let routes = routes.clone();
			let contexts = contexts.clone();
			let ssr: Option<SsrRenderer> = Some(Arc::new(
				move |route: RouteContext, session: Option<String>| {
					if let Some((handler, state_type_id, params)) = {
						let routes = routes.read().unwrap();
						let mut best: Option<(
							SharedRouteHandler,
							TypeId,
							PathParams,
							crate::wui::routing::RouteScore,
						)> = None;
						for entry in routes.iter() {
							if entry.handler.method() != HttpMethod::Get {
								continue;
							}
							if let Some(matched) = entry.pattern.match_path(&route.path) {
								if best
									.as_ref()
									.map(|(_, _, _, score)| matched.score > *score)
									.unwrap_or(true)
								{
									best = Some((
										entry.handler.clone(),
										entry.state_type_id,
										PathParams(matched.params),
										matched.score,
									));
								}
							}
						}
						best.map(|(handler, state_type_id, params, _)| {
							(handler, state_type_id, params)
						})
					} {
						let ctx_any = contexts.read().unwrap().get(&state_type_id).cloned()?;
						let result = tokio::task::block_in_place(|| {
							tokio::runtime::Handle::current().block_on(handler.call_dyn(
								ctx_any,
								params,
								crate::wui::route_handler::RouteFormData::default(),
								RuntimeContext {
									client_id: None,
									session: session.clone(),
									route: Some(route.clone()),
								},
							))
						});
						return match result {
							RouteResult::View(view) => Some(SsrResponse::Render {
								item: view.item,
								title: view.title,
							}),
							RouteResult::Redirect(redirect) => {
								Some(SsrResponse::Redirect(redirect.0))
							}
							RouteResult::NotFound => None,
						};
					}
					if let Some((factory, route)) = {
						let pages = ssr_pages.read().unwrap();
						let index = best_route_index(&pages, &route.path, |(pattern, _)| pattern)?;
						let pattern = &pages[index].0;
						let route = page_route_context(pattern, &route.path, &route.query)?;
						Some((pages[index].1.clone(), route))
					} {
						let mount = tokio::task::block_in_place(|| {
							tokio::runtime::Handle::current().block_on((factory)(
								route.clone(),
								None,
								session.clone(),
							))
						});
						return match mount {
							PageMount::Ready(mut controller) => {
								controller.set_runtime_context(None, session.clone());
								controller.set_route_context(Some(route.clone()));
								let title = controller
									.title()
									.or_else(|| controller.route_title(&route.path));
								Some(SsrResponse::Render {
									item: controller.render_with_route(&route),
									title,
								})
							}
							PageMount::Redirect(url) => Some(SsrResponse::Redirect(url)),
						};
					}

					let factory = {
						let factories = ssr_components.read().unwrap();
						let index = best_component_route_index(
							&factories,
							&route.path,
							|(route_path, _)| route_path.as_str(),
						)?;
						factories[index].1.clone()
					};
					let route = component_route_context(&route.path, &route.query);
					let mut controller = tokio::task::block_in_place(|| {
						tokio::runtime::Handle::current().block_on((factory)())
					});
					controller.set_runtime_context(None, session);
					controller.set_route_context(Some(route.clone()));
					let title = controller
						.title()
						.or_else(|| controller.route_title(&route.path));
					Some(SsrResponse::Render {
						item: controller.render_with_route(&route),
						title,
					})
				},
			));
			tokio::spawn(async move {
				Server::new(server::ServerConfig {
					addr,
					event_tx,
					clients,
					sessions,
					ssr,
					http_handler,
					http_routes,
					app_css,
					static_mounts,
					ssr_hydration_roots,
				})
				.await
				.run()
				.await;
			});
		}

		Self {
			events_rx,
			handle: WguiHandle::new(events_tx, clients, sessions),
			components: Vec::new(),
			pages: Vec::new(),
			custom_components: Vec::new(),
			mounted_custom_components: HashMap::new(),
			ssr_components,
			ssr_pages,
			routes,
			partials,
			db: Arc::new(()),
			contexts,
			http_handler,
			http_routes,
			app_css,
			static_mounts,
			ssr_hydration_roots,
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
		let ssr_pages: SsrPageFactories = Arc::new(std::sync::RwLock::new(Vec::new()));
		let http_handler = Arc::new(std::sync::RwLock::new(None));
		let http_routes = Arc::new(std::sync::RwLock::new(Vec::new()));
		let app_css = Arc::new(std::sync::RwLock::new(None));
		let static_mounts = Arc::new(std::sync::RwLock::new(Vec::new()));
		let ssr_hydration_roots = Arc::new(RwLock::new(HashMap::new()));
		let routes: SharedRoutes = Arc::new(std::sync::RwLock::new(Vec::new()));
		let partials: SharedRoutes = Arc::new(std::sync::RwLock::new(Vec::new()));
		let contexts: SharedContexts = Arc::new(std::sync::RwLock::new(HashMap::new()));

		{
			let clients = clients.clone();
			let event_tx = events_tx.clone();
			let sessions = sessions.clone();
			let http_handler = http_handler.clone();
			let http_routes = http_routes.clone();
			let app_css = app_css.clone();
			let static_mounts = static_mounts.clone();
			let ssr_hydration_roots = ssr_hydration_roots.clone();
			let ssr: Option<SsrRenderer> = Some(Arc::new(
				move |_route: RouteContext, _session: Option<String>| {
					Some(SsrResponse::Render {
						item: (renderer)(),
						title: None,
					})
				},
			));
			tokio::spawn(async move {
				Server::new(server::ServerConfig {
					addr,
					event_tx,
					clients,
					sessions,
					ssr,
					http_handler,
					http_routes,
					app_css,
					static_mounts,
					ssr_hydration_roots,
				})
				.await
				.run()
				.await;
			});
		}

		Self {
			events_rx,
			handle: WguiHandle::new(events_tx, clients, sessions),
			components: Vec::new(),
			pages: Vec::new(),
			custom_components: Vec::new(),
			mounted_custom_components: HashMap::new(),
			ssr_components,
			ssr_pages,
			routes,
			partials,
			db: Arc::new(()),
			contexts,
			http_handler,
			http_routes,
			app_css,
			static_mounts,
			ssr_hydration_roots,
		}
	}

	pub fn new_without_server() -> Self {
		let (events_tx, events_rx) = mpsc::unbounded_channel();
		let clients: Clients = Arc::new(RwLock::new(HashMap::new()));
		let sessions: Sessions = Arc::new(RwLock::new(HashMap::new()));
		let ssr_components: SsrComponentFactories = Arc::new(std::sync::RwLock::new(Vec::new()));
		let ssr_pages: SsrPageFactories = Arc::new(std::sync::RwLock::new(Vec::new()));
		let routes: SharedRoutes = Arc::new(std::sync::RwLock::new(Vec::new()));
		let partials: SharedRoutes = Arc::new(std::sync::RwLock::new(Vec::new()));
		let contexts: SharedContexts = Arc::new(std::sync::RwLock::new(HashMap::new()));
		#[cfg(feature = "hyper")]
		let http_handler = Arc::new(std::sync::RwLock::new(None));
		#[cfg(feature = "hyper")]
		let http_routes = Arc::new(std::sync::RwLock::new(Vec::new()));
		#[cfg(feature = "hyper")]
		let app_css = Arc::new(std::sync::RwLock::new(None));
		#[cfg(feature = "hyper")]
		let static_mounts = Arc::new(std::sync::RwLock::new(Vec::new()));
		#[cfg(feature = "hyper")]
		let ssr_hydration_roots = Arc::new(RwLock::new(HashMap::new()));

		Self {
			events_rx,
			handle: WguiHandle::new(events_tx, clients, sessions),
			components: Vec::new(),
			pages: Vec::new(),
			custom_components: Vec::new(),
			mounted_custom_components: HashMap::new(),
			ssr_components,
			ssr_pages,
			routes,
			partials,
			db: Arc::new(()),
			contexts,
			#[cfg(feature = "hyper")]
			http_handler,
			#[cfg(feature = "hyper")]
			http_routes,
			#[cfg(feature = "hyper")]
			app_css,
			#[cfg(feature = "hyper")]
			static_mounts,
			#[cfg(feature = "hyper")]
			ssr_hydration_roots,
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
			pages: self.pages,
			custom_components: self.custom_components,
			mounted_custom_components: self.mounted_custom_components,
			ssr_components: self.ssr_components,
			ssr_pages: self.ssr_pages,
			routes: self.routes,
			partials: self.partials,
			db: Arc::new(db),
			contexts: Arc::new(std::sync::RwLock::new(HashMap::new())),
			#[cfg(feature = "hyper")]
			http_handler: self.http_handler,
			#[cfg(feature = "hyper")]
			http_routes: self.http_routes,
			#[cfg(feature = "hyper")]
			app_css: self.app_css,
			#[cfg(feature = "hyper")]
			static_mounts: self.static_mounts,
			#[cfg(feature = "hyper")]
			ssr_hydration_roots: self.ssr_hydration_roots,
		}
	}

	#[cfg(feature = "hyper")]
	fn register_http_routes<C>(&mut self, factory: HttpControllerFactory)
	where
		C: crate::wui::runtime::WuiController + Send + 'static,
	{
		let mut routes = self.http_routes.write().unwrap();
		for spec in C::http_routes() {
			let route_id = spec.id.to_string();
			let handler_factory = factory.clone();
			let handler: server::HttpRouteHandler = Arc::new(move |request, ctx| {
				let handler_factory = handler_factory.clone();
				let route_id = route_id.clone();
				Box::pin(async move {
					let route = RouteContext {
						path: request.path.clone(),
						params: ctx.params.clone(),
						query: request.query.clone(),
					};
					match handler_factory(route, ctx.session.clone(), route_id, request, ctx).await
					{
						Some(response) => response,
						None => HttpResponse::new(404, "controller http route not found"),
					}
				})
			});
			routes.push(server::HttpRoute {
				method: spec.method.to_string(),
				pattern: RoutePattern::parse(spec.path),
				handler,
			});
		}
	}

	#[cfg(feature = "hyper")]
	fn redirect_http_response(url: String) -> HttpResponse {
		HttpResponse::new(303, Vec::new())
			.header("location", url)
			.header("cache-control", "no-store")
	}

	#[cfg(feature = "hyper")]
	pub fn set_css(&self, css: impl Into<String>) {
		*self.app_css.write().unwrap() = Some(css.into());
	}

	#[cfg(feature = "hyper")]
	pub fn set_http_handler<F, Fut>(&self, handler: F)
	where
		F: Fn(HttpRequest) -> Fut + Send + Sync + 'static,
		Fut: Future<Output = Option<HttpResponse>> + Send + 'static,
	{
		let handler: HttpHandler = Arc::new(move |request| Box::pin(handler(request)));
		*self.http_handler.write().unwrap() = Some(handler);
	}

	#[cfg(feature = "hyper")]
	pub fn mount_static_file(
		&self,
		route: impl Into<String>,
		file: impl Into<PathBuf>,
	) -> StaticAsset {
		let (mount, asset) = server::StaticMount::file(route.into(), file.into());
		self.static_mounts.write().unwrap().push(mount);
		asset
	}

	#[cfg(feature = "hyper")]
	pub fn mount_static_dir(&self, route_prefix: impl Into<String>, dir: impl Into<PathBuf>) {
		self.static_mounts
			.write()
			.unwrap()
			.push(server::StaticMount::dir(route_prefix.into(), dir.into()));
	}

	pub fn handle(&self) -> WguiHandle {
		self.handle.clone()
	}

	pub fn add_custom_component<C, F>(&mut self, path: &str, factory: F)
	where
		C: CustomComponentController,
		F: Fn() -> C + Send + Sync + 'static,
	{
		let path = path.to_string();
		let entry = custom_component_entry_for_asset(&custom_component_asset_for_type::<C>());
		let factory: CustomComponentFactory =
			Arc::new(move || Box::new(factory()) as BoxedCustomComponentController);
		self.custom_components.push(CustomComponentRegistration {
			path,
			entry,
			factory,
		});
	}

	pub async fn next(&mut self) -> Option<ClientMessage> {
		self.events_rx.recv().await
	}

	fn custom_component_factory(&self, path: &str) -> Option<CustomComponentFactory> {
		self.custom_components
			.iter()
			.find(|component| component.path == path)
			.map(|component| component.factory.clone())
	}

	fn custom_component_entries(&self) -> CustomComponentEntries {
		self.custom_components
			.iter()
			.map(|component| (component.path.clone(), component.entry.clone()))
			.collect()
	}

	fn prepare_item(&self, mut item: Item) -> Item {
		let entries = self.custom_component_entries();
		resolve_custom_component_entries(&mut item, &entries);
		item
	}

	async fn mount_custom_component(
		&mut self,
		key: CustomComponentKey,
		component: RenderedCustomComponent,
		factory: CustomComponentFactory,
	) {
		let ctx = CustomComponentCtx::new(
			self.handle(),
			key.client_id,
			key.item_id,
			key.inx,
			component.path.clone(),
		);
		let controller = Arc::new(Mutex::new(factory()));
		{
			let mut controller = controller.lock().await;
			if let Err(err) = controller.mount(ctx.clone(), component.props).await {
				log::warn!("custom component mount failed: {err}");
				return;
			}
		}

		let process_controller = controller.clone();
		let process_ctx = ctx.clone();
		let process = tokio::spawn(async move {
			let mut controller = process_controller.lock().await;
			if let Err(err) = controller.process(process_ctx).await {
				log::warn!("custom component process failed: {err}");
			}
		});

		self.mounted_custom_components.insert(
			key,
			MountedCustomComponent {
				path: component.path,
				ctx,
				controller,
				process,
			},
		);
	}

	async fn unmount_custom_component(&mut self, key: &CustomComponentKey) {
		if let Some(mounted) = self.mounted_custom_components.remove(key) {
			mounted.unmount().await;
		}
	}

	async fn unmount_custom_components_for_client(&mut self, client_id: usize) {
		let keys = self
			.mounted_custom_components
			.keys()
			.filter(|key| key.client_id == client_id)
			.cloned()
			.collect::<Vec<_>>();
		for key in keys {
			self.unmount_custom_component(&key).await;
		}
	}

	async fn sync_custom_components(&mut self, client_id: usize, item: &Item) {
		let mut rendered = Vec::new();
		collect_rendered_custom_components(item, &mut rendered);

		let mut seen = HashSet::new();
		for component in rendered {
			if component.item_id == 0 {
				if self.custom_component_factory(&component.path).is_some() {
					log::warn!(
						"registered custom component {} rendered without an id",
						component.path
					);
				}
				continue;
			}

			let Some(factory) = self.custom_component_factory(&component.path) else {
				continue;
			};
			let key = CustomComponentKey {
				client_id,
				item_id: component.item_id,
				inx: component.inx,
			};
			seen.insert(key.clone());

			let needs_mount = match self.mounted_custom_components.get(&key) {
				Some(mounted) if mounted.path == component.path => false,
				Some(_) => {
					self.unmount_custom_component(&key).await;
					true
				}
				None => true,
			};
			if needs_mount {
				self.mount_custom_component(key, component, factory).await;
			}
		}

		let stale = self
			.mounted_custom_components
			.keys()
			.filter(|key| key.client_id == client_id && !seen.contains(*key))
			.cloned()
			.collect::<Vec<_>>();
		for key in stale {
			self.unmount_custom_component(&key).await;
		}
	}

	async fn handle_custom_component_event(&mut self, client_id: usize, custom: &OnCustom) -> bool {
		let key = CustomComponentKey {
			client_id,
			item_id: custom.id,
			inx: custom.inx,
		};
		let Some(mounted) = self.mounted_custom_components.get(&key) else {
			return false;
		};
		let ctx = mounted.ctx.clone();
		let controller = mounted.controller.clone();
		let name = custom.name.clone();
		let payload = custom.payload.clone();
		let mut controller = controller.lock().await;
		if let Err(err) = controller.event(ctx, name, payload).await {
			log::warn!("custom component event failed: {err}");
		}
		true
	}

	pub async fn render(&self, client_id: usize, item: Item) {
		self.handle.render(client_id, self.prepare_item(item)).await
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

	pub async fn enable_web_push(
		&self,
		client_id: usize,
		service_worker_path: impl Into<String>,
		vapid_public_key: Option<String>,
	) {
		self.handle
			.enable_web_push(client_id, service_worker_path, vapid_public_key)
			.await;
	}

	pub async fn disable_web_push(&self, client_id: usize, service_worker_path: impl Into<String>) {
		self.handle
			.disable_web_push(client_id, service_worker_path)
			.await;
	}

	pub fn set_ctx<T>(&mut self, ctx: Arc<crate::wui::runtime::Ctx<T, DB>>)
	where
		T: Send + Sync + 'static,
	{
		let mut command_rx = ctx.take_command_rx();
		let handle = self.handle();
		let event_tx = handle.event_tx.clone();
		tokio::spawn(async move {
			while let Some(command) = command_rx.recv().await {
				match command {
					crate::wui::runtime::RuntimeCommand::RenderPartial { topic } => {
						let _ = event_tx.send(ClientMessage {
							client_id: 0,
							event: ClientEvent::RenderPartial { topic },
						});
					}
					crate::wui::runtime::RuntimeCommand::Refresh { client_id } => {
						handle.refresh(client_id);
					}
					crate::wui::runtime::RuntimeCommand::SetTitle { client_id, title } => {
						handle.set_title(client_id, &title).await;
					}
					crate::wui::runtime::RuntimeCommand::PushState { client_id, url } => {
						handle.push_state(client_id, &url).await;
					}
					crate::wui::runtime::RuntimeCommand::Navigate { client_id, url } => {
						handle.navigate(client_id, &url).await;
					}
					crate::wui::runtime::RuntimeCommand::WebPushEnable {
						client_id,
						service_worker_path,
						vapid_public_key,
					} => {
						handle
							.enable_web_push(client_id, service_worker_path, vapid_public_key)
							.await;
					}
					crate::wui::runtime::RuntimeCommand::WebPushDisable {
						client_id,
						service_worker_path,
					} => {
						handle
							.disable_web_push(client_id, service_worker_path)
							.await;
					}
				}
			}
		});

		let erased: Arc<dyn Any + Send + Sync> = ctx;
		self.contexts
			.write()
			.unwrap()
			.insert(TypeId::of::<T>(), erased);
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

	pub fn add_component_with<C, F, Fut>(&mut self, path: &str, controller: F)
	where
		C: crate::wui::runtime::WuiController + Send + 'static,
		F: Fn() -> Fut + Send + Sync + 'static,
		Fut: Future<Output = C> + Send + 'static,
	{
		let controller = Arc::new(controller);
		let factory_controller = controller.clone();
		let factory: ControllerFactory = Arc::new(move || {
			let fut = factory_controller.as_ref()();
			Box::pin(async move { Box::new(fut.await) as BoxedController })
		});
		#[cfg(feature = "hyper")]
		{
			let http_controller = controller.clone();
			let http_factory: HttpControllerFactory =
				Arc::new(move |route, session, route_id, request, ctx| {
					let http_controller = http_controller.clone();
					Box::pin(async move {
						let mut controller = http_controller.as_ref()().await;
						controller.set_runtime_context(None, session);
						controller.set_route_context(Some(route));
						controller.handle_http(&route_id, request, ctx).await
					})
				});
			self.register_http_routes::<C>(http_factory);
		}
		self.ssr_components
			.write()
			.unwrap()
			.push((path.to_string(), factory.clone()));
		let process_factory: ControllerProcessFactory = Arc::new(|ctx| {
			Box::pin(async move {
				if let Err(err) = C::process(ctx).await {
					log::warn!("controller process failed: {err}");
				}
			})
		});
		self.components.push(ComponentRegistration::new(
			path.to_string(),
			factory,
			process_factory,
		));
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
			.read()
			.unwrap()
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

		#[cfg(feature = "hyper")]
		{
			let http_ctx = ctx.clone();
			let http_factory: HttpControllerFactory =
				Arc::new(move |route, session, route_id, request, ctx| {
					let http_ctx = http_ctx.clone();
					Box::pin(async move {
						http_ctx.set_current_client(None);
						http_ctx.set_current_session(session.clone());
						http_ctx.set_current_route(Some(route.clone()));
						match C::mount(http_ctx.clone(), route.clone()).await {
							MountResult::Ready(mut controller) => {
								controller.set_runtime_context(None, session);
								controller.set_route_context(Some(route));
								controller.handle_http(&route_id, request, ctx).await
							}
							MountResult::Redirect(url) => Some(Self::redirect_http_response(url)),
						}
					})
				});
			self.register_http_routes::<C>(http_factory);
		}

		let factory_ctx = ctx.clone();
		let factory: ControllerFactory = Arc::new(move || {
			let ctx = factory_ctx.clone();
			Box::pin(async move {
				Box::new(ContextAwareController {
					inner: match C::mount(ctx.clone(), RouteContext::default()).await {
						MountResult::Ready(inner) => inner,
						MountResult::Redirect(_) => {
							panic!(
								"component mount cannot redirect; use add_page for routable pages"
							)
						}
					},
					ctx,
				}) as BoxedController
			})
		});
		self.ssr_components
			.write()
			.unwrap()
			.push((path.to_string(), factory.clone()));
		let process_ctx = ctx.clone();
		let process_factory: ControllerProcessFactory = Arc::new(move |controller_ctx| {
			let ctx = process_ctx.clone();
			Box::pin(async move {
				if let Err(err) =
					<C as crate::wui::runtime::Component>::process(ctx, controller_ctx).await
				{
					log::warn!("controller process failed: {err}");
				}
			})
		});
		self.components.push(ComponentRegistration::new(
			path.to_string(),
			factory,
			process_factory,
		));
	}

	/// Register a `#[route]`-generated handler.
	///
	/// `handler` is the `*_route` const produced by the `#[route]` macro
	/// (e.g. `page_index_route`, `action_toggle_route`). The macro emits a
	/// zero-sized marker struct implementing [`RouteHandler`]; passing the
	/// const to `add_route` stores the marker in the route registry.
	///
	/// Routes are matched by specificity (`best_route_index`), not
	/// registration order, so `/*` fallback routes lose to every more
	/// specific pattern.
	///
	/// [`RouteHandler`]: crate::wui::route_handler::RouteHandler
	pub fn add_route<H>(&mut self, handler: H)
	where
		H: crate::wui::route_handler::DynRouteHandler,
	{
		let handler: SharedRouteHandler = Arc::new(handler);
		let pattern = RoutePattern::parse(handler.path());
		let state_type_id = handler.state_type_id();
		let method = handler.method();
		let path_str = handler.path().to_string();

		// For POST handlers, also register with the HTTP server so real
		// form submissions reach them. GET handlers are dispatched via the
		// WS PathChanged/Refresh event loop instead — they don't need an
		// HTTP route entry.
		if method == crate::wui::route_handler::HttpMethod::Post {
			#[cfg(feature = "hyper")]
			{
				let contexts = self.contexts.clone();
				let handler_arc = handler.clone();
				let handler_path = handler_arc.path().to_string();
				let handler_method = method.as_str().to_string();
				let http_handler: server::HttpRouteHandler =
					Arc::new(move |request: HttpRequest, http_ctx: HttpCtx| {
						let handler_arc = handler_arc.clone();
						let contexts = contexts.clone();
						let path_str = path_str.clone();
						Box::pin(async move {
							let state_type_id = handler_arc.state_type_id();
							let ctx_any = contexts
								.read()
								.unwrap()
								.get(&state_type_id)
								.cloned()
								.expect("missing Ctx<T> for #[route] POST handler");
							let params_map = RoutePattern::parse(&path_str)
								.match_path(&http_ctx.path)
								.map(|m| m.params)
								.unwrap_or_default();
							let route = RouteContext {
								path: http_ctx.path.clone(),
								params: params_map.clone(),
								query: http_ctx.query.clone(),
							};
							let runtime = crate::wui::route_handler::RuntimeContext {
								client_id: None,
								session: http_ctx.session.clone(),
								route: Some(route),
							};
							let params = crate::wui::route_handler::PathParams(params_map);
							let form = crate::wui::route_handler::RouteFormData::from_urlencoded(
								&request.body,
							);
							let result = handler_arc.call_dyn(ctx_any, params, form, runtime).await;
							match result {
								crate::wui::route_handler::RouteResult::Redirect(redirect) => {
									if redirect.0.is_empty() {
										Self::redirect_http_response(http_ctx.path.clone())
									} else {
										Self::redirect_http_response(redirect.0)
									}
								}
								crate::wui::route_handler::RouteResult::View(view) => {
									let body = serde_json::to_vec(&view.item)
										.unwrap_or_else(|_| b"{}".to_vec());
									HttpResponse::new(200, body)
										.header("content-type", "application/json")
								}
								crate::wui::route_handler::RouteResult::NotFound => {
									HttpResponse::new(404, "not found")
								}
							}
						})
					});
				self.http_routes.write().unwrap().push(server::HttpRoute {
					method: handler_method,
					pattern: RoutePattern::parse(&handler_path),
					handler: http_handler,
				});
			}
			#[cfg(not(feature = "hyper"))]
			{
				let _ = (method, path_str);
			}
		}

		self.routes.write().unwrap().push(RouteEntry {
			pattern,
			handler,
			state_type_id,
		});
	}

	/// Register a `#[partial]`-generated handler.
	///
	/// A partial is rendered only for clients whose current route contains a
	/// matching [`partial_region`] marker. Use `Ctx::render` with its concrete
	/// address to re-run it and send a normal VDOM diff.
	pub fn add_partial<H>(&mut self, handler: H)
	where
		H: crate::wui::route_handler::DynRouteHandler,
	{
		let handler: SharedRouteHandler = Arc::new(handler);
		if handler.method() != HttpMethod::Get {
			panic!("#[partial] handlers must use GET semantics");
		}
		self.partials.write().unwrap().push(RouteEntry {
			pattern: RoutePattern::parse(handler.path()),
			state_type_id: handler.state_type_id(),
			handler,
		});
	}

	/// Find the best matching `#[route]` handler for `path` with the given
	/// HTTP method. Returns the index, extracted path params, and the
	/// `state_type_id` key for looking up the `Ctx<T>` in `self.contexts`.
	fn match_route(
		&self,
		path: &str,
		method: crate::wui::route_handler::HttpMethod,
	) -> Option<RouteMatchResult> {
		let mut best_index: Option<usize> = None;
		let mut best_score = crate::wui::routing::RouteScore {
			static_segments: 0,
			total_segments: 0,
			dynamic_segments: 0,
			exact: false,
		};
		let mut best_params: Option<HashMap<String, String>> = None;
		let routes = self.routes.read().unwrap();
		for (index, entry) in routes.iter().enumerate() {
			if entry.handler.method() != method {
				continue;
			}
			if let Some(matched) = entry.pattern.match_path(path) {
				if best_index.is_none() || matched.score > best_score {
					best_index = Some(index);
					best_score = matched.score;
					best_params = Some(matched.params);
				}
			}
		}
		best_index.map(|index| RouteMatchResult {
			index,
			params: crate::wui::route_handler::PathParams(best_params.unwrap_or_default()),
		})
	}

	fn match_partial(&self, path: &str) -> Option<RouteMatchResult> {
		let mut best_index: Option<usize> = None;
		let mut best_score = crate::wui::routing::RouteScore {
			static_segments: 0,
			total_segments: 0,
			dynamic_segments: 0,
			exact: false,
		};
		let mut best_params: Option<HashMap<String, String>> = None;
		let partials = self.partials.read().unwrap();
		for (index, entry) in partials.iter().enumerate() {
			if let Some(matched) = entry.pattern.match_path(path) {
				if best_index.is_none() || matched.score > best_score {
					best_index = Some(index);
					best_score = matched.score;
					best_params = Some(matched.params);
				}
			}
		}
		best_index.map(|index| RouteMatchResult {
			index,
			params: PathParams(best_params.unwrap_or_default()),
		})
	}

	async fn dispatch_route(
		&self,
		route_match: RouteMatchResult,
		form: crate::wui::route_handler::RouteFormData,
		client_id: Option<usize>,
		session: Option<String>,
		route: RouteContext,
	) -> crate::wui::route_handler::RouteResult {
		let (handler, state_type_id) = {
			let routes = self.routes.read().unwrap();
			let entry = &routes[route_match.index];
			(entry.handler.clone(), entry.state_type_id)
		};
		let ctx_any = self
			.contexts
			.read()
			.unwrap()
			.get(&state_type_id)
			.cloned()
			.expect("missing Ctx<T> for #[route] handler; call wgui.set_ctx(...) first");
		handler
			.call_dyn(
				ctx_any,
				route_match.params,
				form,
				crate::wui::route_handler::RuntimeContext {
					client_id,
					session,
					route: Some(route),
				},
			)
			.await
	}

	async fn dispatch_partial(
		&self,
		route_match: RouteMatchResult,
		client_id: usize,
		session: Option<String>,
		route: RouteContext,
	) -> RouteResult {
		let (handler, state_type_id) = {
			let partials = self.partials.read().unwrap();
			let entry = &partials[route_match.index];
			(entry.handler.clone(), entry.state_type_id)
		};
		let ctx_any = self
			.contexts
			.read()
			.unwrap()
			.get(&state_type_id)
			.cloned()
			.expect("missing Ctx<T> for #[partial] handler; call wgui.set_ctx(...) first");
		let partial_addr = route.path.clone();
		let mut result = handler
			.call_dyn(
				ctx_any,
				route_match.params,
				RouteFormData::default(),
				RuntimeContext {
					client_id: Some(client_id),
					session,
					route: Some(route),
				},
			)
			.await;
		if let RouteResult::View(view) = &mut result {
			if view.partial_addr.is_none() {
				view.partial_addr = Some(partial_addr);
			}
		}
		result
	}

	async fn render_route_view(
		&mut self,
		client_id: usize,
		view: crate::wui::route_handler::View,
		custom_component_entries: &CustomComponentEntries,
	) {
		if let Some(title) = &view.title {
			self.handle.set_title(client_id, title).await;
		}
		let mut rendered = view.item.clone();
		resolve_custom_component_entries(&mut rendered, custom_component_entries);
		self.handle.render(client_id, rendered).await;
		self.sync_custom_components(client_id, &view.item).await;
		for page in self.pages.iter_mut() {
			page.unmount(client_id);
		}
		for component in self.components.iter_mut() {
			component.unmount(client_id);
		}
	}

	fn client_session_for_route(&self, route: RouteContext, page_tree: Item) -> ClientSession {
		let mut partials = HashMap::new();
		let mut regions = Vec::new();
		collect_partial_regions(&page_tree, &mut regions);
		for (addr, tree) in regions {
			if let Some(route_match) = self.match_partial(&addr) {
				partials.insert(
					addr,
					PartialCache {
						params: route_match.params,
						tree,
						last_acked_version: 0,
					},
				);
			}
		}
		ClientSession {
			current_route: route,
			page_tree,
			partials,
		}
	}

	pub fn add_page_with<C, F, Fut>(&mut self, route: &str, controller: F)
	where
		C: crate::wui::runtime::WuiController + Send + 'static,
		F: Fn() -> Fut + Send + Sync + 'static,
		Fut: Future<Output = C> + Send + 'static,
	{
		let controller = Arc::new(controller);
		let pattern = RoutePattern::parse(route);
		let factory_controller = controller.clone();
		let factory: PageControllerFactory = Arc::new(move |_route, _client_id, _session| {
			let fut = factory_controller.as_ref()();
			Box::pin(async move { PageMount::Ready(Box::new(fut.await) as BoxedController) })
		});
		#[cfg(feature = "hyper")]
		{
			let http_controller = controller.clone();
			let http_factory: HttpControllerFactory =
				Arc::new(move |route, session, route_id, request, ctx| {
					let http_controller = http_controller.clone();
					Box::pin(async move {
						let mut controller = http_controller.as_ref()().await;
						controller.set_runtime_context(None, session);
						controller.set_route_context(Some(route));
						controller.handle_http(&route_id, request, ctx).await
					})
				});
			self.register_http_routes::<C>(http_factory);
		}
		self.ssr_pages
			.write()
			.unwrap()
			.push((pattern.clone(), factory.clone()));
		let process_factory: ControllerProcessFactory = Arc::new(|ctx| {
			Box::pin(async move {
				if let Err(err) = C::process(ctx).await {
					log::warn!("controller process failed: {err}");
				}
			})
		});
		self.pages
			.push(PageRegistration::new(pattern, factory, process_factory));
	}

	pub fn add_page_with_route<C, F, Fut>(&mut self, route: &str, controller: F)
	where
		C: crate::wui::runtime::WuiController + Send + 'static,
		F: Fn(RouteContext) -> Fut + Send + Sync + 'static,
		Fut: Future<Output = C> + Send + 'static,
	{
		let controller = Arc::new(controller);
		let pattern = RoutePattern::parse(route);
		let factory_controller = controller.clone();
		let factory: PageControllerFactory = Arc::new(move |route, _client_id, _session| {
			let fut = factory_controller.as_ref()(route);
			Box::pin(async move { PageMount::Ready(Box::new(fut.await) as BoxedController) })
		});
		#[cfg(feature = "hyper")]
		{
			let http_controller = controller.clone();
			let http_factory: HttpControllerFactory =
				Arc::new(move |route, session, route_id, request, ctx| {
					let http_controller = http_controller.clone();
					Box::pin(async move {
						let mut controller = http_controller.as_ref()(route.clone()).await;
						controller.set_runtime_context(None, session);
						controller.set_route_context(Some(route));
						controller.handle_http(&route_id, request, ctx).await
					})
				});
			self.register_http_routes::<C>(http_factory);
		}
		self.ssr_pages
			.write()
			.unwrap()
			.push((pattern.clone(), factory.clone()));
		let process_factory: ControllerProcessFactory = Arc::new(|ctx| {
			Box::pin(async move {
				if let Err(err) = C::process(ctx).await {
					log::warn!("controller process failed: {err}");
				}
			})
		});
		self.pages
			.push(PageRegistration::new(pattern, factory, process_factory));
	}

	pub fn add_page<C>(&mut self, route: &str)
	where
		C: crate::wui::runtime::Component<Db = DB>
			+ crate::wui::runtime::WuiController
			+ Send
			+ 'static,
		<C as crate::wui::runtime::Component>::Context: Send + Sync + 'static,
	{
		let Some(ctx_any) = self
			.contexts
			.read()
			.unwrap()
			.get(&TypeId::of::<<C as crate::wui::runtime::Component>::Context>())
			.cloned()
		else {
			panic!("missing context for page; call wgui.set_ctx(...) first");
		};
		let Ok(ctx) = ctx_any
			.downcast::<crate::wui::runtime::Ctx<<C as crate::wui::runtime::Component>::Context, DB>>(
			)
		else {
			panic!("invalid context type for page");
		};

		#[cfg(feature = "hyper")]
		{
			let http_ctx = ctx.clone();
			let http_factory: HttpControllerFactory =
				Arc::new(move |route, session, route_id, request, ctx| {
					let http_ctx = http_ctx.clone();
					Box::pin(async move {
						http_ctx.set_current_client(None);
						http_ctx.set_current_session(session.clone());
						http_ctx.set_current_route(Some(route.clone()));
						match C::mount(http_ctx.clone(), route.clone()).await {
							MountResult::Ready(mut controller) => {
								controller.set_runtime_context(None, session);
								controller.set_route_context(Some(route));
								controller.handle_http(&route_id, request, ctx).await
							}
							MountResult::Redirect(url) => Some(Self::redirect_http_response(url)),
						}
					})
				});
			self.register_http_routes::<C>(http_factory);
		}

		let pattern = RoutePattern::parse(route);
		let process_ctx = ctx.clone();
		let process_factory: ControllerProcessFactory = Arc::new(move |controller_ctx| {
			let ctx = process_ctx.clone();
			Box::pin(async move {
				if let Err(err) =
					<C as crate::wui::runtime::Component>::process(ctx, controller_ctx).await
				{
					log::warn!("controller process failed: {err}");
				}
			})
		});
		let factory: PageControllerFactory = Arc::new(move |route, client_id, session| {
			let ctx = ctx.clone();
			Box::pin(async move {
				ctx.set_current_client(client_id);
				ctx.set_current_session(session.clone());
				ctx.set_current_route(Some(route.clone()));
				match C::mount(ctx.clone(), route).await {
					MountResult::Ready(inner) => PageMount::Ready(
						Box::new(ContextAwareController { inner, ctx }) as BoxedController,
					),
					MountResult::Redirect(url) => PageMount::Redirect(url),
				}
			})
		});
		self.ssr_pages
			.write()
			.unwrap()
			.push((pattern.clone(), factory.clone()));
		self.pages
			.push(PageRegistration::new(pattern, factory, process_factory));
	}

	pub async fn run(&mut self) {
		let handle = self.handle();
		let mut routes: HashMap<usize, RouteContext> = HashMap::new();
		let mut client_sessions: HashMap<usize, ClientSession> = HashMap::new();
		let mut selected_pages: HashMap<usize, Option<usize>> = HashMap::new();
		let mut rtc_rooms: HashMap<String, BTreeSet<usize>> = HashMap::new();
		let mut rtc_client_rooms: HashMap<usize, BTreeSet<String>> = HashMap::new();
		let mut rtc_room_names: HashMap<String, HashMap<usize, String>> = HashMap::new();

		while let Some(message) = self.next().await {
			let client_id = message.client_id;
			let custom_component_entries = self.custom_component_entries();
			match &message.event {
				ClientEvent::Connected { id: _ } => {}
				ClientEvent::Disconnected { id: _ } => {
					selected_pages.remove(&client_id);
					routes.remove(&client_id);
					client_sessions.remove(&client_id);
					if let Some(rooms) = rtc_client_rooms.remove(&client_id) {
						for room in rooms {
							let mut room_peers = Vec::new();
							let remove_room = if let Some(participants) = rtc_rooms.get_mut(&room) {
								participants.remove(&client_id);
								room_peers = participants.iter().copied().collect::<Vec<_>>();
								participants.is_empty()
							} else {
								false
							};
							if let Some(names) = rtc_room_names.get_mut(&room) {
								names.remove(&client_id);
								if names.is_empty() {
									rtc_room_names.remove(&room);
								}
							}

							if remove_room {
								rtc_rooms.remove(&room);
								rtc_room_names.remove(&room);
								continue;
							}
							let room_participants = room_peers
								.iter()
								.map(|peer_id| WebRtcParticipant {
									client_id: *peer_id,
									display_name: rtc_room_names
										.get(&room)
										.and_then(|names| names.get(peer_id))
										.cloned()
										.unwrap_or_else(|| format!("user {}", peer_id)),
								})
								.collect::<Vec<_>>();

							for peer_id in &room_peers {
								handle
									.send_actions(
										*peer_id,
										vec![ClientAction::WebRtcRoomState {
											room: room.clone(),
											self_client_id: *peer_id,
											peers: room_peers.clone(),
											participants: room_participants.clone(),
										}],
									)
									.await;
							}
						}
					}

					for component in self.components.iter_mut() {
						component.unmount(client_id);
					}
					for page in self.pages.iter_mut() {
						page.unmount(client_id);
					}
					self.unmount_custom_components_for_client(client_id).await;
					routes.remove(&client_id);
					handle.clear_session(client_id).await;
				}
				ClientEvent::WebRtcJoin(join) => {
					if join.room.is_empty() {
						continue;
					}
					let display_name = join
						.display_name
						.clone()
						.map(|name| name.trim().to_string())
						.filter(|name| !name.is_empty())
						.unwrap_or_else(|| format!("user {}", client_id));

					let peers = {
						let participants = rtc_rooms.entry(join.room.clone()).or_default();
						participants.insert(client_id);
						participants.iter().copied().collect::<Vec<_>>()
					};
					rtc_room_names
						.entry(join.room.clone())
						.or_default()
						.insert(client_id, display_name);
					rtc_client_rooms
						.entry(client_id)
						.or_default()
						.insert(join.room.clone());
					let room_participants = peers
						.iter()
						.map(|peer_id| WebRtcParticipant {
							client_id: *peer_id,
							display_name: rtc_room_names
								.get(&join.room)
								.and_then(|names| names.get(peer_id))
								.cloned()
								.unwrap_or_else(|| format!("user {}", peer_id)),
						})
						.collect::<Vec<_>>();

					for peer_id in &peers {
						handle
							.send_actions(
								*peer_id,
								vec![ClientAction::WebRtcRoomState {
									room: join.room.clone(),
									self_client_id: *peer_id,
									peers: peers.clone(),
									participants: room_participants.clone(),
								}],
							)
							.await;
					}
				}
				ClientEvent::WebRtcLeave(leave) => {
					let mut peers = Vec::new();
					let remove_room = if let Some(participants) = rtc_rooms.get_mut(&leave.room) {
						participants.remove(&client_id);
						peers = participants.iter().copied().collect::<Vec<_>>();
						participants.is_empty()
					} else {
						false
					};
					if let Some(names) = rtc_room_names.get_mut(&leave.room) {
						names.remove(&client_id);
						if names.is_empty() {
							rtc_room_names.remove(&leave.room);
						}
					}

					if let Some(rooms) = rtc_client_rooms.get_mut(&client_id) {
						rooms.remove(&leave.room);
						if rooms.is_empty() {
							rtc_client_rooms.remove(&client_id);
						}
					}

					if remove_room {
						rtc_rooms.remove(&leave.room);
						rtc_room_names.remove(&leave.room);
						continue;
					}
					let room_participants = peers
						.iter()
						.map(|peer_id| WebRtcParticipant {
							client_id: *peer_id,
							display_name: rtc_room_names
								.get(&leave.room)
								.and_then(|names| names.get(peer_id))
								.cloned()
								.unwrap_or_else(|| format!("user {}", peer_id)),
						})
						.collect::<Vec<_>>();

					for peer_id in &peers {
						handle
							.send_actions(
								*peer_id,
								vec![ClientAction::WebRtcRoomState {
									room: leave.room.clone(),
									self_client_id: *peer_id,
									peers: peers.clone(),
									participants: room_participants.clone(),
								}],
							)
							.await;
					}
				}
				ClientEvent::WebRtcSignal(signal) => {
					let participants = rtc_rooms
						.get(&signal.room)
						.map(|ids| ids.iter().copied().collect::<Vec<_>>())
						.unwrap_or_default();
					if !participants.contains(&client_id) {
						continue;
					}

					let recipients = if let Some(target_id) = signal.target_client_id {
						if participants.contains(&target_id) {
							vec![target_id]
						} else {
							Vec::new()
						}
					} else {
						participants
							.into_iter()
							.filter(|id| *id != client_id)
							.collect::<Vec<_>>()
					};

					for target_id in recipients {
						handle
							.send_actions(
								target_id,
								vec![ClientAction::WebRtcSignal {
									room: signal.room.clone(),
									from_client_id: client_id,
									payload: signal.payload.clone(),
								}],
							)
							.await;
					}
				}
				ClientEvent::RenderPartial { topic } => {
					let targets = client_sessions
						.iter()
						.filter_map(|(client_id, session)| {
							session.partials.get(topic).map(|cache| {
								(
									*client_id,
									session.current_route.clone(),
									cache.params.clone(),
								)
							})
						})
						.collect::<Vec<_>>();
					for (target_client_id, parent_route, params) in targets {
						let Some(partial_match) = self.match_partial(topic) else {
							continue;
						};
						let session = handle.session_for_client(target_client_id).await;
						let result = self
							.dispatch_partial(
								partial_match,
								target_client_id,
								session,
								RouteContext {
									path: topic.clone(),
									params: params.0,
									query: parent_route.query,
								},
							)
							.await;
						let RouteResult::View(view) = result else {
							continue;
						};
						let Some(client_session) = client_sessions.get_mut(&target_client_id)
						else {
							continue;
						};
						if !replace_partial_region(&mut client_session.page_tree, topic, &view.item)
						{
							continue;
						}
						if let Some(cache) = client_session.partials.get_mut(topic) {
							cache.tree = view.item;
							cache.last_acked_version += 1;
						}
						let mut rendered = client_session.page_tree.clone();
						resolve_custom_component_entries(&mut rendered, &custom_component_entries);
						handle.render(target_client_id, rendered).await;
						self.sync_custom_components(target_client_id, &client_session.page_tree)
							.await;
					}
				}
				ClientEvent::FormSubmit(submit) => {
					let session = handle.session_for_client(client_id).await;
					let Some(route_match) =
						self.match_route(&submit.path, crate::wui::route_handler::HttpMethod::Post)
					else {
						continue;
					};
					let action_route = RouteContext {
						path: submit.path.clone(),
						params: route_match.params.0.clone(),
						query: submit.query.clone(),
					};
					let result = self
						.dispatch_route(
							route_match,
							crate::wui::route_handler::RouteFormData::from_fields(
								submit.fields.clone(),
							),
							Some(client_id),
							session.clone(),
							action_route,
						)
						.await;
					match result {
						crate::wui::route_handler::RouteResult::View(view) => {
							self.render_route_view(client_id, view, &custom_component_entries)
								.await;
						}
						crate::wui::route_handler::RouteResult::Redirect(redirect) => {
							let current_route = routes.get(&client_id).cloned();
							let target = if redirect.0.is_empty() {
								current_route.unwrap_or_else(|| {
									component_route_context("/", &HashMap::new())
								})
							} else {
								let (path, query) = route_target(&redirect.0);
								let Some(target_match) = self
									.match_route(&path, crate::wui::route_handler::HttpMethod::Get)
								else {
									handle.navigate(client_id, &redirect.0).await;
									continue;
								};
								let target_route = RouteContext {
									path: path.clone(),
									params: target_match.params.0.clone(),
									query,
								};
								let rendered = self
									.dispatch_route(
										target_match,
										crate::wui::route_handler::RouteFormData::default(),
										Some(client_id),
										session.clone(),
										target_route.clone(),
									)
									.await;
								if let crate::wui::route_handler::RouteResult::View(view) = rendered
								{
									if current_route.as_ref() != Some(&target_route) {
										handle.push_state(client_id, &redirect.0).await;
									}
									routes.insert(client_id, target_route.clone());
									selected_pages.insert(client_id, None);
									client_sessions.insert(
										client_id,
										self.client_session_for_route(
											target_route,
											view.item.clone(),
										),
									);
									self.render_route_view(
										client_id,
										view,
										&custom_component_entries,
									)
									.await;
								}
								continue;
							};
							if let Some(target_match) = self.match_route(
								&target.path,
								crate::wui::route_handler::HttpMethod::Get,
							) {
								let target = RouteContext {
									path: target.path.clone(),
									params: target_match.params.0.clone(),
									query: target.query.clone(),
								};
								if let crate::wui::route_handler::RouteResult::View(view) = self
									.dispatch_route(
										target_match,
										crate::wui::route_handler::RouteFormData::default(),
										Some(client_id),
										session,
										target.clone(),
									)
									.await
								{
									routes.insert(client_id, target.clone());
									selected_pages.insert(client_id, None);
									client_sessions.insert(
										client_id,
										self.client_session_for_route(target, view.item.clone()),
									);
									self.render_route_view(
										client_id,
										view,
										&custom_component_entries,
									)
									.await;
								}
							}
						}
						crate::wui::route_handler::RouteResult::NotFound => {}
					}
				}
				ClientEvent::PathChanged(change) => {
					let session = handle.session_for_client(client_id).await;
					let mut initial_root = change.initial_root.clone();
					let mut hydrated_title: Option<String> = None;
					#[cfg(feature = "hyper")]
					if initial_root.is_none() {
						if let Some(hydration_id) = &change.ssr_hydration_id {
							let mut roots = self.ssr_hydration_roots.write().await;
							if let Some(root) = roots.remove(hydration_id) {
								if root.path == change.path {
									hydrated_title = root.title;
									initial_root = Some(root.item);
								}
							}
						}
					}

					// ── #[route] handler dispatch (GET pages) ──────────────
					// Check new-style free-fn routes before the legacy
					// `add_page` registry. If a matching GET handler is
					// found, run it and render/diff directly — no
					// WuiController mount, no process() lifecycle.
					if let Some(route_match) =
						self.match_route(&change.path, crate::wui::route_handler::HttpMethod::Get)
					{
						let active_route = RouteContext {
							path: change.path.clone(),
							params: route_match.params.0.clone(),
							query: change.query.clone(),
						};
						routes.insert(client_id, active_route.clone());
						let result = self
							.dispatch_route(
								route_match,
								crate::wui::route_handler::RouteFormData::default(),
								Some(client_id),
								session.clone(),
								active_route.clone(),
							)
							.await;
						match result {
							crate::wui::route_handler::RouteResult::View(view) => {
								client_sessions.insert(
									client_id,
									self.client_session_for_route(
										active_route.clone(),
										view.item.clone(),
									),
								);
								if let Some(title) = &view.title {
									if hydrated_title.as_deref() != Some(title.as_str()) {
										handle.set_title(client_id, title).await;
									}
								}
								let mut rendered = view.item.clone();
								resolve_custom_component_entries(
									&mut rendered,
									&custom_component_entries,
								);
								if let Some(root) = initial_root.clone() {
									handle.hydrate_root(client_id, root).await;
									handle.render(client_id, rendered).await;
								} else {
									handle.render(client_id, rendered).await;
								}
								self.sync_custom_components(client_id, &view.item).await;
								for page in self.pages.iter_mut() {
									page.unmount(client_id);
								}
								for component in self.components.iter_mut() {
									component.unmount(client_id);
								}
								selected_pages.insert(client_id, None);
								continue;
							}
							crate::wui::route_handler::RouteResult::Redirect(redirect) => {
								if !redirect.0.is_empty() {
									handle.push_state(client_id, &redirect.0).await;
								}
								continue;
							}
							crate::wui::route_handler::RouteResult::NotFound => {
								// Fall through to legacy pages below
							}
						}
					}

					let selected_page =
						best_route_index(&self.pages, &change.path, |page| &page.pattern);
					let selected_page_changed =
						selected_pages.get(&client_id).copied().flatten() != selected_page;
					selected_pages.insert(client_id, selected_page);
					let active_route = if let Some(index) = selected_page {
						page_route_context(&self.pages[index].pattern, &change.path, &change.query)
							.unwrap_or_else(|| component_route_context(&change.path, &change.query))
					} else {
						component_route_context(&change.path, &change.query)
					};
					routes.insert(client_id, active_route.clone());

					let mut rendered_custom_sync: Option<Item> = None;
					for (index, page) in self.pages.iter_mut().enumerate() {
						if Some(index) != selected_page {
							page.unmount(client_id);
							continue;
						}

						page.unmount(client_id);
						match (page.factory)(active_route.clone(), Some(client_id), session.clone())
							.await
						{
							PageMount::Ready(mut controller) => {
								controller.set_runtime_context(Some(client_id), session.clone());
								controller.set_route_context(Some(active_route.clone()));
								let item = controller.render_with_route(&active_route);
								let title = controller
									.title()
									.or_else(|| controller.route_title(&active_route.path));
								if let Some(title) = title {
									if hydrated_title.as_deref() != Some(title.as_str()) {
										handle.set_title(client_id, &title).await;
									}
								}
								let mut rendered = item.clone();
								resolve_custom_component_entries(
									&mut rendered,
									&custom_component_entries,
								);
								if selected_page_changed {
									if let Some(root) = initial_root.clone() {
										handle.hydrate_root(client_id, root).await;
										handle.render(client_id, rendered).await;
									} else {
										handle.replace_root(client_id, rendered).await;
									}
								} else {
									handle.render(client_id, rendered).await;
								}
								rendered_custom_sync = Some(item);
								page.controllers.insert(client_id, controller);
								page.mount_process(client_id, handle.event_tx.clone());
							}
							PageMount::Redirect(url) => {
								page.unmount(client_id);
								handle.push_state(client_id, &url).await;
							}
						}
					}
					if let Some(item) = rendered_custom_sync {
						self.sync_custom_components(client_id, &item).await;
					} else if selected_page.is_some() {
						self.unmount_custom_components_for_client(client_id).await;
					}

					if selected_page.is_some() {
						for component in self.components.iter_mut() {
							component.unmount(client_id);
						}
						continue;
					}

					let selected_component =
						best_component_route_index(&self.components, &change.path, |component| {
							component.route_path.as_str()
						});

					let mut hydrated_initial_root = false;
					let mut rendered_custom_sync: Option<Item> = None;
					for (index, component) in self.components.iter_mut().enumerate() {
						if Some(index) != selected_component {
							component.unmount(client_id);
							continue;
						}

						if let Some(controller) = component.controllers.get_mut(&client_id) {
							controller.set_runtime_context(Some(client_id), session.clone());
							controller.set_route_context(Some(active_route.clone()));
							let item = controller.render_with_route(&active_route);
							let title = controller
								.title()
								.or_else(|| controller.route_title(&active_route.path));
							if let Some(title) = title {
								if hydrated_title.as_deref() != Some(title.as_str()) {
									handle.set_title(client_id, &title).await;
								}
							}
							let mut rendered = item.clone();
							resolve_custom_component_entries(
								&mut rendered,
								&custom_component_entries,
							);
							handle.render(client_id, rendered).await;
							rendered_custom_sync = Some(item);
						} else {
							let mut controller = (component.factory)().await;
							controller.set_runtime_context(Some(client_id), session.clone());
							controller.set_route_context(Some(active_route.clone()));
							let item = controller.render_with_route(&active_route);
							let title = controller
								.title()
								.or_else(|| controller.route_title(&active_route.path));
							if let Some(title) = title {
								if hydrated_title.as_deref() != Some(title.as_str()) {
									handle.set_title(client_id, &title).await;
								}
							}
							let mut rendered = item.clone();
							resolve_custom_component_entries(
								&mut rendered,
								&custom_component_entries,
							);
							if !hydrated_initial_root {
								if let Some(root) = initial_root.clone() {
									handle.hydrate_root(client_id, root).await;
									hydrated_initial_root = true;
								}
							}
							handle.render(client_id, rendered).await;
							rendered_custom_sync = Some(item);
							component.controllers.insert(client_id, controller);
							component.mount_process(client_id, handle.event_tx.clone());
						}
					}
					if let Some(item) = rendered_custom_sync {
						self.sync_custom_components(client_id, &item).await;
					} else {
						self.unmount_custom_components_for_client(client_id).await;
					}
				}
				ClientEvent::Refresh => {
					let session = handle.session_for_client(client_id).await;
					let route = routes
						.get(&client_id)
						.cloned()
						.unwrap_or_else(|| component_route_context("/", &HashMap::new()));
					let mut rendered_custom_sync: Option<Item> = None;

					if let Some(page_index) = selected_pages.get(&client_id).copied().flatten() {
						if let Some(controller) =
							self.pages[page_index].controllers.get_mut(&client_id)
						{
							controller.set_runtime_context(Some(client_id), session.clone());
							controller.set_route_context(Some(route.clone()));
							let item = controller.render_with_route(&route);
							let title = controller
								.title()
								.or_else(|| controller.route_title(&route.path));
							if let Some(title) = title {
								handle.set_title(client_id, &title).await;
							}
							let mut rendered = item.clone();
							resolve_custom_component_entries(
								&mut rendered,
								&custom_component_entries,
							);
							handle.render(client_id, rendered).await;
							rendered_custom_sync = Some(item);
						}
					} else if let Some(component_index) =
						best_component_route_index(&self.components, &route.path, |component| {
							component.route_path.as_str()
						}) {
						if let Some(controller) = self.components[component_index]
							.controllers
							.get_mut(&client_id)
						{
							controller.set_runtime_context(Some(client_id), session.clone());
							controller.set_route_context(Some(route.clone()));
							let item = controller.render_with_route(&route);
							let title = controller
								.title()
								.or_else(|| controller.route_title(&route.path));
							if let Some(title) = title {
								handle.set_title(client_id, &title).await;
							}
							let mut rendered = item.clone();
							resolve_custom_component_entries(
								&mut rendered,
								&custom_component_entries,
							);
							handle.render(client_id, rendered).await;
							rendered_custom_sync = Some(item);
						}
					}

					// ── #[route] re-render on refresh ─────────────────
					// If no legacy page/component handled the refresh, check
					// whether the client's current route matches a
					// #[route] GET handler and re-render it.
					if rendered_custom_sync.is_none() {
						if let Some(route_match) = self
							.match_route(&route.path, crate::wui::route_handler::HttpMethod::Get)
						{
							let active_route = RouteContext {
								path: route.path.clone(),
								params: route_match.params.0.clone(),
								query: route.query.clone(),
							};
							let result = self
								.dispatch_route(
									route_match,
									crate::wui::route_handler::RouteFormData::default(),
									Some(client_id),
									session.clone(),
									active_route,
								)
								.await;
							if let crate::wui::route_handler::RouteResult::View(view) = result {
								if let Some(title) = &view.title {
									handle.set_title(client_id, title).await;
								}
								let mut rendered = view.item.clone();
								resolve_custom_component_entries(
									&mut rendered,
									&custom_component_entries,
								);
								handle.render(client_id, rendered).await;
								self.sync_custom_components(client_id, &view.item).await;
							}
						}
					}

					if let Some(item) = rendered_custom_sync {
						self.sync_custom_components(client_id, &item).await;
					}
				}
				ClientEvent::Input(_) => {}
				_ => {
					let session = handle.session_for_client(client_id).await;
					if let ClientEvent::OnCustom(custom) = &message.event {
						if self.handle_custom_component_event(client_id, custom).await {
							continue;
						}
					}
					let mut handled = false;
					let mut custom_sync_updates: Vec<(usize, Item)> = Vec::new();
					for component in self.components.iter_mut() {
						if let Some(controller) = component.controllers.get_mut(&client_id) {
							let route = routes
								.get(&client_id)
								.cloned()
								.unwrap_or_else(|| component_route_context("/", &HashMap::new()));
							controller.set_runtime_context(Some(client_id), session.clone());
							controller.set_route_context(Some(route));
							handled = controller.handle(&message.event).await;
						}
						if handled {
							let mut updates: Vec<(usize, Item, Option<String>)> = Vec::new();
							for (mounted_client_id, mounted_controller) in
								component.controllers.iter_mut()
							{
								let mounted_session =
									handle.session_for_client(*mounted_client_id).await;
								let route =
									routes.get(mounted_client_id).cloned().unwrap_or_else(|| {
										component_route_context("/", &HashMap::new())
									});
								mounted_controller
									.set_runtime_context(Some(*mounted_client_id), mounted_session);
								mounted_controller.set_route_context(Some(route.clone()));
								let item = mounted_controller.render_with_route(&route);
								let title = mounted_controller
									.title()
									.or_else(|| mounted_controller.route_title(&route.path));
								updates.push((*mounted_client_id, item, title));
							}

							for (mounted_client_id, item, title) in updates {
								if let Some(title) = title {
									handle.set_title(mounted_client_id, &title).await;
								}
								let mut rendered = item.clone();
								resolve_custom_component_entries(
									&mut rendered,
									&custom_component_entries,
								);
								handle.render(mounted_client_id, rendered).await;
								custom_sync_updates.push((mounted_client_id, item));
							}
							break;
						}
					}
					if handled {
						for (mounted_client_id, item) in custom_sync_updates {
							self.sync_custom_components(mounted_client_id, &item).await;
						}
						continue;
					}
					let mut custom_sync_updates: Vec<(usize, Item)> = Vec::new();
					for page in self.pages.iter_mut() {
						if let Some(controller) = page.controllers.get_mut(&client_id) {
							let route = routes
								.get(&client_id)
								.cloned()
								.unwrap_or_else(|| component_route_context("/", &HashMap::new()));
							controller.set_runtime_context(Some(client_id), session.clone());
							controller.set_route_context(Some(route));
							handled = controller.handle(&message.event).await;
						}
						if handled {
							let mut updates: Vec<(usize, Item, Option<String>)> = Vec::new();
							for (mounted_client_id, mounted_controller) in
								page.controllers.iter_mut()
							{
								let mounted_session =
									handle.session_for_client(*mounted_client_id).await;
								let route =
									routes.get(mounted_client_id).cloned().unwrap_or_else(|| {
										component_route_context("/", &HashMap::new())
									});
								mounted_controller
									.set_runtime_context(Some(*mounted_client_id), mounted_session);
								mounted_controller.set_route_context(Some(route.clone()));
								let item = mounted_controller.render_with_route(&route);
								let title = mounted_controller
									.title()
									.or_else(|| mounted_controller.route_title(&route.path));
								updates.push((*mounted_client_id, item, title));
							}

							for (mounted_client_id, item, title) in updates {
								if let Some(title) = title {
									handle.set_title(mounted_client_id, &title).await;
								}
								let mut rendered = item.clone();
								resolve_custom_component_entries(
									&mut rendered,
									&custom_component_entries,
								);
								handle.render(mounted_client_id, rendered).await;
								custom_sync_updates.push((mounted_client_id, item));
							}
							break;
						}
					}
					for (mounted_client_id, item) in custom_sync_updates {
						self.sync_custom_components(mounted_client_id, &item).await;
					}
				}
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn root_route_is_fallback_match() {
		assert_eq!(component_route_match_score("/", "/"), Some(0));
		assert_eq!(component_route_match_score("/", "/login"), Some(0));
	}

	#[test]
	fn component_route_requires_exact_segment_prefix() {
		assert_eq!(component_route_match_score("/peers", "/peers"), Some(6));
		assert_eq!(component_route_match_score("/peers", "/peers/abc"), Some(6));
		assert_eq!(component_route_match_score("/peers", "/peers-other"), None);
	}

	#[test]
	fn best_matching_route_prefers_specific_route_over_root() {
		let routes = vec!["/".to_string(), "/login".to_string(), "/peers".to_string()];

		assert_eq!(
			best_component_route_index(&routes, "/login", |route| route.as_str()),
			Some(1)
		);
		assert_eq!(
			best_component_route_index(&routes, "/peers/abc", |route| route.as_str()),
			Some(2)
		);
		assert_eq!(
			best_component_route_index(&routes, "/missing", |route| route.as_str()),
			Some(0)
		);
	}

	#[test]
	fn custom_component_asset_comes_from_type_name() {
		struct RobotSceneComponent;

		assert_eq!(
			custom_component_asset_for_type::<RobotSceneComponent>(),
			"robot-scene"
		);
		assert_eq!(
			custom_component_entry_for_asset(
				&custom_component_asset_for_type::<RobotSceneComponent>()
			),
			"/fs/wgui-controllers/robot-scene/controller.js"
		);
	}
}
