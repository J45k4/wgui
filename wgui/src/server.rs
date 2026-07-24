#![cfg(feature = "hyper")]

use futures_util::{Stream, StreamExt};
use http_body_util::{combinators::UnsyncBoxBody, BodyExt, Full, StreamBody};
use hyper::body::{Bytes, Frame};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::Request;
use hyper::Response;
use hyper_util::rt::TokioIo;
use std::collections::HashMap;
use std::convert::Infallible;
use std::future::Future;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::net::TcpListener;
use tokio::sync::mpsc;

use crate::ssr;
use crate::types::{ClientMessage, Clients};
use crate::ws::TungsteniteWs;
use crate::wui::routing::{best_route_index, RoutePattern};
use crate::wui::runtime::RouteContext;
use crate::{Sessions, SsrHydrationRoot, SsrHydrationRoots, SsrResponse, WguiHandle};

const INDEX_HTML_BYTES: &[u8] = include_bytes!("../../dist/index.html");
const INDEX_JS_BYTES: &[u8] = include_bytes!("../../dist/index.js");
const CSS_JS_BYTES: &[u8] = include_bytes!("../../dist/index.css");
const MAX_SSR_HYDRATION_ROOTS: usize = 128;
const STATIC_ASSET_VERSION_PARAM: &str = "wgui-v";
const IMMUTABLE_STATIC_CACHE_CONTROL: &str = "public, max-age=31536000, immutable";
const DEFAULT_STATIC_CACHE_CONTROL: &str = "public, max-age=86400";
const UNVERSIONED_STATIC_CACHE_CONTROL: &str = "no-store";
static NEXT_SSR_HYDRATION_ID: AtomicU64 = AtomicU64::new(1);

pub type HttpHandler = Arc<
	dyn Fn(HttpRequest) -> Pin<Box<dyn Future<Output = Option<HttpResponse>> + Send>> + Send + Sync,
>;
pub(crate) type HttpRouteHandler = Arc<
	dyn Fn(HttpRequest, HttpCtx) -> Pin<Box<dyn Future<Output = HttpResponse> + Send>>
		+ Send
		+ Sync,
>;
pub(crate) type SharedAppCss = Arc<RwLock<Option<String>>>;
pub(crate) type SharedHttpHandler = Arc<RwLock<Option<HttpHandler>>>;
pub(crate) type SharedHttpRoutes = Arc<RwLock<Vec<HttpRoute>>>;
pub(crate) type SharedStaticMounts = Arc<RwLock<Vec<StaticMount>>>;

#[derive(Clone)]
pub(crate) struct HttpRoute {
	pub(crate) method: String,
	pub(crate) pattern: RoutePattern,
	pub(crate) handler: HttpRouteHandler,
}

#[derive(Debug, Clone)]
pub struct HttpRouteSpec {
	pub method: &'static str,
	pub path: &'static str,
	pub id: &'static str,
}

impl HttpRouteSpec {
	pub fn post(path: &'static str) -> Self {
		Self {
			method: "POST",
			path,
			id: path,
		}
	}
}

#[derive(Clone)]
pub(crate) enum StaticMount {
	File {
		route: String,
		file: PathBuf,
		version: Option<String>,
	},
	Dir {
		prefix: String,
		dir: PathBuf,
	},
}

impl StaticMount {
	pub(crate) fn file(route: String, file: PathBuf) -> (Self, StaticAsset) {
		let route = normalize_mount_route(route);
		let version = match static_file_version(&file) {
			Ok(version) => Some(version),
			Err(error) => {
				log::warn!(
					"unable to fingerprint mounted static file '{}': {error}",
					file.display()
				);
				None
			}
		};
		let asset = StaticAsset::new(&route, version.as_deref());
		(
			Self::File {
				route,
				file,
				version,
			},
			asset,
		)
	}

	pub(crate) fn dir(prefix: String, dir: PathBuf) -> Self {
		Self::Dir {
			prefix: normalize_mount_route(prefix),
			dir,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticAsset {
	url: String,
}

impl StaticAsset {
	fn new(route: &str, version: Option<&str>) -> Self {
		let url = version.map_or_else(
			|| route.to_string(),
			|version| format!("{route}?{STATIC_ASSET_VERSION_PARAM}={version}"),
		);
		Self { url }
	}

	pub fn url(&self) -> &str {
		&self.url
	}
}

#[derive(Debug, Clone)]
pub struct HttpRequest {
	pub method: String,
	pub path: String,
	pub query: HashMap<String, String>,
	pub headers: HashMap<String, String>,
	pub body: Vec<u8>,
}

pub struct HttpResponse {
	pub status: u16,
	pub headers: Vec<(String, String)>,
	pub body: Vec<u8>,
	stream: Option<HttpResponseStream>,
}

#[derive(Debug, Clone)]
pub struct HttpCtx {
	pub path: String,
	pub params: HashMap<String, String>,
	pub query: HashMap<String, String>,
	pub headers: HashMap<String, String>,
	pub session: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FormData {
	fields: HashMap<String, String>,
}

pub struct Json<T>(pub T);

pub trait FromHttpRequest: Sized {
	fn from_http_request(req: &HttpRequest) -> Result<Self, HttpResponse>;
}

type HttpBody = UnsyncBoxBody<Bytes, Infallible>;
type HttpResponseStream = Pin<Box<dyn Stream<Item = Result<Vec<u8>, Infallible>> + Send>>;

impl HttpResponse {
	pub fn new(status: u16, body: impl Into<Vec<u8>>) -> Self {
		Self {
			status,
			headers: Vec::new(),
			body: body.into(),
			stream: None,
		}
	}

	pub fn stream<S>(status: u16, stream: S) -> Self
	where
		S: Stream<Item = Result<Vec<u8>, Infallible>> + Send + 'static,
	{
		Self {
			status,
			headers: Vec::new(),
			body: Vec::new(),
			stream: Some(Box::pin(stream)),
		}
	}

	pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
		self.headers.push((name.into(), value.into()));
		self
	}
}

impl FormData {
	pub fn get(&self, name: &str) -> Option<&str> {
		self.fields.get(name).map(String::as_str)
	}

	pub fn into_inner(self) -> HashMap<String, String> {
		self.fields
	}
}

impl FromHttpRequest for FormData {
	fn from_http_request(req: &HttpRequest) -> Result<Self, HttpResponse> {
		if !content_type_matches(req, "application/x-www-form-urlencoded") {
			return Err(HttpResponse::new(
				415,
				"expected application/x-www-form-urlencoded",
			));
		}
		let fields = form_urlencoded::parse(&req.body)
			.into_owned()
			.collect::<HashMap<_, _>>();
		Ok(Self { fields })
	}
}

impl<T> FromHttpRequest for Json<T>
where
	T: serde::de::DeserializeOwned,
{
	fn from_http_request(req: &HttpRequest) -> Result<Self, HttpResponse> {
		if !content_type_matches(req, "application/json") {
			return Err(HttpResponse::new(415, "expected application/json"));
		}
		serde_json::from_slice(&req.body)
			.map(Self)
			.map_err(|_| HttpResponse::new(400, "invalid json"))
	}
}

impl FromHttpRequest for HttpRequest {
	fn from_http_request(req: &HttpRequest) -> Result<Self, HttpResponse> {
		Ok(req.clone())
	}
}

fn content_type_matches(req: &HttpRequest, expected: &str) -> bool {
	req.headers
		.get("content-type")
		.and_then(|value| value.split(';').next())
		.map(|value| value.trim().eq_ignore_ascii_case(expected))
		.unwrap_or(false)
}

fn content_type_for(path: &Path) -> &'static str {
	match path
		.extension()
		.and_then(|ext| ext.to_str())
		.unwrap_or_default()
	{
		"css" => "text/css",
		"js" => "text/javascript",
		"html" => "text/html",
		"ico" => "image/x-icon",
		"stl" => "model/stl",
		"jpg" | "jpeg" => "image/jpeg",
		"png" => "image/png",
		"svg" => "image/svg+xml",
		_ => "application/octet-stream",
	}
}

fn normalize_mount_route(route: String) -> String {
	let trimmed = route.trim();
	let with_slash = if trimmed.starts_with('/') {
		trimmed.to_string()
	} else {
		format!("/{trimmed}")
	};
	if with_slash.len() > 1 {
		with_slash.trim_end_matches('/').to_string()
	} else {
		with_slash
	}
}

fn static_file_version(path: &Path) -> std::io::Result<String> {
	let bytes = std::fs::read(path)?;
	let mut hasher = DefaultHasher::new();
	bytes.hash(&mut hasher);
	Ok(format!("{:016x}", hasher.finish()))
}

fn relative_static_path(base: &Path, relative: &str) -> Option<PathBuf> {
	let mut out = base.to_path_buf();
	if relative.is_empty() {
		return Some(out);
	}
	for part in relative.split('/') {
		if part.is_empty() || part == "." || part == ".." || part.contains('\\') {
			return None;
		}
		out.push(part);
	}
	Some(out)
}

fn sanitize_asset_path(uri_path: &str) -> Option<PathBuf> {
	if !uri_path.starts_with("/assets/") {
		return None;
	}
	let relative = uri_path.trim_start_matches("/assets/");
	if relative.is_empty() {
		return None;
	}
	let mut out = PathBuf::from("assets");
	for part in relative.split('/') {
		if part.is_empty() || part == "." || part == ".." {
			return None;
		}
		out.push(part);
	}
	Some(out)
}

fn sanitize_fs_path(uri_path: &str) -> Option<PathBuf> {
	if !uri_path.starts_with("/fs/") {
		return None;
	}
	let relative = uri_path.trim_start_matches("/fs/");
	if relative.is_empty() {
		return None;
	}
	let mut out = std::env::current_dir().ok()?;
	for part in relative.split('/') {
		if part.is_empty() || part == "." || part == ".." {
			return None;
		}
		out.push(part);
	}
	Some(out)
}

async fn read_static_file(path: &Path, cache_control: &'static str) -> Response<HttpBody> {
	match tokio::fs::read(path).await {
		Ok(bytes) => Response::builder()
			.header("content-type", content_type_for(path))
			.header("cache-control", cache_control)
			.body(full_body(bytes))
			.unwrap(),
		Err(_) => Response::builder()
			.status(404)
			.body(full_body("file not found"))
			.unwrap(),
	}
}

async fn static_mount_response(
	uri_path: &str,
	requested_version: Option<&str>,
	mounts: &SharedStaticMounts,
) -> Option<Response<HttpBody>> {
	let mounts = mounts.read().unwrap().clone();
	for mount in mounts {
		match mount {
			StaticMount::File {
				route,
				file,
				version,
			} => {
				if uri_path == route {
					let cache_control =
						if version.as_deref() == requested_version && version.is_some() {
							IMMUTABLE_STATIC_CACHE_CONTROL
						} else {
							UNVERSIONED_STATIC_CACHE_CONTROL
						};
					return Some(read_static_file(&file, cache_control).await);
				}
			}
			StaticMount::Dir { prefix, dir } => {
				let relative = if prefix == "/" {
					uri_path.trim_start_matches('/')
				} else if uri_path == prefix {
					""
				} else if let Some(relative) = uri_path.strip_prefix(&format!("{prefix}/")) {
					relative
				} else {
					continue;
				};
				let Some(file) = relative_static_path(&dir, relative) else {
					return Some(
						Response::builder()
							.status(400)
							.body(full_body("bad static path"))
							.unwrap(),
					);
				};
				return Some(read_static_file(&file, DEFAULT_STATIC_CACHE_CONTROL).await);
			}
		}
	}
	None
}

fn index_html_response(app_css: &SharedAppCss) -> Vec<u8> {
	if app_css.read().unwrap().is_none() {
		return INDEX_HTML_BYTES.to_vec();
	}

	let html = String::from_utf8_lossy(INDEX_HTML_BYTES);
	html.replace(
		"<link rel=\"stylesheet\" href=\"/index.css\"></link>",
		"<link rel=\"stylesheet\" href=\"/index.css\"></link><link rel=\"stylesheet\" href=\"/app.css\"></link>",
	)
	.into_bytes()
}

struct Ctx {
	event_tx: mpsc::UnboundedSender<ClientMessage>,
	clients: Clients,
	sessions: Sessions,
	ssr: Option<Arc<dyn Fn(RouteContext, Option<String>) -> Option<SsrResponse> + Send + Sync>>,
	http_handler: SharedHttpHandler,
	http_routes: SharedHttpRoutes,
	app_css: SharedAppCss,
	static_mounts: SharedStaticMounts,
	ssr_hydration_roots: SsrHydrationRoots,
}

fn next_ssr_hydration_id() -> String {
	let count = NEXT_SSR_HYDRATION_ID.fetch_add(1, Ordering::Relaxed);
	let nanos = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.map(|duration| duration.as_nanos())
		.unwrap_or(0);
	format!("{nanos:x}-{count:x}")
}

async fn store_ssr_hydration_root(
	roots: &SsrHydrationRoots,
	id: String,
	path: String,
	item: crate::gui::Item,
	title: Option<String>,
) {
	let mut roots = roots.write().await;
	while roots.len() >= MAX_SSR_HYDRATION_ROOTS {
		let Some(key) = roots.keys().next().cloned() else {
			break;
		};
		roots.remove(&key);
	}
	roots.insert(id, SsrHydrationRoot { path, item, title });
}

fn query_map(req: &Request<hyper::body::Incoming>) -> HashMap<String, String> {
	let mut out = HashMap::new();
	let Some(query) = req.uri().query() else {
		return out;
	};
	for pair in query.split('&') {
		let mut parts = pair.splitn(2, '=');
		let key = parts.next().unwrap_or("");
		if key.is_empty() {
			continue;
		}
		let value = parts.next().unwrap_or("");
		out.insert(key.to_string(), value.to_string());
	}
	out
}

fn header_map(req: &Request<hyper::body::Incoming>) -> HashMap<String, String> {
	let mut out = HashMap::new();
	for (name, value) in req.headers() {
		if let Ok(value) = value.to_str() {
			out.insert(name.as_str().to_ascii_lowercase(), value.to_string());
		}
	}
	out
}

fn cookie_value(req: &Request<hyper::body::Incoming>, name: &str) -> Option<String> {
	let raw = req.headers().get(hyper::header::COOKIE)?;
	let header = raw.to_str().ok()?;
	for part in header.split(';') {
		let mut kv = part.trim().splitn(2, '=');
		let (Some(key), Some(value)) = (kv.next(), kv.next()) else {
			continue;
		};
		if key == name && !value.is_empty() {
			return Some(value.to_string());
		}
	}
	None
}

fn session_from_query(req: &Request<hyper::body::Incoming>) -> Option<String> {
	let query = req.uri().query()?;
	for pair in query.split('&') {
		let mut parts = pair.splitn(2, '=');
		let key = parts.next().unwrap_or("");
		let value = parts.next().unwrap_or("");
		if key == "sid" && !value.is_empty() {
			return Some(value.to_string());
		}
	}
	None
}

fn session_from_request(req: &Request<hyper::body::Incoming>) -> Option<String> {
	cookie_value(req, "sid").or_else(|| session_from_query(req))
}

fn matching_http_route(
	routes: &SharedHttpRoutes,
	method: &str,
	path: &str,
) -> Option<(HttpRoute, HashMap<String, String>)> {
	let routes = routes
		.read()
		.unwrap()
		.iter()
		.filter(|route| route.method == method)
		.cloned()
		.collect::<Vec<_>>();
	let index = best_route_index(&routes, path, |route| &route.pattern)?;
	let route = routes[index].clone();
	let params = route.pattern.match_path(path)?.params;
	Some((route, params))
}

fn full_body(body: impl Into<Bytes>) -> HttpBody {
	Full::new(body.into())
		.map_err(|never| match never {})
		.boxed_unsync()
}

fn response_body(response: HttpResponse) -> HttpBody {
	if let Some(stream) = response.stream {
		return BodyExt::boxed_unsync(StreamBody::new(
			stream.map(|chunk| chunk.map(|bytes| Frame::data(Bytes::from(bytes)))),
		));
	}
	full_body(response.body)
}

fn http_response(response: HttpResponse) -> Response<HttpBody> {
	let mut builder = Response::builder().status(response.status);
	for (name, value) in &response.headers {
		builder = builder.header(name.as_str(), value.as_str());
	}
	builder.body(response_body(response)).unwrap()
}

async fn custom_http_response(
	request: HttpRequest,
	handler: &SharedHttpHandler,
) -> Option<Response<HttpBody>> {
	let handler = handler.read().unwrap().clone()?;
	(handler)(request).await.map(http_response)
}

async fn http_request(
	req: &mut Request<hyper::body::Incoming>,
) -> Result<HttpRequest, hyper::Error> {
	let method = req.method().as_str().to_string();
	let path = req.uri().path().to_string();
	let query = query_map(req);
	let headers = header_map(req);
	let body = req.body_mut().collect().await?.to_bytes().to_vec();
	Ok(HttpRequest {
		method,
		path,
		query,
		headers,
		body,
	})
}

async fn handle_req(
	mut req: Request<hyper::body::Incoming>,
	ctx: Ctx,
) -> Result<Response<HttpBody>, hyper::Error> {
	log::info!("{} {}", req.method(), req.uri().path());

	if req.uri().path() == "/ws" && hyper_tungstenite::is_upgrade_request(&req) {
		log::info!("upgrade request");
		let (response, websocket) = hyper_tungstenite::upgrade(&mut req, None).unwrap();
		let event_tx = ctx.event_tx.clone();
		let clients = ctx.clients.clone();
		let sessions = ctx.sessions.clone();
		let session = session_from_request(&req);
		tokio::spawn(async move {
			match websocket.await {
				Ok(ws) => {
					log::info!("websocket connected");
					let ws = TungsteniteWs::new(ws);
					let handle = WguiHandle::new(event_tx, clients, sessions);
					handle.handle_ws_with_session(ws, session).await;
				}
				Err(err) => {
					log::error!("websocket error: {:?}", err);
				}
			}
		});
		return Ok(response.map(|body| body.map_err(|never| match never {}).boxed_unsync()));
	}

	let session = session_from_request(&req);
	let route = matching_http_route(&ctx.http_routes, req.method().as_str(), req.uri().path());
	let has_http_handler = ctx.http_handler.read().unwrap().is_some();
	if has_http_handler || route.is_some() {
		let request = http_request(&mut req).await?;
		if let Some(response) = custom_http_response(request.clone(), &ctx.http_handler).await {
			return Ok(response);
		}
		if let Some((route, params)) = route {
			let http_ctx = HttpCtx {
				path: request.path.clone(),
				params,
				query: request.query.clone(),
				headers: request.headers.clone(),
				session,
			};
			return Ok(http_response((route.handler)(request, http_ctx).await));
		}
	}

	let requested_static_version = query_map(&req)
		.get(STATIC_ASSET_VERSION_PARAM)
		.map(String::as_str)
		.map(str::to_owned);
	if let Some(response) = static_mount_response(
		req.uri().path(),
		requested_static_version.as_deref(),
		&ctx.static_mounts,
	)
	.await
	{
		return Ok(response);
	}

	match req.uri().path() {
		"/favicon.ico" => Ok(Response::builder()
			.status(204)
			.header("cache-control", "public, max-age=86400")
			.body(full_body(Bytes::new()))
			.unwrap()),
		"/index.js" => Ok(Response::builder()
			.header("content-type", "text/javascript")
			.header("cache-control", "no-store")
			.body(full_body(INDEX_JS_BYTES))
			.unwrap()),
		"/index.css" => Ok(Response::builder()
			.header("content-type", "text/css")
			.header("cache-control", "no-store")
			.body(full_body(CSS_JS_BYTES))
			.unwrap()),
		"/app.css" => {
			let css = ctx.app_css.read().unwrap().clone();
			match css {
				Some(css) => Ok(Response::builder()
					.header("content-type", "text/css")
					.header("cache-control", "no-store")
					.body(full_body(css))
					.unwrap()),
				None => Ok(Response::builder()
					.status(404)
					.header("cache-control", "no-store")
					.body(full_body("app css not set"))
					.unwrap()),
			}
		}
		path if path.starts_with("/assets/") => {
			let Some(asset_path) = sanitize_asset_path(path) else {
				return Ok(Response::builder()
					.status(400)
					.body(full_body("bad asset path"))
					.unwrap());
			};
			match tokio::fs::read(&asset_path).await {
				Ok(bytes) => Ok(Response::builder()
					.header("content-type", content_type_for(&asset_path))
					.body(full_body(bytes))
					.unwrap()),
				Err(_) => Ok(Response::builder()
					.status(404)
					.body(full_body("asset not found"))
					.unwrap()),
			}
		}
		path if path.starts_with("/fs/") => {
			let Some(file_path) = sanitize_fs_path(path) else {
				return Ok(Response::builder()
					.status(400)
					.body(full_body("bad file path"))
					.unwrap());
			};
			match tokio::fs::read(&file_path).await {
				Ok(bytes) => Ok(Response::builder()
					.header("content-type", content_type_for(&file_path))
					.body(full_body(bytes))
					.unwrap()),
				Err(_) => Ok(Response::builder()
					.status(404)
					.body(full_body("file not found"))
					.unwrap()),
			}
		}
		_ => {
			if let Some(renderer) = ctx.ssr {
				let session = session_from_request(&req);
				let route = RouteContext {
					path: req.uri().path().to_string(),
					params: std::collections::HashMap::new(),
					query: query_map(&req),
				};
				match (renderer)(route, session) {
					Some(SsrResponse::Render { item, title }) => {
						let hydration_id = next_ssr_hydration_id();
						store_ssr_hydration_root(
							&ctx.ssr_hydration_roots,
							hydration_id.clone(),
							req.uri().path().to_string(),
							item.clone(),
							title.clone(),
						)
						.await;
						let html = ssr::render_document_with_app_css_hydration_title(
							&item,
							ctx.app_css.read().unwrap().is_some(),
							Some(&hydration_id),
							title.as_deref(),
						);
						Ok(Response::builder()
							.header("content-type", "text/html")
							.header("cache-control", "no-store")
							.body(full_body(html))
							.unwrap())
					}
					Some(SsrResponse::Redirect(url)) => Ok(Response::builder()
						.status(303)
						.header("location", url)
						.header("cache-control", "no-store")
						.body(full_body(Bytes::new()))
						.unwrap()),
					None => Ok(Response::builder()
						.header("content-type", "text/html")
						.header("cache-control", "no-store")
						.body(full_body(index_html_response(&ctx.app_css)))
						.unwrap()),
				}
			} else {
				Ok(Response::builder()
					.header("content-type", "text/html")
					.header("cache-control", "no-store")
					.body(full_body(index_html_response(&ctx.app_css)))
					.unwrap())
			}
		}
	}
}

pub struct Server {
	listener: TcpListener,
	event_tx: mpsc::UnboundedSender<ClientMessage>,
	clients: Clients,
	sessions: Sessions,
	ssr: Option<Arc<dyn Fn(RouteContext, Option<String>) -> Option<SsrResponse> + Send + Sync>>,
	http_handler: SharedHttpHandler,
	http_routes: SharedHttpRoutes,
	app_css: SharedAppCss,
	static_mounts: SharedStaticMounts,
	ssr_hydration_roots: SsrHydrationRoots,
}

impl Server {
	pub async fn new(
		addr: SocketAddr,
		event_tx: mpsc::UnboundedSender<ClientMessage>,
		clients: Clients,
		sessions: Sessions,
		ssr: Option<Arc<dyn Fn(RouteContext, Option<String>) -> Option<SsrResponse> + Send + Sync>>,
		http_handler: SharedHttpHandler,
		http_routes: SharedHttpRoutes,
		app_css: SharedAppCss,
		static_mounts: SharedStaticMounts,
		ssr_hydration_roots: SsrHydrationRoots,
	) -> Self {
		let listener = TcpListener::bind(addr).await.unwrap();
		log::info!("listening on http://localhost:{}", addr.port());

		Self {
			listener,
			event_tx,
			clients,
			sessions,
			ssr,
			http_handler,
			http_routes,
			app_css,
			static_mounts,
			ssr_hydration_roots,
		}
	}

	pub async fn run(self) {
		loop {
			tokio::select! {
				res = self.listener.accept() => {
					match res {
						Ok((socket, addr)) => {
							log::info!("accepted connection from {}", addr);
							let io = TokioIo::new(socket);
							let event_tx = self.event_tx.clone();
							let clients = self.clients.clone();
								let sessions = self.sessions.clone();
								let ssr = self.ssr.clone();
								let http_handler = self.http_handler.clone();
								let http_routes = self.http_routes.clone();
								let app_css = self.app_css.clone();
								let static_mounts = self.static_mounts.clone();
								let ssr_hydration_roots = self.ssr_hydration_roots.clone();
								tokio::spawn(async move {
									let service = service_fn(move |req| {
										handle_req(req, Ctx {
											event_tx: event_tx.clone(),
											clients: clients.clone(),
											sessions: sessions.clone(),
											ssr: ssr.clone(),
											http_handler: http_handler.clone(),
											http_routes: http_routes.clone(),
											app_css: app_css.clone(),
											static_mounts: static_mounts.clone(),
											ssr_hydration_roots: ssr_hydration_roots.clone(),
										})
									});

								if let Err(err) = http1::Builder::new()
									.serve_connection(io, service)
									.with_upgrades()
									.await {

									log::error!("server error: {:?}", err);
								}
							});
						},
						Err(err) => {
							log::error!("accept error: {:?}", err);
						}
					}
				}
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn temporary_static_file(contents: &[u8]) -> PathBuf {
		let unique = SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.unwrap()
			.as_nanos();
		let path = std::env::temp_dir().join(format!(
			"wgui-static-asset-{}-{unique}.js",
			std::process::id()
		));
		std::fs::write(&path, contents).unwrap();
		path
	}

	fn cache_control(response: &Response<HttpBody>) -> &str {
		response
			.headers()
			.get("cache-control")
			.unwrap()
			.to_str()
			.unwrap()
	}

	#[test]
	fn static_asset_url_uses_content_fingerprint() {
		let path = temporary_static_file(b"export default 1");
		let (_, first) = StaticMount::file("component.js".to_string(), path.clone());
		let (_, repeated) = StaticMount::file("/component.js".to_string(), path.clone());

		assert_eq!(first, repeated);
		assert!(first.url().starts_with("/component.js?wgui-v="));

		std::fs::write(&path, b"export default 2").unwrap();
		let (_, changed) = StaticMount::file("/component.js".to_string(), path.clone());
		assert_ne!(first, changed);

		std::fs::remove_file(path).unwrap();
	}

	#[test]
	fn unreadable_static_asset_keeps_unversioned_route() {
		let path = std::env::temp_dir().join("wgui-static-asset-does-not-exist.js");
		let (_, asset) = StaticMount::file("missing.js".to_string(), path);

		assert_eq!(asset.url(), "/missing.js");
	}

	#[tokio::test]
	async fn fingerprinted_static_request_is_immutable() {
		let path = temporary_static_file(b"export default 1");
		let (mount, asset) = StaticMount::file("/component.js".to_string(), path.clone());
		let version = asset.url().split_once('=').unwrap().1;
		let mounts = Arc::new(RwLock::new(vec![mount]));

		let response = static_mount_response("/component.js", Some(version), &mounts)
			.await
			.unwrap();
		assert_eq!(cache_control(&response), IMMUTABLE_STATIC_CACHE_CONTROL);

		let unversioned = static_mount_response("/component.js", None, &mounts)
			.await
			.unwrap();
		assert_eq!(
			cache_control(&unversioned),
			UNVERSIONED_STATIC_CACHE_CONTROL
		);

		let mismatched = static_mount_response("/component.js", Some("old"), &mounts)
			.await
			.unwrap();
		assert_eq!(cache_control(&mismatched), UNVERSIONED_STATIC_CACHE_CONTROL);

		std::fs::remove_file(path).unwrap();
	}
}
