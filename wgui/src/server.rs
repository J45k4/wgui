#![cfg(feature = "hyper")]

use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::Request;
use hyper::Response;
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::sync::mpsc;

use crate::types::{ClientEvent, Clients};
use crate::ws::TungsteniteWs;
use crate::WguiHandle;

const INDEX_HTML_BYTES: &[u8] = include_bytes!("../../dist/index.html");
const INDEX_JS_BYTES: &[u8] = include_bytes!("../../dist/index.js");
const CSS_JS_BYTES: &[u8] = include_bytes!("../../dist/index.css");

struct Ctx {
	event_tx: mpsc::UnboundedSender<ClientEvent>,
	clients: Clients,
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
		tokio::spawn(async move {
			match websocket.await {
				Ok(ws) => {
					log::info!("websocket connected");
					let ws = TungsteniteWs::new(ws);
					let handle = WguiHandle::new(event_tx, clients);
					handle.handle_ws(ws).await;
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
		_ => Ok(Response::new(Full::new(Bytes::from(INDEX_HTML_BYTES)))),
	}
}

pub struct Server {
	listener: TcpListener,
	event_tx: mpsc::UnboundedSender<ClientEvent>,
	clients: Clients,
}

impl Server {
	pub async fn new(
		addr: SocketAddr,
		event_tx: mpsc::UnboundedSender<ClientEvent>,
		clients: Clients,
	) -> Self {
		let listener = TcpListener::bind(addr).await.unwrap();
		log::info!("listening on http://localhost:{}", addr.port());

		Self {
			listener,
			event_tx,
			clients,
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
							tokio::spawn(async move {
								let service = service_fn(move |req| {
									handle_req(req, Ctx {
										event_tx: event_tx.clone(),
										clients: clients.clone()
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
