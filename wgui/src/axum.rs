#![cfg(feature = "axum")]

use anyhow::Error;
use axum::{
	extract::ws::{Message as AxumMessage, WebSocket, WebSocketUpgrade},
	http::header,
	response::IntoResponse,
	routing::get,
	Router,
};
use futures_util::{Sink, Stream};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::{WguiHandle, WsMessage};

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
}

async fn index_html() -> impl IntoResponse {
	(
		[(header::CONTENT_TYPE, "text/html")],
		crate::dist::index_html(),
	)
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
