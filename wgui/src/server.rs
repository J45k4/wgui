#![cfg(feature = "hyper")]

use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::Request;
use hyper::Response;
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::mpsc;

use crate::gui::Item;
use crate::ssr;
use crate::types::{ClientMessage, Clients};
use crate::ws::TungsteniteWs;
use crate::{Sessions, WguiHandle};

const INDEX_HTML_BYTES: &[u8] = include_bytes!("../../dist/index.html");
const INDEX_JS_BYTES: &[u8] = include_bytes!("../../dist/index.js");
const CSS_JS_BYTES: &[u8] = include_bytes!("../../dist/index.css");

fn content_type_for(path: &Path) -> &'static str {
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

struct Ctx {
	event_tx: mpsc::UnboundedSender<ClientMessage>,
	clients: Clients,
	sessions: Sessions,
	ssr: Option<Arc<dyn Fn(&str) -> Option<Item> + Send + Sync>>,
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

async fn handle_req(
	mut req: Request<hyper::body::Incoming>,
	ctx: Ctx,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
	log::info!("{} {}", req.method(), req.uri().path());

	if req.uri().path() == "/ws" && hyper_tungstenite::is_upgrade_request(&req) {
		log::info!("upgrade request");
		let (response, websocket) = hyper_tungstenite::upgrade(&mut req, None).unwrap();
		let event_tx = ctx.event_tx.clone();
		let clients = ctx.clients.clone();
		let sessions = ctx.sessions.clone();
		let session = session_from_query(&req);
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
		return Ok(response);
	}

	match req.uri().path() {
		"/index.js" => Ok(Response::new(Full::new(Bytes::from(INDEX_JS_BYTES)))),
		"/index.css" => Ok(Response::new(Full::new(Bytes::from(CSS_JS_BYTES)))),
		path if path.starts_with("/assets/") => {
			let Some(asset_path) = sanitize_asset_path(path) else {
				return Ok(Response::builder()
					.status(400)
					.body(Full::new(Bytes::from("bad asset path")))
					.unwrap());
			};
			match tokio::fs::read(&asset_path).await {
				Ok(bytes) => Ok(Response::builder()
					.header("content-type", content_type_for(&asset_path))
					.body(Full::new(Bytes::from(bytes)))
					.unwrap()),
				Err(_) => Ok(Response::builder()
					.status(404)
					.body(Full::new(Bytes::from("asset not found")))
					.unwrap()),
			}
		}
		path if path.starts_with("/fs/") => {
			let Some(file_path) = sanitize_fs_path(path) else {
				return Ok(Response::builder()
					.status(400)
					.body(Full::new(Bytes::from("bad file path")))
					.unwrap());
			};
			match tokio::fs::read(&file_path).await {
				Ok(bytes) => Ok(Response::builder()
					.header("content-type", content_type_for(&file_path))
					.body(Full::new(Bytes::from(bytes)))
					.unwrap()),
				Err(_) => Ok(Response::builder()
					.status(404)
					.body(Full::new(Bytes::from("file not found")))
					.unwrap()),
			}
		}
		_ => {
			if let Some(renderer) = ctx.ssr {
				if let Some(item) = (renderer)(req.uri().path()) {
					let html = ssr::render_document(&item);
					Ok(Response::new(Full::new(Bytes::from(html))))
				} else {
					Ok(Response::new(Full::new(Bytes::from(INDEX_HTML_BYTES))))
				}
			} else {
				Ok(Response::new(Full::new(Bytes::from(INDEX_HTML_BYTES))))
			}
		}
	}
}

pub struct Server {
	listener: TcpListener,
	event_tx: mpsc::UnboundedSender<ClientMessage>,
	clients: Clients,
	sessions: Sessions,
	ssr: Option<Arc<dyn Fn(&str) -> Option<Item> + Send + Sync>>,
}

impl Server {
	pub async fn new(
		addr: SocketAddr,
		event_tx: mpsc::UnboundedSender<ClientMessage>,
		clients: Clients,
		sessions: Sessions,
		ssr: Option<Arc<dyn Fn(&str) -> Option<Item> + Send + Sync>>,
	) -> Self {
		let listener = TcpListener::bind(addr).await.unwrap();
		log::info!("listening on http://localhost:{}", addr.port());

		Self {
			listener,
			event_tx,
			clients,
			sessions,
			ssr,
		}
	}

	pub async fn run(mut self) {
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
							tokio::spawn(async move {
								let service = service_fn(move |req| {
									handle_req(req, Ctx {
										event_tx: event_tx.clone(),
										clients: clients.clone(),
										sessions: sessions.clone(),
										ssr: ssr.clone(),
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
