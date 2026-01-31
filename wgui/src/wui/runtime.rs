use crate::gui::{self, Item};
use crate::wui::ast::{BinaryOp, Expr, Literal, UnaryOp};
use crate::wui::compiler::ir::{ActionDef, ActionPayload, EventKind, IrNode, IrProp, IrWidget};
use crate::wui::diagnostic::Diagnostic;
use crate::wui::imports;

use async_trait::async_trait;
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

pub trait WuiController {
	fn render(&self) -> Item;
	fn handle(&mut self, event: &crate::types::ClientEvent) -> bool;
}

pub struct Ctx<T> {
	pub state: Arc<T>,
	title: Arc<Mutex<String>>,
	title_tx: mpsc::UnboundedSender<String>,
}

impl<T> Ctx<T> {
	pub fn new(state: T) -> Self {
		let (title_tx, mut title_rx) = mpsc::unbounded_channel::<String>();
		let title = Arc::new(Mutex::new(String::new()));
		let title_handle = title.clone();
		tokio::spawn(async move {
			while let Some(next) = title_rx.recv().await {
				*title_handle.lock().unwrap() = next;
			}
		});
		Self {
			state: Arc::new(state),
			title,
			title_tx,
		}
	}

	pub fn spawn<F>(&self, fut: F)
	where
		F: Future + Send + 'static,
		F::Output: Send + 'static,
	{
		tokio::spawn(fut);
	}

	pub fn set_title(&self, title: impl Into<String>) {
		*self.title.lock().unwrap() = title.into();
	}

	pub fn set_title_deferred(&self, title: impl Into<String>) {
		let _ = self.title_tx.send(title.into());
	}

	pub fn title(&self) -> String {
		self.title.lock().unwrap().clone()
	}
}

#[async_trait]
pub trait Component: Send + Sync + 'static {
	type Context: Send + Sync + 'static;
	type Model: WuiModel;

	async fn mount(ctx: Arc<Ctx<Self::Context>>) -> Self;
	fn render(&self, ctx: &Ctx<Self::Context>) -> Self::Model;
	fn unmount(self, ctx: Arc<Ctx<Self::Context>>);
}

#[derive(Debug, Clone)]
pub enum RuntimeAction {
	Click { name: String, arg: Option<u32> },
	TextChanged { name: String, value: String },
	SliderChange { name: String, value: i32 },
	Select { name: String, value: String },
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

pub trait WuiModel: WuiValueConvert {}

impl<T: WuiValueConvert + ?Sized> WuiModel for T {}

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
		let mut ctx = EvalContext::new(state.wui_value());
		let mut children = Vec::new();
		render_nodes(&self.doc.nodes, &mut children, &mut ctx);
		gui::vstack(children)
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

#[cfg(feature = "axum")]
pub fn router_with_component<T>(
	ctx: Arc<Ctx<T::Context>>,
	routes: &[&'static str],
) -> axum::Router
where
	T: Component + WuiController,
{
	let mut wgui = crate::Wgui::new_without_server();
	let handle = wgui.handle();
	let ssr_ctx = ctx.clone();
	let router = crate::axum::router_with_ssr_routes(
		handle.clone(),
		Arc::new(move || {
			let controller = tokio::task::block_in_place(|| {
				tokio::runtime::Handle::current().block_on(T::mount(ssr_ctx.clone()))
			});
			WuiController::render(&controller)
		}),
		routes,
	);
	let render_handle = handle.clone();
	tokio::spawn(async move {
		let mut controllers: HashMap<_, T> = HashMap::new();
		while let Some(message) = wgui.next().await {
			let client_id = message.client_id;
			match message.event {
				crate::ClientEvent::Connected { id: _ } => {
					let controller = T::mount(ctx.clone()).await;
					let item = WuiController::render(&controller);
					render_handle.render(client_id, item).await;
					let title = ctx.title();
					if !title.is_empty() {
						render_handle.set_title(client_id, &title).await;
					}
					controllers.insert(client_id, controller);
				}
					crate::ClientEvent::Disconnected { id: _ } => {
						if let Some(controller) = controllers.remove(&client_id) {
							controller.unmount(ctx.clone());
						}
					}
				crate::ClientEvent::PathChanged(_) => {}
				crate::ClientEvent::Input(_) => {}
				_ => {
					let item = match controllers.get_mut(&client_id) {
						Some(controller) => {
							if WuiController::handle(controller, &message.event) {
								Some(WuiController::render(controller))
							} else {
								None
							}
						}
						None => None,
					};
					if let Some(item) = item {
						render_handle.render(client_id, item).await;
						let title = ctx.title();
						if !title.is_empty() {
							render_handle.set_title(client_id, &title).await;
						}
					}
				}
			}
		}
	});
	router
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
	fn new(state: WuiValue) -> Self {
		let mut vars = HashMap::new();
		vars.insert("state".to_string(), state);
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
