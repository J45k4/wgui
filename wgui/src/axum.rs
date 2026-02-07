#![cfg(feature = "axum")]

use anyhow::Error;
use axum::{
	extract::ws::{Message as AxumMessage, WebSocket, WebSocketUpgrade},
	extract::Path,
	http::{header, HeaderMap, HeaderValue},
	response::IntoResponse,
	routing::get,
	Router,
};
use futures_util::{Sink, Stream};
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::{gui::Item, ssr, WguiHandle, WsMessage};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
pub enum SameSite {
	Lax,
	Strict,
	None,
}

#[derive(Clone, Debug)]
pub struct SessionCookieConfig {
	pub name: String,
	pub path: String,
	pub max_age_seconds: Option<i64>,
	pub http_only: bool,
	pub secure: bool,
	pub same_site: SameSite,
}

impl SessionCookieConfig {
	pub fn new(name: impl Into<String>) -> Self {
		Self {
			name: name.into(),
			path: "/".to_string(),
			max_age_seconds: None,
			http_only: true,
			secure: false,
			same_site: SameSite::Lax,
		}
	}

	pub fn path(mut self, path: impl Into<String>) -> Self {
		self.path = path.into();
		self
	}

	pub fn max_age_seconds(mut self, max_age_seconds: i64) -> Self {
		self.max_age_seconds = Some(max_age_seconds);
		self
	}

	pub fn http_only(mut self, http_only: bool) -> Self {
		self.http_only = http_only;
		self
	}

	pub fn secure(mut self, secure: bool) -> Self {
		self.secure = secure;
		self
	}

	pub fn same_site(mut self, same_site: SameSite) -> Self {
		self.same_site = same_site;
		self
	}
}

/// Convenience router that exposes WgUi-specific routes for axum applications.
pub fn router(handle: WguiHandle) -> Router {
	let ws_handle = handle.clone();

	Router::new()
		.route(
			"/ws",
			get(move |ws: WebSocketUpgrade| {
				let handle = ws_handle.clone();
				async move {
					ws.on_upgrade(move |socket| async move {
						let ws = AxumWs::new(socket);
						handle.handle_ws(ws).await;
					})
				}
			}),
		)
		.route("/", get(index_html))
		.route("/index.js", get(index_js))
		.route("/index.css", get(index_css))
		.route("/assets/{*path}", get(asset_file))
}

/// Convenience router that issues a session cookie on initial HTML responses.
pub fn router_with_session(handle: WguiHandle, session: SessionCookieConfig) -> Router {
	let ws_handle = handle.clone();
	let session_for_root = session.clone();
	let session_for_ws = session.clone();

	Router::new()
		.route(
			"/ws",
			get(move |ws: WebSocketUpgrade, headers: HeaderMap| {
				let handle = ws_handle.clone();
				let session_name = session_for_ws.name.clone();
				async move {
					let session_id = cookie_value(&headers, &session_name);
					ws.on_upgrade(move |socket| async move {
						let ws = AxumWs::new(socket);
						handle.handle_ws_with_session(ws, session_id).await;
					})
				}
			}),
		)
		.route(
			"/",
			get(move |headers: HeaderMap| {
				let session = session_for_root.clone();
				async move { index_html_with_session(headers, session).await }
			}),
		)
		.route("/index.js", get(index_js))
		.route("/index.css", get(index_css))
		.route("/assets/{*path}", get(asset_file))
}

/// Convenience router that serves a server-rendered HTML snapshot on first load.
pub fn router_with_ssr(
	handle: WguiHandle,
	renderer: Arc<dyn Fn() -> Item + Send + Sync>,
) -> Router {
	router_with_ssr_routes(handle, renderer, &["/"])
}

/// Convenience router that serves a server-rendered HTML snapshot on first load.
/// Also ensures a session cookie exists on HTML responses.
pub fn router_with_ssr_routes_and_session(
	handle: WguiHandle,
	renderer: Arc<dyn Fn() -> Item + Send + Sync>,
	routes: &[&str],
	session: SessionCookieConfig,
) -> Router {
	let ws_handle = handle.clone();
	let ssr_renderer = renderer.clone();
	let session_for_ws = session.clone();

	let mut router = Router::new()
		.route(
			"/ws",
			get(move |ws: WebSocketUpgrade, headers: HeaderMap| {
				let handle = ws_handle.clone();
				let session_name = session_for_ws.name.clone();
				async move {
					let session_id = cookie_value(&headers, &session_name);
					ws.on_upgrade(move |socket| async move {
						let ws = AxumWs::new(socket);
						handle.handle_ws_with_session(ws, session_id).await;
					})
				}
			}),
		)
		.route("/index.js", get(index_js))
		.route("/index.css", get(index_css))
		.route("/assets/{*path}", get(asset_file));

	for route in routes {
		let renderer = ssr_renderer.clone();
		let session = session.clone();
		router = router.route(
			route,
			get(move |headers: HeaderMap| {
				let renderer = renderer.clone();
				let session = session.clone();
				async move { index_html_ssr_with_session(headers, renderer, session).await }
			}),
		);
	}
	router
}

/// Convenience router that serves a server-rendered HTML snapshot on first load.
/// Additional routes receive the same HTML shell so client routing can take over.
pub fn router_with_ssr_routes(
	handle: WguiHandle,
	renderer: Arc<dyn Fn() -> Item + Send + Sync>,
	routes: &[&str],
) -> Router {
	let ws_handle = handle.clone();
	let ssr_renderer = renderer.clone();

	let mut router = Router::new()
		.route(
			"/ws",
			get(move |ws: WebSocketUpgrade| {
				let handle = ws_handle.clone();
				async move {
					ws.on_upgrade(move |socket| async move {
						let ws = AxumWs::new(socket);
						handle.handle_ws(ws).await;
					})
				}
			}),
		)
		.route("/index.js", get(index_js))
		.route("/index.css", get(index_css))
		.route("/assets/{*path}", get(asset_file));

	for route in routes {
		let renderer = ssr_renderer.clone();
		router = router.route(
			route,
			get(move || {
				let renderer = renderer.clone();
				async move { index_html_ssr(renderer).await }
			}),
		);
	}
	router
}

async fn index_html() -> impl IntoResponse {
	(
		[(header::CONTENT_TYPE, "text/html")],
		crate::dist::index_html(),
	)
}

async fn index_html_with_session(
	headers: HeaderMap,
	session: SessionCookieConfig,
) -> impl IntoResponse {
	let mut response_headers = HeaderMap::new();
	response_headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/html"));
	if let Some(cookie) = ensure_session_cookie(&headers, &session) {
		response_headers.insert(header::SET_COOKIE, cookie);
	}
	(response_headers, crate::dist::index_html())
}

async fn index_html_ssr(renderer: Arc<dyn Fn() -> Item + Send + Sync>) -> impl IntoResponse {
	let item = (renderer)();
	let html = ssr::render_document(&item);
	([(header::CONTENT_TYPE, "text/html")], html)
}

async fn index_html_ssr_with_session(
	headers: HeaderMap,
	renderer: Arc<dyn Fn() -> Item + Send + Sync>,
	session: SessionCookieConfig,
) -> impl IntoResponse {
	let item = (renderer)();
	let html = ssr::render_document(&item);
	let mut response_headers = HeaderMap::new();
	response_headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/html"));
	if let Some(cookie) = ensure_session_cookie(&headers, &session) {
		response_headers.insert(header::SET_COOKIE, cookie);
	}
	(response_headers, html)
}
async fn index_js() -> impl IntoResponse {
	(
		[(header::CONTENT_TYPE, "text/javascript")],
		crate::dist::index_js(),
	)
}

async fn index_css() -> impl IntoResponse {
	(
		[(header::CONTENT_TYPE, "text/css")],
		crate::dist::index_css(),
	)
}

fn content_type_for(path: &std::path::Path) -> &'static str {
	match path
		.extension()
		.and_then(|ext| ext.to_str())
		.unwrap_or_default()
	{
		"css" => "text/css",
		"js" => "text/javascript",
		"html" => "text/html",
		"stl" => "model/stl",
		"jpg" | "jpeg" => "image/jpeg",
		"png" => "image/png",
		"svg" => "image/svg+xml",
		_ => "application/octet-stream",
	}
}

fn sanitize_asset_path(path: &str) -> Option<PathBuf> {
	let mut out = PathBuf::from("assets");
	for part in path.split('/') {
		if part.is_empty() || part == "." || part == ".." {
			return None;
		}
		out.push(part);
	}
	Some(out)
}

async fn asset_file(Path(path): Path<String>) -> impl IntoResponse {
	let Some(full_path) = sanitize_asset_path(&path) else {
		return ([(header::CONTENT_TYPE, "text/plain")], "bad asset path").into_response();
	};
	match tokio::fs::read(&full_path).await {
		Ok(bytes) => (
			[(header::CONTENT_TYPE, content_type_for(&full_path))],
			bytes,
		)
			.into_response(),
		Err(_) => axum::http::StatusCode::NOT_FOUND.into_response(),
	}
}

fn ensure_session_cookie(headers: &HeaderMap, config: &SessionCookieConfig) -> Option<HeaderValue> {
	if cookie_value(headers, &config.name).is_some() {
		return None;
	}
	let session_id = new_session_id();
	let cookie = build_cookie(&config.name, &session_id, config);
	HeaderValue::from_str(&cookie).ok()
}

fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
	let header_value = headers.get(header::COOKIE)?;
	let header_str = header_value.to_str().ok()?;
	for part in header_str.split(';') {
		let part = part.trim();
		let (key, value) = part.split_once('=')?;
		if key == name {
			return Some(value.to_string());
		}
	}
	None
}

fn build_cookie(name: &str, value: &str, config: &SessionCookieConfig) -> String {
	let mut cookie = format!("{}={}; Path={}", name, value, config.path);
	if let Some(max_age) = config.max_age_seconds {
		cookie.push_str(&format!("; Max-Age={}", max_age));
	}
	if config.http_only {
		cookie.push_str("; HttpOnly");
	}
	if config.secure {
		cookie.push_str("; Secure");
	}
	match config.same_site {
		SameSite::Lax => cookie.push_str("; SameSite=Lax"),
		SameSite::Strict => cookie.push_str("; SameSite=Strict"),
		SameSite::None => cookie.push_str("; SameSite=None"),
	}
	cookie
}

fn new_session_id() -> String {
	static COUNTER: AtomicU64 = AtomicU64::new(1);
	let nanos = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.unwrap_or_default()
		.as_nanos();
	let count = COUNTER.fetch_add(1, Ordering::Relaxed);
	format!("{:x}{:x}", nanos, count)
}

struct AxumWs {
	inner: WebSocket,
}

impl AxumWs {
	fn new(inner: WebSocket) -> Self {
		Self { inner }
	}
}

impl Stream for AxumWs {
	type Item = Result<WsMessage, Error>;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		match Stream::poll_next(Pin::new(&mut self.inner), cx) {
			Poll::Ready(Some(Ok(msg))) => {
				let converted = match msg {
					AxumMessage::Text(text) => WsMessage::Text(text.to_string()),
					AxumMessage::Binary(data) => WsMessage::Binary(data.to_vec()),
					AxumMessage::Ping(data) => WsMessage::Ping(data.to_vec()),
					AxumMessage::Pong(data) => WsMessage::Pong(data.to_vec()),
					AxumMessage::Close(_) => WsMessage::Close,
				};
				Poll::Ready(Some(Ok(converted)))
			}
			Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(err.into()))),
			Poll::Ready(None) => Poll::Ready(None),
			Poll::Pending => Poll::Pending,
		}
	}
}

impl Sink<WsMessage> for AxumWs {
	type Error = Error;

	fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		Sink::poll_ready(Pin::new(&mut self.inner), cx).map_err(Into::into)
	}

	fn start_send(mut self: Pin<&mut Self>, item: WsMessage) -> Result<(), Self::Error> {
		let msg = match item {
			WsMessage::Text(text) => AxumMessage::Text(text.into()),
			WsMessage::Binary(data) => AxumMessage::Binary(data.into()),
			WsMessage::Ping(data) => AxumMessage::Ping(data.into()),
			WsMessage::Pong(data) => AxumMessage::Pong(data.into()),
			WsMessage::Close => AxumMessage::Close(None),
		};

		Sink::start_send(Pin::new(&mut self.inner), msg).map_err(Into::into)
	}

	fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		Sink::poll_flush(Pin::new(&mut self.inner), cx).map_err(Into::into)
	}

	fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		Sink::poll_close(Pin::new(&mut self.inner), cx).map_err(Into::into)
	}
}
