use crate::gui::{self, Item};
use crate::wui::ast::{BinaryOp, Expr, Literal, UnaryOp};
use crate::wui::compiler::ir::{ActionDef, ActionPayload, EventKind, IrNode, IrProp, IrWidget};
use crate::wui::diagnostic::Diagnostic;
use crate::wui::imports;
use crate::wui::routing::route_params;

pub use anyhow;
pub use async_trait::async_trait;
use std::collections::HashMap;
use std::fs;
use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct Template {
	doc: crate::wui::compiler::ir::IrDocument,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RouteContext {
	pub path: String,
	pub params: HashMap<String, String>,
	pub query: HashMap<String, String>,
}

pub enum MountResult<C> {
	Ready(C),
	Redirect(String),
}

impl<C> MountResult<C> {
	pub fn ready(controller: C) -> Self {
		Self::Ready(controller)
	}

	pub fn redirect(url: impl Into<String>) -> Self {
		Self::Redirect(url.into())
	}
}

#[async_trait]
pub trait WuiController {
	fn render(&self) -> Item;
	fn render_with_path(&self, path: &str) -> Item {
		let _ = path;
		self.render()
	}
	fn render_with_route(&self, route: &RouteContext) -> Item {
		self.render_with_path(&route.path)
	}
	fn title(&self) -> Option<String> {
		None
	}
	fn route_title(&self, _path: &str) -> Option<String> {
		None
	}
	fn set_runtime_context(&mut self, _client_id: Option<usize>, _session: Option<String>) {}
	fn set_route_context(&mut self, _route: Option<RouteContext>) {}
	#[cfg(feature = "hyper")]
	fn http_routes() -> Vec<crate::HttpRouteSpec>
	where
		Self: Sized,
	{
		Vec::new()
	}
	#[cfg(feature = "hyper")]
	async fn handle_http(
		&mut self,
		_route: &str,
		_request: crate::HttpRequest,
		_ctx: crate::HttpCtx,
	) -> Option<crate::HttpResponse>
	where
		Self: Sized,
	{
		None
	}
	async fn process(_ctx: ControllerProcessCtx) -> anyhow::Result<()>
	where
		Self: Sized,
	{
		Ok(())
	}
	async fn handle(&mut self, event: &crate::types::ClientEvent) -> bool;
}

#[derive(Clone)]
pub struct ControllerProcessCtx {
	client_id: usize,
	event_tx: mpsc::UnboundedSender<crate::types::ClientMessage>,
}

impl ControllerProcessCtx {
	pub(crate) fn new(
		client_id: usize,
		event_tx: mpsc::UnboundedSender<crate::types::ClientMessage>,
	) -> Self {
		Self {
			client_id,
			event_tx,
		}
	}

	pub fn client_id(&self) -> usize {
		self.client_id
	}

	pub fn refresh(&self) {
		let _ = self.event_tx.send(crate::types::ClientMessage {
			client_id: self.client_id,
			event: crate::types::ClientEvent::Refresh,
		});
	}
}

pub struct Ctx<T, DB = ()> {
	pub state: Arc<T>,
	pub db: Arc<DB>,
	current_client: Arc<Mutex<Option<usize>>>,
	current_session: Arc<Mutex<Option<String>>>,
	current_route: Arc<Mutex<Option<RouteContext>>>,
	pubsub: crate::PubSub<()>,
	command_tx: mpsc::UnboundedSender<RuntimeCommand>,
	command_rx: Mutex<Option<mpsc::UnboundedReceiver<RuntimeCommand>>>,
}

impl<T> Ctx<T, ()> {
	pub fn new(state: T) -> Self {
		Self::new_with_db(state, ())
	}
}

impl<T, DB> Ctx<T, DB>
where
	DB: Send + Sync + 'static,
{
	pub fn new_with_db<D>(state: T, db: D) -> Self
	where
		D: Into<Arc<DB>>,
	{
		let (command_tx, command_rx) = mpsc::unbounded_channel::<RuntimeCommand>();
		Self {
			state: Arc::new(state),
			db: db.into(),
			current_client: Arc::new(Mutex::new(None)),
			current_session: Arc::new(Mutex::new(None)),
			current_route: Arc::new(Mutex::new(None)),
			pubsub: crate::PubSub::new(),
			command_tx,
			command_rx: Mutex::new(Some(command_rx)),
		}
	}

	pub fn db(&self) -> &DB {
		self.db.as_ref()
	}

	pub fn spawn<F>(&self, fut: F)
	where
		F: Future + Send + 'static,
		F::Output: Send + 'static,
	{
		tokio::spawn(fut);
	}

	pub fn set_title(&self, title: impl Into<String>) {
		let title = title.into();
		if let Some(client_id) = *self.current_client.lock().unwrap() {
			let _ = self
				.command_tx
				.send(RuntimeCommand::SetTitle { client_id, title });
		}
	}

	pub fn refresh(&self) {
		if let Some(client_id) = *self.current_client.lock().unwrap() {
			self.refresh_for_client(client_id);
		}
	}

	pub fn refresh_for_client(&self, client_id: usize) {
		let _ = self.command_tx.send(RuntimeCommand::Refresh { client_id });
	}

	pub fn push_state(&self, url: impl Into<String>) {
		let url = url.into();
		if let Some(client_id) = *self.current_client.lock().unwrap() {
			self.push_state_for_client(client_id, url);
		}
	}

	pub fn push_state_for_client(&self, client_id: usize, url: impl Into<String>) {
		let url = url.into();
		let _ = self
			.command_tx
			.send(RuntimeCommand::PushState { client_id, url });
	}

	pub fn navigate(&self, url: impl Into<String>) {
		let url = url.into();
		if let Some(client_id) = *self.current_client.lock().unwrap() {
			let _ = self
				.command_tx
				.send(RuntimeCommand::Navigate { client_id, url });
		}
	}

	pub fn enable_web_push(
		&self,
		service_worker_path: impl Into<String>,
		vapid_public_key: Option<String>,
	) {
		let service_worker_path = service_worker_path.into();
		if service_worker_path.trim().is_empty() {
			return;
		}
		if let Some(client_id) = *self.current_client.lock().unwrap() {
			let _ = self.command_tx.send(RuntimeCommand::WebPushEnable {
				client_id,
				service_worker_path,
				vapid_public_key,
			});
		}
	}

	pub fn disable_web_push(&self, service_worker_path: impl Into<String>) {
		let service_worker_path = service_worker_path.into();
		if service_worker_path.trim().is_empty() {
			return;
		}
		if let Some(client_id) = *self.current_client.lock().unwrap() {
			let _ = self.command_tx.send(RuntimeCommand::WebPushDisable {
				client_id,
				service_worker_path,
			});
		}
	}

	pub fn session_id(&self) -> Option<String> {
		self.current_session.lock().unwrap().clone()
	}

	pub fn client_id(&self) -> Option<usize> {
		*self.current_client.lock().unwrap()
	}

	pub fn route(&self) -> Option<RouteContext> {
		self.current_route.lock().unwrap().clone()
	}

	pub fn param(&self, name: &str) -> Option<String> {
		self.current_route
			.lock()
			.unwrap()
			.as_ref()
			.and_then(|route| route.params.get(name).cloned())
	}

	pub fn query(&self, name: &str) -> Option<String> {
		self.current_route
			.lock()
			.unwrap()
			.as_ref()
			.and_then(|route| route.query.get(name).cloned())
	}

	pub fn pubsub(&self) -> crate::PubSub<()> {
		self.pubsub.clone()
	}

	pub(crate) fn set_current_client(&self, client_id: Option<usize>) {
		*self.current_client.lock().unwrap() = client_id;
	}

	pub(crate) fn set_current_session(&self, session: Option<String>) {
		*self.current_session.lock().unwrap() = session;
	}

	pub(crate) fn set_current_route(&self, route: Option<RouteContext>) {
		*self.current_route.lock().unwrap() = route;
	}

	pub(crate) fn take_command_rx(&self) -> mpsc::UnboundedReceiver<RuntimeCommand> {
		self.command_rx
			.lock()
			.unwrap()
			.take()
			.expect("command receiver already taken")
	}
}

#[async_trait]
pub trait Component: Send + Sync + 'static {
	type Context: Send + Sync + 'static;
	type Db: Send + Sync + 'static;
	type Model: WguiModel;

	async fn mount(
		ctx: Arc<Ctx<Self::Context, Self::Db>>,
		route: RouteContext,
	) -> MountResult<Self>
	where
		Self: Sized;
	fn render(&self, ctx: &Ctx<Self::Context, Self::Db>) -> Self::Model;
	async fn process(
		_ctx: Arc<Ctx<Self::Context, Self::Db>>,
		_process_ctx: ControllerProcessCtx,
	) -> anyhow::Result<()>
	where
		Self: Sized,
	{
		Ok(())
	}
	fn unmount(self, ctx: Arc<Ctx<Self::Context, Self::Db>>);
}

#[derive(Debug, Clone)]
pub enum RuntimeAction {
	Click {
		name: String,
		arg: Option<u32>,
	},
	TextChanged {
		name: String,
		value: String,
	},
	SliderChange {
		name: String,
		arg: Option<u32>,
		value: i32,
	},
	Select {
		name: String,
		value: String,
	},
	Custom {
		name: String,
		event: String,
		payload: serde_json::Value,
	},
}

#[derive(Debug, Clone)]
pub enum RuntimeCommand {
	Refresh {
		client_id: usize,
	},
	SetTitle {
		client_id: usize,
		title: String,
	},
	PushState {
		client_id: usize,
		url: String,
	},
	Navigate {
		client_id: usize,
		url: String,
	},
	WebPushEnable {
		client_id: usize,
		service_worker_path: String,
		vapid_public_key: Option<String>,
	},
	WebPushDisable {
		client_id: usize,
		service_worker_path: String,
	},
}

#[derive(Debug, Clone)]
pub enum WuiValue {
	String(String),
	Number(f64),
	Bool(bool),
	Null,
	List(Vec<WuiValue>),
	Object(HashMap<String, WuiValue>),
}

pub trait WuiValueProvider {
	fn wui_value(&self) -> WuiValue;
}

pub trait WuiValueConvert {
	fn to_wui_value(&self) -> WuiValue;
}

#[derive(Debug, Clone)]
pub struct WdbFieldSchema {
	pub name: &'static str,
	pub rust_type: &'static str,
}

#[derive(Debug, Clone)]
pub struct WdbModelSchema {
	pub model: &'static str,
	pub fields: Vec<WdbFieldSchema>,
}

pub trait WdbModel {
	fn schema() -> WdbModelSchema;
}

pub trait WdbSchema {
	fn schema() -> Vec<WdbModelSchema>;
}

pub trait WguiModel: WuiValueConvert {
	fn find_id(_id: &str) {}
}

impl<T: WuiValueConvert + ?Sized> WguiModel for T {}

impl<T: WuiValueConvert + ?Sized> WuiValueProvider for T {
	fn wui_value(&self) -> WuiValue {
		self.to_wui_value()
	}
}

impl WuiValueConvert for WuiValue {
	fn to_wui_value(&self) -> WuiValue {
		self.clone()
	}
}

impl WuiValueConvert for String {
	fn to_wui_value(&self) -> WuiValue {
		WuiValue::String(self.clone())
	}
}

impl WuiValueConvert for &str {
	fn to_wui_value(&self) -> WuiValue {
		WuiValue::String(self.to_string())
	}
}

impl WuiValueConvert for bool {
	fn to_wui_value(&self) -> WuiValue {
		WuiValue::Bool(*self)
	}
}

impl WuiValueConvert for u32 {
	fn to_wui_value(&self) -> WuiValue {
		WuiValue::Number(*self as f64)
	}
}

impl WuiValueConvert for i32 {
	fn to_wui_value(&self) -> WuiValue {
		WuiValue::Number(*self as f64)
	}
}

impl WuiValueConvert for usize {
	fn to_wui_value(&self) -> WuiValue {
		WuiValue::Number(*self as f64)
	}
}

impl WuiValueConvert for f32 {
	fn to_wui_value(&self) -> WuiValue {
		WuiValue::Number(*self as f64)
	}
}

impl WuiValueConvert for f64 {
	fn to_wui_value(&self) -> WuiValue {
		WuiValue::Number(*self)
	}
}

impl<T: WuiValueConvert> WuiValueConvert for Vec<T> {
	fn to_wui_value(&self) -> WuiValue {
		WuiValue::List(self.iter().map(|item| item.to_wui_value()).collect())
	}
}

impl<T: WuiValueConvert> WuiValueConvert for Option<T> {
	fn to_wui_value(&self) -> WuiValue {
		match self {
			Some(value) => value.to_wui_value(),
			None => WuiValue::Null,
		}
	}
}

#[derive(Debug)]
pub enum TemplateLoadError {
	Io(std::io::Error),
	Diagnostics(Vec<Diagnostic>),
}

impl Template {
	pub fn parse(source: &str, module_name: &str) -> Result<Self, Vec<Diagnostic>> {
		Self::parse_with_dir(source, module_name, None)
	}

	pub fn parse_with_dir(
		source: &str,
		module_name: &str,
		base_dir: Option<&Path>,
	) -> Result<Self, Vec<Diagnostic>> {
		let resolved = imports::resolve(source, module_name, base_dir)?;
		Self::from_resolved(resolved, module_name)
	}

	pub fn parse_with_loader<F>(
		source: &str,
		module_name: &str,
		base_dir: Option<&Path>,
		loader: F,
	) -> Result<Self, Vec<Diagnostic>>
	where
		F: FnMut(&Path) -> io::Result<String>,
	{
		let resolved = imports::resolve_with_loader(source, module_name, base_dir, loader)?;
		Self::from_resolved(resolved, module_name)
	}

	pub fn parse_with_sources(
		source: &str,
		module_name: &str,
		base_dir: Option<&Path>,
		sources: &[(&str, &str)],
	) -> Result<Self, Vec<Diagnostic>> {
		let sources = sources
			.iter()
			.map(|(path, source)| (normalize_embedded_path(Path::new(path)), *source))
			.collect::<HashMap<_, _>>();
		Self::parse_with_loader(source, module_name, base_dir, move |path| {
			let path = normalize_embedded_path(path);
			sources
				.get(&path)
				.map(|source| (*source).to_string())
				.ok_or_else(|| {
					io::Error::new(
						io::ErrorKind::NotFound,
						format!("embedded WUI source {} not found", path.display()),
					)
				})
		})
	}

	fn from_resolved(
		resolved: crate::wui::imports::ImportResult,
		module_name: &str,
	) -> Result<Self, Vec<Diagnostic>> {
		let mut diags = Vec::new();
		let validated = crate::wui::compiler::validate::validate(
			&resolved.nodes,
			&resolved.components,
			&mut diags,
		);
		let Some(validated) = validated else {
			return Err(diags);
		};
		let lowered = crate::wui::compiler::lower::lower(&validated, module_name, &mut diags);
		if !diags.is_empty() {
			return Err(diags);
		}
		Ok(Self { doc: lowered })
	}

	pub fn render<T: WuiValueProvider>(&self, state: &T) -> Item {
		self.render_with_path(state, "")
	}

	pub fn render_with_path<T: WuiValueProvider>(&self, state: &T, path: &str) -> Item {
		let route = RouteContext {
			path: path.to_string(),
			params: HashMap::new(),
			query: HashMap::new(),
		};
		self.render_with_route(state, &route)
	}

	pub fn render_with_route<T: WuiValueProvider>(&self, state: &T, route: &RouteContext) -> Item {
		let mut ctx = EvalContext::new(state.wui_value(), route, &self.doc.components);
		let mut children = Vec::new();
		render_nodes(&self.doc.nodes, &mut children, &mut ctx);
		single_or_wrapped(children)
	}

	pub fn title_for_path(&self, path: &str) -> Option<String> {
		for page in &self.doc.pages {
			let Some(route) = &page.route else {
				continue;
			};
			if route_params(route, path).is_some() {
				return page.title.clone();
			}
		}
		None
	}

	pub fn decode(&self, event: &crate::types::ClientEvent) -> Option<RuntimeAction> {
		for action in &self.doc.actions {
			if let Some(decoded) = decode_action(action, event) {
				return Some(decoded);
			}
		}
		None
	}
}

pub fn load_template(path: &Path, module_name: &str) -> Result<Template, TemplateLoadError> {
	let source = fs::read_to_string(path).map_err(TemplateLoadError::Io)?;
	Template::parse_with_dir(&source, module_name, path.parent())
		.map_err(TemplateLoadError::Diagnostics)
}

pub fn spawn_template_watcher(
	path: PathBuf,
	tx: mpsc::UnboundedSender<()>,
) -> thread::JoinHandle<()> {
	thread::spawn(move || {
		let mut last_mtime = file_mtime(&path);
		loop {
			thread::sleep(Duration::from_millis(250));
			let mtime = file_mtime(&path);
			if mtime > last_mtime {
				last_mtime = mtime;
				let _ = tx.send(());
			}
		}
	})
}

fn file_mtime(path: &Path) -> SystemTime {
	fs::metadata(path)
		.and_then(|meta| meta.modified())
		.unwrap_or(SystemTime::UNIX_EPOCH)
}

fn normalize_embedded_path(path: &Path) -> PathBuf {
	let mut out = PathBuf::new();
	for component in path.components() {
		match component {
			std::path::Component::CurDir => {}
			std::path::Component::ParentDir => {
				out.pop();
			}
			_ => out.push(component.as_os_str()),
		}
	}
	out
}

impl WuiValue {
	pub fn object(entries: Vec<(String, WuiValue)>) -> Self {
		let mut map = HashMap::new();
		for (k, v) in entries {
			map.insert(k, v);
		}
		WuiValue::Object(map)
	}
}

struct EvalContext {
	vars: HashMap<String, WuiValue>,
	components: HashMap<String, crate::wui::compiler::ir::IrComponent>,
	children: Vec<IrNode>,
}

impl EvalContext {
	fn new(
		state: WuiValue,
		route: &RouteContext,
		components: &HashMap<String, crate::wui::compiler::ir::IrComponent>,
	) -> Self {
		let mut vars = HashMap::new();
		vars.insert("state".to_string(), state);
		vars.insert("path".to_string(), WuiValue::String(route.path.clone()));
		vars.insert("params".to_string(), string_map_to_wui_value(&route.params));
		vars.insert("query".to_string(), string_map_to_wui_value(&route.query));
		Self {
			vars,
			components: components.clone(),
			children: Vec::new(),
		}
	}

	fn with_var(&self, name: &str, value: WuiValue) -> Self {
		let mut vars = self.vars.clone();
		vars.insert(name.to_string(), value);
		Self {
			vars,
			components: self.components.clone(),
			children: self.children.clone(),
		}
	}
}

fn string_map_to_wui_value(map: &HashMap<String, String>) -> WuiValue {
	WuiValue::object(
		map.iter()
			.map(|(key, value)| (key.clone(), WuiValue::String(value.clone())))
			.collect(),
	)
}

fn decode_action(action: &ActionDef, event: &crate::types::ClientEvent) -> Option<RuntimeAction> {
	match &action.kind {
		EventKind::Click | EventKind::Press | EventKind::Release | EventKind::Repeat => match event
		{
			crate::types::ClientEvent::OnClick(ev) if ev.id == action.id => match action.payload {
				ActionPayload::None => Some(RuntimeAction::Click {
					name: action.name.clone(),
					arg: None,
				}),
				ActionPayload::U32 => ev.inx.map(|arg| RuntimeAction::Click {
					name: action.name.clone(),
					arg: Some(arg),
				}),
				_ => None,
			},
			crate::types::ClientEvent::OnPress(ev) if ev.id == action.id => match action.payload {
				ActionPayload::None => Some(RuntimeAction::Click {
					name: action.name.clone(),
					arg: None,
				}),
				ActionPayload::U32 => ev.inx.map(|arg| RuntimeAction::Click {
					name: action.name.clone(),
					arg: Some(arg),
				}),
				_ => None,
			},
			crate::types::ClientEvent::OnRelease(ev) if ev.id == action.id => {
				match action.payload {
					ActionPayload::None => Some(RuntimeAction::Click {
						name: action.name.clone(),
						arg: None,
					}),
					ActionPayload::U32 => ev.inx.map(|arg| RuntimeAction::Click {
						name: action.name.clone(),
						arg: Some(arg),
					}),
					_ => None,
				}
			}
			crate::types::ClientEvent::OnRepeat(ev) if ev.id == action.id => match action.payload {
				ActionPayload::None => Some(RuntimeAction::Click {
					name: action.name.clone(),
					arg: None,
				}),
				ActionPayload::U32 => ev.inx.map(|arg| RuntimeAction::Click {
					name: action.name.clone(),
					arg: Some(arg),
				}),
				_ => None,
			},
			_ => None,
		},
		EventKind::TextChanged => match event {
			crate::types::ClientEvent::OnTextChanged(ev) if ev.id == action.id => {
				Some(RuntimeAction::TextChanged {
					name: action.name.clone(),
					value: ev.value.clone(),
				})
			}
			_ => None,
		},
		EventKind::SliderChange => match event {
			crate::types::ClientEvent::OnSliderChange(ev) if ev.id == action.id => {
				match action.payload {
					ActionPayload::I32 => Some(RuntimeAction::SliderChange {
						name: action.name.clone(),
						arg: None,
						value: ev.value,
					}),
					ActionPayload::U32I32 => ev.inx.map(|arg| RuntimeAction::SliderChange {
						name: action.name.clone(),
						arg: Some(arg),
						value: ev.value,
					}),
					_ => None,
				}
			}
			_ => None,
		},
		EventKind::Select => match event {
			crate::types::ClientEvent::OnSelect(ev) if ev.id == action.id => {
				Some(RuntimeAction::Select {
					name: action.name.clone(),
					value: ev.value.clone(),
				})
			}
			_ => None,
		},
		EventKind::Custom(event_name) => match event {
			crate::types::ClientEvent::OnCustom(ev)
				if ev.id == action.id && ev.name == *event_name =>
			{
				Some(RuntimeAction::Custom {
					name: action.name.clone(),
					event: ev.name.clone(),
					payload: ev.payload.clone(),
				})
			}
			_ => None,
		},
	}
}

fn render_nodes(nodes: &[IrNode], out: &mut Vec<Item>, ctx: &mut EvalContext) {
	for node in nodes {
		match node {
			IrNode::Widget(widget) => out.push(render_widget(widget, ctx)),
			IrNode::Children => {
				let children = ctx.children.clone();
				render_nodes(&children, out, ctx);
			}
			IrNode::Text(text) => out.push(gui::text(text)),
			IrNode::For(node) => {
				let list_value = eval_expr(&node.each, ctx);
				let WuiValue::List(items) = list_value else {
					continue;
				};
				for (inx, item) in items.into_iter().enumerate() {
					let mut nested = ctx.with_var(&node.item, item);
					if let Some(index) = &node.index {
						nested = nested.with_var(index, WuiValue::Number(inx as f64));
					}
					render_nodes(&node.body, out, &mut nested);
				}
			}
			IrNode::If(node) => {
				let test = eval_expr(&node.test, ctx);
				if value_as_bool(&test) {
					render_nodes(&node.then_body, out, ctx);
				} else {
					render_nodes(&node.else_body, out, ctx);
				}
			}
			IrNode::Scope(node) => {
				render_nodes(&node.body, out, ctx);
			}
			IrNode::Route(node) => {
				let path = ctx
					.vars
					.get("path")
					.map(value_as_string)
					.unwrap_or_else(String::new);
				if let Some(params) = route_params(&node.path, &path) {
					let params = WuiValue::object(
						params
							.into_iter()
							.map(|(k, v)| (k, WuiValue::String(v)))
							.collect(),
					);
					let mut nested = ctx.with_var("params", params);
					render_nodes(&node.body, out, &mut nested);
				}
			}
			IrNode::Switch(node) => {
				let path = ctx
					.vars
					.get("path")
					.map(value_as_string)
					.unwrap_or_else(String::new);
				for case in &node.cases {
					if let Some(params) = route_params(&case.path, &path) {
						let params = WuiValue::object(
							params
								.into_iter()
								.map(|(k, v)| (k, WuiValue::String(v)))
								.collect(),
						);
						let mut nested = ctx.with_var("params", params);
						render_nodes(&case.body, out, &mut nested);
						break;
					}
				}
			}
		}
	}
}

fn render_widget(widget: &IrWidget, ctx: &mut EvalContext) -> Item {
	if ctx.components.contains_key(&widget.tag) {
		let mut item = single_or_wrapped(render_component(widget, ctx));
		for prop in &widget.props {
			if matches!(prop, IrProp::Event { .. }) {
				item = apply_prop(item, prop, ctx);
			}
		}
		return item;
	}
	let mut base = match widget.tag.as_str() {
		"VStack" => render_container(gui::vstack, &widget.children, ctx),
		"HStack" => render_container(gui::hstack, &widget.children, ctx),
		"Form" => render_container(gui::form, &widget.children, ctx),
		"Text" => gui::text(&text_value(widget, ctx)),
		"Button" => gui::button(&textual_value(widget, ctx, "text")),
		"Link" => gui::link(
			&textual_value(widget, ctx, "href"),
			&textual_value(widget, ctx, "text"),
		),
		"TextInput" => gui::text_input(),
		"DatePicker" => gui::date_picker(),
		"Checkbox" => gui::checkbox(),
		"Slider" => gui::slider(),
		"Select" => render_select(widget, ctx),
		"Custom" | "CustomComponent" => render_custom(widget, ctx),
		"Image" => {
			let (src, alt) = image_values(widget, ctx);
			gui::img(&src, &alt)
		}
		"Video" => {
			let room = media_room_value(widget, ctx);
			gui::video(&room)
		}
		"Audio" => {
			let room = media_room_value(widget, ctx);
			gui::audio(&room)
		}
		"FolderPicker" => gui::folder_picker(),
		"Modal" => render_modal(widget, ctx),
		_ => gui::text("unsupported"),
	};

	for prop in &widget.props {
		if !should_apply_prop(&widget.tag, prop) {
			continue;
		}
		base = apply_prop(base, prop, ctx);
	}

	base
}

fn render_custom(widget: &IrWidget, ctx: &mut EvalContext) -> Item {
	let src = textual_value(widget, ctx, "src");
	let (name, entry) = if src.is_empty() {
		(
			textual_value(widget, ctx, "name"),
			textual_value(widget, ctx, "entry"),
		)
	} else {
		(src.clone(), textual_value(widget, ctx, "entry"))
	};
	let props = widget
		.props
		.iter()
		.find_map(|prop| match prop {
			IrProp::Value { name, expr } if name == "props" => {
				Some(wui_value_to_json(&eval_expr(expr, ctx)))
			}
			IrProp::Literal { name, value } if name == "props" => {
				Some(serde_json::Value::String(value.clone()))
			}
			IrProp::Number { name, value } if name == "props" => Some(serde_json::json!(*value)),
			IrProp::Bool { name, value } if name == "props" => Some(serde_json::json!(*value)),
			_ => None,
		})
		.unwrap_or_else(|| serde_json::json!({}));

	gui::custom_component(name, entry, props)
}

fn wui_value_to_json(value: &WuiValue) -> serde_json::Value {
	match value {
		WuiValue::String(value) => serde_json::Value::String(value.clone()),
		WuiValue::Number(value) => serde_json::Number::from_f64(*value)
			.map(serde_json::Value::Number)
			.unwrap_or(serde_json::Value::Null),
		WuiValue::Bool(value) => serde_json::Value::Bool(*value),
		WuiValue::Null => serde_json::Value::Null,
		WuiValue::List(values) => {
			serde_json::Value::Array(values.iter().map(wui_value_to_json).collect())
		}
		WuiValue::Object(values) => serde_json::Value::Object(
			values
				.iter()
				.map(|(key, value)| (key.clone(), wui_value_to_json(value)))
				.collect(),
		),
	}
}

fn render_component(widget: &IrWidget, ctx: &mut EvalContext) -> Vec<Item> {
	let Some(component) = ctx.components.get(&widget.tag).cloned() else {
		return vec![gui::text("unsupported")];
	};
	let mut vars = ctx.vars.clone();
	for prop in &widget.props {
		let (name, value) = prop_value(prop, ctx);
		vars.insert(name, value);
	}
	let mut nested = EvalContext {
		vars,
		components: ctx.components.clone(),
		children: widget.children.clone(),
	};
	let mut items = Vec::new();
	render_nodes(&component.body, &mut items, &mut nested);
	items
}

fn single_or_wrapped(mut items: Vec<Item>) -> Item {
	if items.len() == 1 {
		items.remove(0)
	} else {
		gui::vstack(items).fill(true)
	}
}

fn prop_value(prop: &IrProp, ctx: &mut EvalContext) -> (String, WuiValue) {
	match prop {
		IrProp::Literal { name, value } => (name.clone(), WuiValue::String(value.clone())),
		IrProp::Number { name, value } => (name.clone(), WuiValue::Number(*value)),
		IrProp::Bool { name, value } => (name.clone(), WuiValue::Bool(*value)),
		IrProp::Value { name, expr } => (name.clone(), eval_expr(expr, ctx)),
		IrProp::Bind { name, expr } => (name.clone(), eval_expr(expr, ctx)),
		IrProp::Event { name, .. } => (name.clone(), WuiValue::Null),
	}
}

fn render_container<F>(builder: F, children: &[IrNode], ctx: &mut EvalContext) -> Item
where
	F: Fn(Vec<Item>) -> Item,
{
	let mut items = Vec::new();
	render_nodes(children, &mut items, ctx);
	builder(items)
}

fn render_modal(widget: &IrWidget, ctx: &mut EvalContext) -> Item {
	let mut items = Vec::new();
	render_nodes(&widget.children, &mut items, ctx);
	gui::modal(items)
}

fn text_value(widget: &IrWidget, ctx: &mut EvalContext) -> String {
	for prop in &widget.props {
		match prop {
			IrProp::Literal { name, value } if name == "value" => return value.clone(),
			IrProp::Value { name, expr } if name == "value" => {
				return value_as_string(&eval_expr(expr, ctx));
			}
			_ => {}
		}
	}
	String::new()
}

fn textual_value(widget: &IrWidget, ctx: &mut EvalContext, prop_name: &str) -> String {
	for prop in &widget.props {
		match prop {
			IrProp::Literal { name, value } if name == prop_name => return value.clone(),
			IrProp::Value { name, expr } if name == prop_name => {
				return value_as_string(&eval_expr(expr, ctx));
			}
			_ => {}
		}
	}
	String::new()
}

fn image_values(widget: &IrWidget, ctx: &mut EvalContext) -> (String, String) {
	let mut src = String::new();
	let mut alt = String::new();
	for prop in &widget.props {
		match prop {
			IrProp::Literal { name, value } if name == "src" => src = value.clone(),
			IrProp::Value { name, expr } if name == "src" => {
				src = value_as_string(&eval_expr(expr, ctx));
			}
			IrProp::Literal { name, value } if name == "alt" => alt = value.clone(),
			IrProp::Value { name, expr } if name == "alt" => {
				alt = value_as_string(&eval_expr(expr, ctx));
			}
			_ => {}
		}
	}
	(src, alt)
}

fn media_room_value(widget: &IrWidget, ctx: &mut EvalContext) -> String {
	for prop in &widget.props {
		match prop {
			IrProp::Literal { name, value } if name == "room" => return value.clone(),
			IrProp::Value { name, expr } if name == "room" => {
				return value_as_string(&eval_expr(expr, ctx));
			}
			_ => {}
		}
	}
	String::new()
}

fn select_option_from_value(value: WuiValue) -> Option<gui::SelectOption> {
	let WuiValue::Object(mut map) = value else {
		return None;
	};
	let value = value_as_string(&map.remove("value").unwrap_or(WuiValue::Null));
	let name = value_as_string(&map.remove("name").unwrap_or(WuiValue::Null));
	if value.is_empty() {
		return None;
	}
	Some(gui::option(
		&value,
		if name.is_empty() { &value } else { &name },
	))
}

fn select_options(widget: &IrWidget, ctx: &mut EvalContext) -> Vec<gui::SelectOption> {
	for prop in &widget.props {
		match prop {
			IrProp::Value { name, expr } if name == "options" => {
				let WuiValue::List(values) = eval_expr(expr, ctx) else {
					return Vec::new();
				};
				return values
					.into_iter()
					.filter_map(select_option_from_value)
					.collect();
			}
			_ => {}
		}
	}
	Vec::new()
}

fn render_select(widget: &IrWidget, ctx: &mut EvalContext) -> Item {
	let mut item = gui::select(select_options(widget, ctx));
	for prop in &widget.props {
		match prop {
			IrProp::Literal { name, value } if name == "value" => {
				item = item.svalue(value);
			}
			IrProp::Value { name, expr } if name == "value" => {
				item = item.svalue(&value_as_string(&eval_expr(expr, ctx)));
			}
			IrProp::Bind { name, expr } if name == "bind:value" => {
				item = item.svalue(&value_as_string(&eval_expr(expr, ctx)));
			}
			_ => {}
		}
	}
	item
}

fn should_apply_prop(tag: &str, prop: &IrProp) -> bool {
	match prop {
		IrProp::Event { .. } => true,
		IrProp::Literal { name, .. }
		| IrProp::Number { name, .. }
		| IrProp::Bool { name, .. }
		| IrProp::Value { name, .. }
		| IrProp::Bind { name, .. } => match tag {
			"Text" => name != "value",
			"Button" => name != "text",
			"Link" => name != "href" && name != "text",
			"Image" => name != "src" && name != "alt",
			"Video" | "Audio" => name != "room",
			"Select" => name != "value" && name != "bind:value" && name != "options",
			_ => true,
		},
	}
}

fn apply_prop(item: Item, prop: &IrProp, ctx: &mut EvalContext) -> Item {
	match prop {
		IrProp::Event { name, action, arg } => {
			let id = action_id(action);
			let mut item = if matches!(&item.payload, gui::ItemPayload::Button { .. }) {
				apply_button_event(item, name, id)
			} else if matches!(&item.payload, gui::ItemPayload::Custom { .. }) {
				item.custom_event(name, id)
			} else {
				item.id(id)
			};
			if let Some(expr) = arg {
				let value = eval_expr(expr, ctx);
				if let Some(inx) = value_as_u32(&value) {
					item = item.inx(inx);
				}
			}
			item
		}
		IrProp::Literal { name, value } => apply_string_prop(item, name, value),
		IrProp::Number { name, value } => apply_number_prop(item, name, *value),
		IrProp::Bool { name, value } => apply_bool_prop(item, name, *value),
		IrProp::Value { name, expr } => {
			let value = eval_expr(expr, ctx);
			apply_value_prop(item, name, value)
		}
		IrProp::Bind { name, expr } => {
			let value = eval_expr(expr, ctx);
			apply_value_prop(item, name, value)
		}
	}
}

fn apply_button_event(mut item: Item, name: &str, id: u32) -> Item {
	let gui::ItemPayload::Button { events, .. } = &mut item.payload else {
		return item;
	};
	let events = events.get_or_insert_with(gui::ButtonEvents::default);
	match name {
		"onClick" => events.click = Some(id),
		"onPress" => events.press = Some(id),
		"onRelease" => events.release = Some(id),
		"onRepeat" => events.repeat = Some(id),
		_ => {}
	}
	item
}

fn apply_value_prop(item: Item, name: &str, value: WuiValue) -> Item {
	if is_string_prop(name) {
		return apply_string_prop(item, name, &value_as_string(&value));
	}
	match value {
		WuiValue::Number(n) => apply_number_prop(item, name, n),
		WuiValue::Bool(b) => apply_bool_prop(item, name, b),
		WuiValue::String(s) => apply_string_prop(item, name, &s),
		_ => item,
	}
}

fn apply_string_prop(item: Item, name: &str, value: &str) -> Item {
	match name {
		"svalue" | "bind:svalue" => item.svalue(value),
		"name" => item.name(value),
		"action" => item.action(value),
		"method" => item.method(value),
		"placeholder" => item.placeholder(value),
		"type" => item.input_type(value),
		"textAlign" => item.text_align(value),
		"cursor" => item.cursor(value),
		"overflow" => item.overflow(value),
		"backgroundColor" => item.background_color(value),
		"color" => item.color(value),
		"border" => item.border(value),
		"objectFit" => item.object_fit(value),
		"room" => item.room(value),
		_ => item,
	}
}

fn apply_number_prop(item: Item, name: &str, value: f64) -> Item {
	match name {
		"id" => item.id(value as u32),
		"ivalue" | "bind:ivalue" => item.ivalue(value as i32),
		"min" => item.min(value as i32),
		"max" => item.max(value as i32),
		"step" => item.step(value as i32),
		"spacing" => item.spacing(value as u32),
		"padding" => item.padding(value as u16),
		"paddingLeft" => item.padding_left(value as u16),
		"paddingRight" => item.padding_right(value as u16),
		"paddingTop" => item.padding_top(value as u16),
		"paddingBottom" => item.padding_bottom(value as u16),
		"margin" => item.margin(value as u16),
		"marginLeft" => item.margin_left(value as u16),
		"marginRight" => item.margin_right(value as u16),
		"marginTop" => item.margin_top(value as u16),
		"marginBottom" => item.margin_bottom(value as u16),
		"width" => item.width(value as u32),
		"height" => item.height(value as u32),
		"minWidth" => item.min_width(value as u32),
		"maxWidth" => item.max_width(value as u32),
		"minHeight" => item.min_height(value as u32),
		"maxHeight" => item.max_height(value as u32),
		"grow" => item.grow(value as u32),
		"repeatInterval" => apply_button_repeat_interval(item, value as u32),
		_ => item,
	}
}

fn apply_button_repeat_interval(mut item: Item, interval: u32) -> Item {
	let gui::ItemPayload::Button { events, .. } = &mut item.payload else {
		return item;
	};
	events
		.get_or_insert_with(gui::ButtonEvents::default)
		.repeat_interval = Some(interval);
	item
}

fn apply_bool_prop(item: Item, name: &str, value: bool) -> Item {
	match name {
		"checked" | "bind:checked" => item.checked(value),
		"breakWords" => item.break_words(value),
		"fill" => item.fill(value),
		"wrap" => item.wrap(value),
		"open" => item.open(value),
		"hresize" => item.hresize(value),
		"vresize" => item.vresize(value),
		"local" => item.local(value),
		"autoplay" => item.autoplay(value),
		"muted" => item.muted(value),
		"controls" => item.controls(value),
		_ => item,
	}
}

fn is_string_prop(name: &str) -> bool {
	matches!(
		name,
		"svalue"
			| "bind:svalue"
			| "placeholder"
			| "type" | "textAlign"
			| "cursor"
			| "overflow"
			| "backgroundColor"
			| "color" | "border"
			| "objectFit"
			| "room"
	)
}

fn eval_expr(expr: &Expr, ctx: &EvalContext) -> WuiValue {
	match expr {
		Expr::Literal(lit, _) => match lit {
			Literal::String(s) => WuiValue::String(s.clone()),
			Literal::Number(n) => WuiValue::Number(*n),
			Literal::Bool(b) => WuiValue::Bool(*b),
			Literal::Null => WuiValue::Null,
		},
		Expr::Path(parts, _) => resolve_path(parts, ctx),
		Expr::Call { name, args, .. } => eval_call(name, args, ctx),
		Expr::Unary { op, expr, .. } => match op {
			UnaryOp::Not => WuiValue::Bool(!value_as_bool(&eval_expr(expr, ctx))),
			UnaryOp::Neg => WuiValue::Number(-value_as_number(&eval_expr(expr, ctx))),
		},
		Expr::Binary {
			left, op, right, ..
		} => {
			let left = eval_expr(left, ctx);
			let right = eval_expr(right, ctx);
			match op {
				BinaryOp::Add => WuiValue::Number(value_as_number(&left) + value_as_number(&right)),
				BinaryOp::Sub => WuiValue::Number(value_as_number(&left) - value_as_number(&right)),
				BinaryOp::Mul => WuiValue::Number(value_as_number(&left) * value_as_number(&right)),
				BinaryOp::Div => WuiValue::Number(value_as_number(&left) / value_as_number(&right)),
				BinaryOp::Mod => WuiValue::Number(value_as_number(&left) % value_as_number(&right)),
				BinaryOp::Eq => WuiValue::Bool(values_equal(&left, &right)),
				BinaryOp::Neq => WuiValue::Bool(!values_equal(&left, &right)),
				BinaryOp::Lt => WuiValue::Bool(value_as_number(&left) < value_as_number(&right)),
				BinaryOp::Lte => WuiValue::Bool(value_as_number(&left) <= value_as_number(&right)),
				BinaryOp::Gt => WuiValue::Bool(value_as_number(&left) > value_as_number(&right)),
				BinaryOp::Gte => WuiValue::Bool(value_as_number(&left) >= value_as_number(&right)),
				BinaryOp::And => WuiValue::Bool(value_as_bool(&left) && value_as_bool(&right)),
				BinaryOp::Or => WuiValue::Bool(value_as_bool(&left) || value_as_bool(&right)),
			}
		}
		Expr::Ternary {
			cond,
			then_expr,
			else_expr,
			..
		} => {
			if value_as_bool(&eval_expr(cond, ctx)) {
				eval_expr(then_expr, ctx)
			} else {
				eval_expr(else_expr, ctx)
			}
		}
		Expr::Coalesce { left, right, .. } => {
			let left_value = eval_expr(left, ctx);
			if matches!(left_value, WuiValue::Null) {
				eval_expr(right, ctx)
			} else {
				left_value
			}
		}
	}
}

fn eval_call(name: &str, args: &[Expr], ctx: &EvalContext) -> WuiValue {
	match name {
		"path_matches" => eval_path_matches(args, ctx),
		_ => WuiValue::Null,
	}
}

fn eval_path_matches(args: &[Expr], ctx: &EvalContext) -> WuiValue {
	let Some(pattern) = args
		.first()
		.map(|arg| value_as_string(&eval_expr(arg, ctx)))
	else {
		return WuiValue::Bool(false);
	};
	let path = ctx
		.vars
		.get("path")
		.map(value_as_string)
		.unwrap_or_else(String::new);
	WuiValue::Bool(route_params(&pattern, &path).is_some())
}

fn resolve_path(parts: &[String], ctx: &EvalContext) -> WuiValue {
	let Some((first, rest)) = parts.split_first() else {
		return WuiValue::Null;
	};
	let Some(mut current) = ctx.vars.get(first).cloned() else {
		return WuiValue::Null;
	};
	for part in rest {
		current = match current {
			WuiValue::Object(mut map) => map.remove(part).unwrap_or(WuiValue::Null),
			_ => WuiValue::Null,
		};
	}
	current
}

fn value_as_bool(value: &WuiValue) -> bool {
	match value {
		WuiValue::Bool(b) => *b,
		_ => false,
	}
}

fn value_as_number(value: &WuiValue) -> f64 {
	match value {
		WuiValue::Number(n) => *n,
		_ => 0.0,
	}
}

fn value_as_u32(value: &WuiValue) -> Option<u32> {
	match value {
		WuiValue::Number(n) if *n >= 0.0 => Some(*n as u32),
		_ => None,
	}
}

fn value_as_string(value: &WuiValue) -> String {
	match value {
		WuiValue::String(s) => s.clone(),
		WuiValue::Number(n) => n.to_string(),
		WuiValue::Bool(b) => b.to_string(),
		_ => String::new(),
	}
}

fn values_equal(left: &WuiValue, right: &WuiValue) -> bool {
	match (left, right) {
		(WuiValue::String(a), WuiValue::String(b)) => a == b,
		(WuiValue::Number(a), WuiValue::Number(b)) => (a - b).abs() < f64::EPSILON,
		(WuiValue::Bool(a), WuiValue::Bool(b)) => a == b,
		(WuiValue::Null, WuiValue::Null) => true,
		_ => false,
	}
}

fn action_id(name: &str) -> u32 {
	let mut hash = 0x811c9dc5u32;
	for byte in name.as_bytes() {
		hash ^= *byte as u32;
		hash = hash.wrapping_mul(0x01000193);
	}
	if hash == 0 {
		1
	} else {
		hash
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::gui::{Item, ItemPayload};

	fn text_values(item: &Item, out: &mut Vec<String>) {
		match &item.payload {
			ItemPayload::Text { value } => out.push(value.clone()),
			ItemPayload::Layout(layout) => {
				for child in &layout.body {
					text_values(child, out);
				}
			}
			_ => {}
		}
	}

	#[test]
	fn ctx_exposes_current_route_params_and_query() {
		let ctx = Ctx::new(());
		ctx.set_current_route(Some(RouteContext {
			path: "/posts/123".to_string(),
			params: HashMap::from([("post_id".to_string(), "123".to_string())]),
			query: HashMap::from([("tab".to_string(), "comments".to_string())]),
		}));

		assert_eq!(ctx.param("post_id"), Some("123".to_string()));
		assert_eq!(ctx.query("tab"), Some("comments".to_string()));
		assert_eq!(
			ctx.route().map(|route| route.path),
			Some("/posts/123".to_string())
		);
	}

	#[test]
	fn template_eval_context_includes_route_params_query_and_path() {
		let template = Template::parse(
			r#"
			<Text value={path} />
			<Text value={params.post_id} />
			<Text value={query.tab} />
			<Text value={state.title} />
			"#,
			"test",
		)
		.expect("parse template");
		let state = WuiValue::object(vec![(
			"title".to_string(),
			WuiValue::String("Hello".to_string()),
		)]);
		let route = RouteContext {
			path: "/posts/123".to_string(),
			params: HashMap::from([("post_id".to_string(), "123".to_string())]),
			query: HashMap::from([("tab".to_string(), "comments".to_string())]),
		};
		let rendered = template.render_with_route(&state, &route);
		let mut values = Vec::new();
		text_values(&rendered, &mut values);

		assert_eq!(
			values,
			vec![
				"/posts/123".to_string(),
				"123".to_string(),
				"comments".to_string(),
				"Hello".to_string(),
			]
		);
	}

	#[test]
	fn single_root_template_renders_directly() {
		let template =
			Template::parse(r#"<Text value="Hello" />"#, "test").expect("parse template");
		let state = WuiValue::object(Vec::new());
		let rendered = template.render_with_route(
			&state,
			&RouteContext {
				path: "/".to_string(),
				params: HashMap::new(),
				query: HashMap::new(),
			},
		);

		assert_eq!(
			rendered.payload,
			ItemPayload::Text {
				value: "Hello".to_string()
			}
		);
	}

	#[test]
	fn text_input_type_prop_sets_input_type() {
		let template = Template::parse(
			r#"<TextInput value="" placeholder="Password" type="password" />"#,
			"test",
		)
		.expect("parse template");
		let state = WuiValue::object(Vec::new());
		let rendered = template.render(&state);

		assert_eq!(
			rendered.payload,
			ItemPayload::TextInput {
				value: "".to_string(),
				placeholder: "Password".to_string(),
				input_type: "password".to_string(),
			}
		);
	}

	#[test]
	fn form_props_render_to_item_fields() {
		let template = Template::parse(
			r#"<Form action="/auth/login" method="post"><TextInput name="username" value="" /></Form>"#,
			"test",
		)
		.expect("parse template");
		let state = WuiValue::object(Vec::new());
		let rendered = template.render(&state);

		match rendered.payload {
			ItemPayload::Form {
				action,
				method,
				body,
				..
			} => {
				assert_eq!(action, "/auth/login");
				assert_eq!(method, "post");
				assert_eq!(body[0].name, "username");
			}
			other => panic!("expected form, got {other:?}"),
		}
	}

	#[test]
	fn custom_component_src_without_entry_keeps_entry_empty() {
		let template = Template::parse(
			r#"<CustomComponent id=7 src="/robot-scene" props={state.props} />"#,
			"test",
		)
		.expect("parse template");
		let state = WuiValue::object(vec![(
			"props".to_string(),
			WuiValue::object(vec![(
				"mode".to_string(),
				WuiValue::String("live".to_string()),
			)]),
		)]);
		let rendered = template.render(&state);

		assert_eq!(rendered.id, 7);
		assert_eq!(
			rendered.payload,
			ItemPayload::Custom {
				name: "/robot-scene".to_string(),
				entry: "".to_string(),
				props: serde_json::json!({ "mode": "live" }),
				events: HashMap::new(),
			}
		);
	}

	#[test]
	fn custom_component_src_can_use_explicit_entry() {
		let template = Template::parse(
			r#"<CustomComponent id=7 src="/project/demo/robot-scene" entry="/fs/wgui-controllers/robot-scene/controller.js" />"#,
			"test",
		)
		.expect("parse template");
		let rendered = template.render(&WuiValue::Null);

		assert_eq!(
			rendered.payload,
			ItemPayload::Custom {
				name: "/project/demo/robot-scene".to_string(),
				entry: "/fs/wgui-controllers/robot-scene/controller.js".to_string(),
				props: serde_json::json!({}),
				events: HashMap::new(),
			}
		);
	}

	#[test]
	fn custom_component_events_decode_by_event_name() {
		let template = Template::parse(
			r#"<CustomComponent src="/trackpad" onMouseMoved="MovePeerMouse" />"#,
			"test",
		)
		.expect("parse template");
		let event = crate::types::ClientEvent::OnCustom(crate::types::OnCustom {
			id: action_id("MovePeerMouse"),
			inx: None,
			name: "mouseMoved".to_string(),
			payload: serde_json::json!({ "dx": 4, "dy": -2 }),
		});

		let decoded = template.decode(&event).expect("decode action");

		match decoded {
			RuntimeAction::Custom {
				name,
				event,
				payload,
			} => {
				assert_eq!(name, "MovePeerMouse");
				assert_eq!(event, "mouseMoved");
				assert_eq!(payload, serde_json::json!({ "dx": 4, "dy": -2 }));
			}
			_ => panic!("expected custom action"),
		}
	}

	#[test]
	fn imported_component_forwards_custom_event_bindings_to_custom_root() {
		let template = Template::parse_with_sources(
			r#"
			<Import name="Trackpad" from="trackpad" />
			<Trackpad onMouseMoved="MovePeerMouse" />
			"#,
			"test",
			Some(Path::new("/embedded/pages/control.wui").parent().unwrap()),
			&[(
				"/embedded/pages/trackpad.wui",
				r#"<CustomComponent src="/trackpad" />"#,
			)],
		)
		.expect("parse template");
		let rendered = template.render(&WuiValue::Null);

		let ItemPayload::Custom { events, .. } = rendered.payload else {
			panic!("expected custom component");
		};
		assert_eq!(
			events.get("mouseMoved").copied(),
			Some(action_id("MovePeerMouse"))
		);
	}

	#[test]
	fn multi_root_template_fallback_fills_width() {
		let template = Template::parse(r#"<Text value="Hello" /><Text value="World" />"#, "test")
			.expect("parse template");
		let rendered = template.render(&WuiValue::Null);

		let ItemPayload::Layout(_) = rendered.payload else {
			panic!("expected root layout");
		};
		assert!(rendered.fill);
	}

	#[test]
	fn imported_component_renders_props_and_children() {
		let suffix = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_nanos();
		let dir = std::env::temp_dir().join(format!("wui_runtime_component_test_{}", suffix));
		std::fs::create_dir_all(&dir).expect("create temp dir");
		std::fs::write(
			dir.join("layout.wui"),
			r#"
			<VStack fill=true>
				<Text value={title} />
				<Children />
			</VStack>
			"#,
		)
		.expect("write layout");
		let template = Template::parse_with_dir(
			r#"
			<Import name="AppLayout" from="layout" />
			<AppLayout title="Peers">
				<Text value="Body" />
			</AppLayout>
			"#,
			"test",
			Some(&dir),
		)
		.expect("parse template");
		let rendered = template.render(&WuiValue::Null);
		let mut values = Vec::new();
		text_values(&rendered, &mut values);

		assert!(rendered.fill);
		assert_eq!(values, vec!["Peers".to_string(), "Body".to_string()]);
	}

	#[test]
	fn embedded_sources_resolve_imports_without_filesystem_templates() {
		let root = "/embedded/pages/home.wui";
		let root_source = r#"
			<Import name="AppLayout" from="../layouts/app" />
			<AppLayout title="Home">
				<Text value="Body" />
			</AppLayout>
			"#;
		let template = Template::parse_with_sources(
			root_source,
			"pages/home",
			Path::new(root).parent(),
			&[
				(root, root_source),
				(
					"/embedded/layouts/app.wui",
					r#"
					<VStack fill=true>
						<Text value={title} />
						<Children />
					</VStack>
					"#,
				),
			],
		)
		.expect("parse embedded template");
		let rendered = template.render(&WuiValue::Null);
		let mut values = Vec::new();
		text_values(&rendered, &mut values);

		assert!(rendered.fill);
		assert_eq!(values, vec!["Home".to_string(), "Body".to_string()]);
	}

	#[test]
	fn path_matches_can_style_imported_component() {
		let suffix = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_nanos();
		let dir = std::env::temp_dir().join(format!("wui_runtime_path_match_test_{}", suffix));
		std::fs::create_dir_all(&dir).expect("create temp dir");
		std::fs::write(
			dir.join("nav_link.wui"),
			r##"
			<VStack
				backgroundColor={path_matches(activeWhen ?? href) ? "#dbeafe" : "#ffffff"}
				border={path_matches(activeWhen ?? href) ? "1px solid #60a5fa" : "1px solid #cbd5e1"}
			>
				<Link text={text} href={href} />
			</VStack>
			"##,
		)
		.expect("write nav link");
		let template = Template::parse_with_dir(
			r#"
			<Import name="NavLink" from="nav_link" />
			<NavLink text="Home" href="/" />
			<NavLink text="Peers" href="/peers" activeWhen="/peers/*" />
			"#,
			"test",
			Some(&dir),
		)
		.expect("parse template");
		let rendered = template.render_with_path(&WuiValue::Null, "/peers/123");
		let ItemPayload::Layout(layout) = rendered.payload else {
			panic!("expected wrapper layout");
		};

		assert_eq!(layout.body[0].background_color, "#ffffff");
		assert_eq!(layout.body[1].background_color, "#dbeafe");
		assert_eq!(layout.body[1].border, "1px solid #60a5fa");
	}

	#[test]
	fn template_renders_link_widget() {
		let template = Template::parse(r#"<Link href="/peers" text="Peers" />"#, "test")
			.expect("parse template");
		let rendered = template.render(&WuiValue::Null);

		assert_eq!(
			rendered.payload,
			ItemPayload::Link {
				href: "/peers".to_string(),
				text: "Peers".to_string(),
			}
		);
	}
}
