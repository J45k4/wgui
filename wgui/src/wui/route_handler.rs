use crate::gui::Item;
use crate::wui::runtime::{Ctx, RouteContext, Template, WuiValue};
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Path params extracted from a route's `:segment` placeholders.
///
/// Keys are the segment names (e.g. `"id"` for `/todos/:id`); values are the
/// raw URL-decoded strings. Use [`PathParams::get`] to decode them into typed
/// values via [`FromParam`].
#[derive(Debug, Clone, Default)]
pub struct PathParams(pub HashMap<String, String>);

impl PathParams {
	/// Look up a path param and decode it via [`FromParam`].
	///
	/// Returns `None` if the name isn't in this map; returns the conversion
	/// error if the raw string can't be parsed into `T`.
	pub fn get<T: FromParam>(&self, name: &str) -> Option<Result<T, ParamError>> {
		self.0.get(name).map(|raw| T::from_param(raw))
	}

	/// Borrow a raw param value without decoding.
	pub fn raw(&self, name: &str) -> Option<&str> {
		self.0.get(name).map(|s| s.as_str())
	}

	pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
		self.0.iter()
	}

	pub fn len(&self) -> usize {
		self.0.len()
	}

	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}
}

/// URL-encoded fields submitted with a `POST` route.
///
/// The `#[route]` macro decodes one non-path handler argument from these
/// fields. For example, `form: CreateTodoForm` on a `POST` route invokes
/// [`RouteFormData::decode`] to build `CreateTodoForm`.
#[derive(Debug, Clone, Default)]
pub struct RouteFormData(HashMap<String, String>);

impl RouteFormData {
	pub fn from_fields(fields: HashMap<String, String>) -> Self {
		Self(fields)
	}

	pub fn from_urlencoded(body: &[u8]) -> Self {
		Self(
			form_urlencoded::parse(body)
				.into_owned()
				.collect::<HashMap<_, _>>(),
		)
	}

	/// Decode fields into a `#[derive(serde::Deserialize)]` form type.
	///
	pub fn decode<T: DeserializeOwned>(&self) -> Result<T, ParamError> {
		let encoded = form_urlencoded::Serializer::new(String::new())
			.extend_pairs(
				self.0
					.iter()
					.map(|(name, value)| (name.as_str(), value.as_str())),
			)
			.finish();
		serde_urlencoded::from_str(&encoded)
			.map_err(|error| ParamError::new(format!("invalid form data: {error}")))
	}

	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}
}

/// Error returned when a path param can't be decoded into the requested type.
#[derive(Debug, Clone)]
pub struct ParamError(pub String);

impl ParamError {
	pub fn new(msg: impl Into<String>) -> Self {
		Self(msg.into())
	}
}

impl std::fmt::Display for ParamError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "failed to decode path param: {}", self.0)
	}
}

impl std::error::Error for ParamError {}

/// Conversion from a raw path-param string into a typed value.
///
/// Implemented for the primitive numeric/string types the `#[route]` macro
/// supports as handler arguments. Add impls on demand.
pub trait FromParam: Sized {
	fn from_param(raw: &str) -> Result<Self, ParamError>;
}

impl FromParam for String {
	fn from_param(raw: &str) -> Result<Self, ParamError> {
		Ok(raw.to_string())
	}
}

impl FromParam for u32 {
	fn from_param(raw: &str) -> Result<Self, ParamError> {
		raw.parse::<u32>()
			.map_err(|e| ParamError::new(format!("expected u32: {e}")))
	}
}

impl FromParam for u64 {
	fn from_param(raw: &str) -> Result<Self, ParamError> {
		raw.parse::<u64>()
			.map_err(|e| ParamError::new(format!("expected u64: {e}")))
	}
}

impl FromParam for i32 {
	fn from_param(raw: &str) -> Result<Self, ParamError> {
		raw.parse::<i32>()
			.map_err(|e| ParamError::new(format!("expected i32: {e}")))
	}
}

impl FromParam for i64 {
	fn from_param(raw: &str) -> Result<Self, ParamError> {
		raw.parse::<i64>()
			.map_err(|e| ParamError::new(format!("expected i64: {e}")))
	}
}

/// What a `#[route]` handler returns. The framework follows [`Redirect`]s
/// (re-render target page, diff, send patch), and renders [`View`]s directly.
#[derive(Debug)]
pub enum RouteResult {
	/// Render a page or partial. `title` is set on full page renders; partials
	/// leave it `None` and the framework patches the existing title.
	View(Box<View>),
	/// Issue a client-side navigation to a new URL (PRG: Post/Redirect/Get).
	Redirect(Redirect),
	/// No matching route / guarded access denied. Triggers the fallback page.
	NotFound,
}

/// A rendered view: an [`Item`] tree plus optional page title.
#[derive(Debug, Clone)]
pub struct View {
	pub item: Item,
	pub title: Option<String>,
	/// HTTP status used for normal form submissions. Websocket form
	/// submissions render the same view in place and ignore this value.
	pub status: u16,
	/// `Some(addr)` if this view is a partial render addressable via
	/// `ctx.render(addr)`. `None` for full page renders.
	pub partial_addr: Option<String>,
	/// A WUI model waiting for the route's registered template. This is set by
	/// [`crate::view!`] and resolved by the route dispatcher before a response
	/// reaches the client or SSR renderer.
	wui_model: Option<WuiValue>,
}

impl View {
	/// Full page render with an explicit `<title>`.
	pub fn page(title: impl Into<String>, item: Item) -> Self {
		Self {
			item,
			title: Some(title.into()),
			status: 200,
			partial_addr: None,
			wui_model: None,
		}
	}

	/// Full page render with no title (title stays whatever the client has).
	pub fn untitled(item: Item) -> Self {
		Self {
			item,
			title: None,
			status: 200,
			partial_addr: None,
			wui_model: None,
		}
	}

	/// Partial render: re-renderable in isolation via `ctx.render(addr)`.
	///
	/// The concrete address is supplied by the registered `#[partial]` route at
	/// dispatch time, so handlers only describe the item region itself.
	pub fn partial(item: Item) -> Self {
		Self {
			item,
			title: None,
			status: 200,
			partial_addr: None,
			wui_model: None,
		}
	}

	/// Create a view whose model will be rendered with the template attached to
	/// the route handler. Prefer the [`crate::view!`] macro at call sites.
	#[doc(hidden)]
	pub fn wui(model: WuiValue) -> Self {
		Self {
			item: Item::default(),
			title: None,
			status: 200,
			partial_addr: None,
			wui_model: Some(model),
		}
	}

	/// Set the HTTP status used when this view is returned to a regular form
	/// request. The websocket transport renders it in place.
	pub fn with_status(mut self, status: u16) -> Self {
		self.status = status;
		self
	}

	pub(crate) fn render_wui(&mut self, template: Option<&Template>, route: &RouteContext) {
		let Some(model) = self.wui_model.take() else {
			return;
		};
		let template = template.expect(
			"view! requires a route declared with `#[route(path, view)]` or an explicit template",
		);
		self.item = template.render_with_route(&model, route);
	}
}

impl From<Item> for View {
	fn from(item: Item) -> Self {
		Self::untitled(item)
	}
}

impl From<Item> for RouteResult {
	fn from(item: Item) -> Self {
		RouteResult::View(Box::new(item.into()))
	}
}

impl From<View> for RouteResult {
	fn from(view: View) -> Self {
		RouteResult::View(Box::new(view))
	}
}

/// A redirect response. Wire-shape: client navigates to `url`, server
/// matches a `#[route]`, renders, diffs, patches.
#[derive(Debug, Clone)]
pub struct Redirect(pub String);

impl Redirect {
	pub fn to(url: impl Into<String>) -> Self {
		Self(url.into())
	}
}

impl From<Redirect> for RouteResult {
	fn from(r: Redirect) -> Self {
		RouteResult::Redirect(r)
	}
}

/// `()` handler returns are treated as "action ran, redirect to current
/// page". Lets action handlers omit an explicit `Redirect::to(...)`.
impl From<()> for RouteResult {
	fn from(_: ()) -> Self {
		// Redirect to "" is interpreted by the dispatcher as "current route".
		RouteResult::Redirect(Redirect(String::new()))
	}
}

/// A boxed future returned by [`RouteHandler::call`]. Required so the trait
/// is object-safe (a single `dyn RouteHandler` can serve heterogeneous
/// handlers with different future types).
///
/// `'static` because handlers may borrow `Arc<Ctx<T>>` (cloned cheaply)
/// and `PathParams` (owned) — no borrowed data crosses the await.
pub type RouteFuture = Pin<Box<dyn Future<Output = RouteResult> + Send + 'static>>;

/// Trait implemented by every `#[route]`-generated marker struct.
///
/// The `#[route]` proc macro parses the handler fn's signature and emits a
/// sibling marker implementing this trait. Users never write `impl
/// RouteHandler` by hand — they register handlers via
/// [`crate::Wgui::add_route`].
///
/// `State` is the `T` in `Ctx<T, DB>` the handler expects. The framework
/// type-erases this when storing handlers, downcasting at dispatch time.
pub trait RouteHandler: Send + Sync + 'static + Copy {
	/// App state type. Must match the `Ctx<T>` the `Wgui` instance was set
	/// up with via `Wgui::set_ctx`.
	type State: Send + Sync + 'static;
	/// Database type carried by the route context. Use `()` for apps without a
	/// database.
	type Db: Send + Sync + 'static;

	/// Route pattern, e.g. `"/todos/:id"`.
	fn path(&self) -> &str;

	/// HTTP method this handler accepts. `GET` for page renders, `POST` for
	/// mutations. Defaults to `GET`.
	fn method(&self) -> HttpMethod {
		HttpMethod::Get
	}

	/// Template used to render a [`crate::view!`] result. The `#[route]`
	/// macro supplies this for routes declared with its `view` option.
	fn wui_template(&self) -> Option<&'static Template> {
		None
	}

	/// Dispatch the handler. Params have already been extracted from the
	/// URL and placed in `params`. `ctx` is an `Arc<Ctx<T>>` clone from the
	/// framework's context registry — handlers can cheaply clone it into
	/// async blocks since it's reference-counted.
	///
	/// `Self` is `Copy` because the generated marker structs are zero-sized
	/// — the trait is only implemented on those, so the bound is free and
	/// lets us thread the handler into an async block without lifetime
	/// acrobatics.
	fn call(
		self,
		ctx: Arc<Ctx<Self::State, Self::Db>>,
		params: PathParams,
		form: RouteFormData,
	) -> RouteFuture;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
	Get,
	Post,
}

impl HttpMethod {
	pub fn as_str(self) -> &'static str {
		match self {
			HttpMethod::Get => "GET",
			HttpMethod::Post => "POST",
		}
	}
}

/// Object-safe view of a [`RouteHandler`] used by the framework's boxed
/// route registry. `State` is erased — the framework downcasts `ctx` to
/// the concrete `Ctx<T>` at dispatch time using the registered type id.
/// Runtime context passed to [`DynRouteHandler::call_dyn`] so the blanket
/// impl can configure `Ctx<T>` before dispatching the typed handler.
pub struct RuntimeContext {
	pub client_id: Option<usize>,
	pub session: Option<String>,
	pub route: Option<crate::wui::runtime::RouteContext>,
}

pub trait DynRouteHandler: Send + Sync + 'static {
	fn path(&self) -> &str;
	fn method(&self) -> HttpMethod {
		HttpMethod::Get
	}
	fn state_type_id(&self) -> std::any::TypeId;
	/// Object-safe dispatch. Implementors downcast `ctx_any` to their
	/// concrete `Ctx<T>`, set runtime context, then forward to the typed
	/// handler.
	fn call_dyn(
		&self,
		ctx_any: std::sync::Arc<dyn std::any::Any + Send + Sync>,
		params: PathParams,
		form: RouteFormData,
		runtime: RuntimeContext,
	) -> RouteFuture;
}

/// Blanket: any `RouteHandler` is also a `DynRouteHandler` via downcasting.
impl<H: RouteHandler> DynRouteHandler for H
where
	H::State: 'static + Send + Sync,
	H::Db: 'static + Send + Sync,
{
	fn path(&self) -> &str {
		RouteHandler::path(self)
	}

	fn method(&self) -> HttpMethod {
		RouteHandler::method(self)
	}

	fn state_type_id(&self) -> std::any::TypeId {
		std::any::TypeId::of::<H::State>()
	}

	fn call_dyn(
		&self,
		ctx_any: std::sync::Arc<dyn std::any::Any + Send + Sync>,
		params: PathParams,
		form: RouteFormData,
		runtime: RuntimeContext,
	) -> RouteFuture {
		let this = *self;
		Box::pin(async move {
			let ctx = ctx_any
				.downcast::<Ctx<H::State, H::Db>>()
				.expect("route context type mismatch: Ctx<T, DB> registered with Wgui does not match the #[route] handler");
			ctx.set_current_client(runtime.client_id);
			ctx.set_current_session(runtime.session);
			ctx.set_current_route(runtime.route.clone());
			let mut result = RouteHandler::call(this, ctx, params, form).await;
			if let RouteResult::View(view) = &mut result {
				if let Some(route) = runtime.route.as_ref() {
					view.render_wui(RouteHandler::wui_template(&this), route);
				}
			}
			result
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn from_param_string_passes_through() {
		assert_eq!(String::from_param("hello").unwrap(), "hello");
	}

	#[test]
	fn from_param_u32_parses() {
		assert_eq!(u32::from_param("42").unwrap(), 42);
	}

	#[test]
	fn from_param_u32_rejects_garbage() {
		assert!(u32::from_param("not-a-number").is_err());
	}

	#[test]
	fn path_params_get_decodes() {
		let mut params = PathParams::default();
		params.0.insert("id".to_string(), "7".to_string());
		let id: u32 = params.get::<u32>("id").unwrap().unwrap();
		assert_eq!(id, 7);
	}

	#[test]
	fn path_params_raw_no_decode() {
		let mut params = PathParams::default();
		params.0.insert("id".to_string(), "7".to_string());
		assert_eq!(params.raw("id"), Some("7"));
		assert_eq!(params.raw("missing"), None);
	}

	#[test]
	fn route_form_data_decodes_typed_fields() {
		#[derive(Debug, serde::Deserialize, PartialEq)]
		struct Form {
			name: String,
			count: u32,
			done: bool,
		}

		let form = RouteFormData::from_urlencoded(b"name=puppy&count=7&done=true");
		assert_eq!(
			form.decode::<Form>().unwrap(),
			Form {
				name: "puppy".to_string(),
				count: 7,
				done: true,
			}
		);
	}

	#[test]
	fn view_page_carries_title() {
		let v = View::page("Hello", Item::default());
		assert_eq!(v.title.as_deref(), Some("Hello"));
		assert!(v.partial_addr.is_none());
	}

	#[test]
	fn view_partial_has_addr_no_title() {
		let v = View::partial(Item::default());
		assert!(v.partial_addr.is_none());
		assert!(v.title.is_none());
	}

	#[test]
	fn redirect_to_string() {
		let r = Redirect::to("/todos");
		assert_eq!(r.0, "/todos");
	}

	#[test]
	fn route_result_from_item_is_view() {
		let item = Item::default();
		let r: RouteResult = item.into();
		assert!(matches!(r, RouteResult::View(_)));
	}

	#[test]
	fn view_can_return_an_unprocessable_form_response() {
		let view = View::untitled(Item::default()).with_status(422);
		assert_eq!(view.status, 422);
	}

	#[test]
	fn route_result_from_redirect_is_redirect() {
		let r: RouteResult = Redirect::to("/x").into();
		assert!(matches!(r, RouteResult::Redirect(_)));
	}

	#[test]
	fn route_result_from_unit_is_redirect_to_empty() {
		let r: RouteResult = ().into();
		assert!(matches!(r, RouteResult::Redirect(_)));
	}

	#[tokio::test]
	async fn ctx_render_queues_partial_command() {
		let ctx = Ctx::new(());
		let mut rx = ctx.take_command_rx();
		ctx.render("/todos/7/status");
		assert!(matches!(
			rx.recv().await,
			Some(crate::wui::runtime::RuntimeCommand::RenderPartial { topic }) if topic == "/todos/7/status"
		));
	}
}
