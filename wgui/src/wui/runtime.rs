use crate::gui::{self, Item};
use crate::wui::ast::{BinaryOp, Expr, Literal, UnaryOp};
use crate::wui::compiler::ir::{ActionDef, ActionPayload, EventKind, IrNode, IrProp, IrWidget};
use crate::wui::diagnostic::Diagnostic;
use crate::wui::imports;

pub use async_trait::async_trait;
use std::collections::HashMap;
use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct Template {
	doc: crate::wui::compiler::ir::IrDocument,
}

#[async_trait]
pub trait WuiController {
	fn render(&self) -> Item;
	fn render_with_path(&self, path: &str) -> Item {
		let _ = path;
		self.render()
	}
	fn route_title(&self, _path: &str) -> Option<String> {
		None
	}
	fn set_runtime_context(&mut self, _client_id: Option<usize>, _session: Option<String>) {}
	async fn handle(&mut self, event: &crate::types::ClientEvent) -> bool;
}

pub struct Ctx<T, DB = ()> {
	pub state: Arc<T>,
	pub db: Arc<DB>,
	current_client: Arc<Mutex<Option<usize>>>,
	current_session: Arc<Mutex<Option<String>>>,
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

	pub fn push_state(&self, url: impl Into<String>) {
		let url = url.into();
		if let Some(client_id) = *self.current_client.lock().unwrap() {
			let _ = self
				.command_tx
				.send(RuntimeCommand::PushState { client_id, url });
		}
	}

	pub fn session_id(&self) -> Option<String> {
		self.current_session.lock().unwrap().clone()
	}

	pub fn client_id(&self) -> Option<usize> {
		*self.current_client.lock().unwrap()
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

	async fn mount(ctx: Arc<Ctx<Self::Context, Self::Db>>) -> Self;
	fn render(&self, ctx: &Ctx<Self::Context, Self::Db>) -> Self::Model;
	fn unmount(self, ctx: Arc<Ctx<Self::Context, Self::Db>>);
}

#[derive(Debug, Clone)]
pub enum RuntimeAction {
	Click { name: String, arg: Option<u32> },
	TextChanged { name: String, value: String },
	SliderChange { name: String, value: i32 },
	Select { name: String, value: String },
}

#[derive(Debug, Clone)]
pub enum RuntimeCommand {
	SetTitle { client_id: usize, title: String },
	PushState { client_id: usize, url: String },
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
		let mut diags = Vec::new();
		let validated = crate::wui::compiler::validate::validate(&resolved.nodes, &mut diags);
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
		let mut ctx = EvalContext::new(state.wui_value(), path);
		let mut children = Vec::new();
		render_nodes(&self.doc.nodes, &mut children, &mut ctx);
		gui::vstack(children)
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
}

impl EvalContext {
	fn new(state: WuiValue, path: &str) -> Self {
		let mut vars = HashMap::new();
		vars.insert("state".to_string(), state);
		vars.insert("path".to_string(), WuiValue::String(path.to_string()));
		Self { vars }
	}

	fn with_var(&self, name: &str, value: WuiValue) -> Self {
		let mut vars = self.vars.clone();
		vars.insert(name.to_string(), value);
		Self { vars }
	}
}

fn decode_action(action: &ActionDef, event: &crate::types::ClientEvent) -> Option<RuntimeAction> {
	match action.kind {
		EventKind::Click => match event {
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
				Some(RuntimeAction::SliderChange {
					name: action.name.clone(),
					value: ev.value,
				})
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
	}
}

fn render_nodes(nodes: &[IrNode], out: &mut Vec<Item>, ctx: &mut EvalContext) {
	for node in nodes {
		match node {
			IrNode::Widget(widget) => out.push(render_widget(widget, ctx)),
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
	let mut base = match widget.tag.as_str() {
		"VStack" => render_container(gui::vstack, &widget.children, ctx),
		"HStack" => render_container(gui::hstack, &widget.children, ctx),
		"Text" => gui::text(&text_value(widget, ctx)),
		"Button" => gui::button(&textual_value(widget, ctx, "text")),
		"TextInput" => gui::text_input(),
		"Checkbox" => gui::checkbox(),
		"Slider" => gui::slider(),
		"Image" => {
			let (src, alt) = image_values(widget, ctx);
			gui::img(&src, &alt)
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
			"Image" => name != "src" && name != "alt",
			_ => true,
		},
	}
}

fn apply_prop(item: Item, prop: &IrProp, ctx: &mut EvalContext) -> Item {
	match prop {
		IrProp::Event { action, arg, .. } => {
			let mut item = item.id(action_id(action));
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
		"placeholder" => item.placeholder(value),
		"textAlign" => item.text_align(value),
		"cursor" => item.cursor(value),
		"overflow" => item.overflow(value),
		"backgroundColor" => item.background_color(value),
		"border" => item.border(value),
		"objectFit" => item.object_fit(value),
		_ => item,
	}
}

fn apply_number_prop(item: Item, name: &str, value: f64) -> Item {
	match name {
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
		_ => item,
	}
}

fn apply_bool_prop(item: Item, name: &str, value: bool) -> Item {
	match name {
		"checked" | "bind:checked" => item.checked(value),
		"wrap" => item.wrap(value),
		"open" => item.open(value),
		"hresize" => item.hresize(value),
		"vresize" => item.vresize(value),
		_ => item,
	}
}

fn is_string_prop(name: &str) -> bool {
	matches!(
		name,
		"svalue"
			| "bind:svalue"
			| "placeholder"
			| "textAlign"
			| "cursor"
			| "overflow"
			| "backgroundColor"
			| "border"
			| "objectFit"
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

fn route_params(route: &str, path: &str) -> Option<HashMap<String, String>> {
	if route == path {
		return Some(HashMap::new());
	}
	let route_parts: Vec<&str> = route
		.trim_matches('/')
		.split('/')
		.filter(|s| !s.is_empty())
		.collect();
	let path_parts: Vec<&str> = path
		.trim_matches('/')
		.split('/')
		.filter(|s| !s.is_empty())
		.collect();
	let mut params = HashMap::new();
	let mut wildcard_at = None;
	for (index, seg) in route_parts.iter().enumerate() {
		if *seg == "*" || *seg == "{*wildcard}" {
			wildcard_at = Some(index);
			break;
		}
	}
	let end = wildcard_at.unwrap_or(route_parts.len());
	if wildcard_at.is_none() && end != path_parts.len() {
		return None;
	}
	if wildcard_at.is_some() && path_parts.len() < end {
		return None;
	}
	for i in 0..end {
		let route_seg = route_parts[i];
		let path_seg = path_parts[i];
		if let Some(name) = param_name(route_seg) {
			params.insert(name.to_string(), path_seg.to_string());
		} else if route_seg != path_seg {
			return None;
		}
	}
	Some(params)
}

fn param_name(segment: &str) -> Option<&str> {
	if let Some(name) = segment.strip_prefix(':') {
		if !name.is_empty() {
			return Some(name);
		}
	}
	if segment.starts_with('{') && segment.ends_with('}') {
		let inner = &segment[1..segment.len() - 1];
		if inner.starts_with('*') {
			return None;
		}
		if !inner.is_empty() {
			return Some(inner);
		}
	}
	None
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
