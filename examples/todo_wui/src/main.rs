use ::axum::Router;
use log::Level;
use std::net::SocketAddr;
use std::sync::Arc;
use wgui::WuiModel;
use wgui::wui::runtime::Ctx;

mod context;
mod controllers;
mod generated;

#[derive(Debug, Clone, WuiModel)]
pub struct TodoItem {
	id: u32,
	name: String,
	completed: bool,
}

#[derive(Debug, Default, Clone, WuiModel)]
pub struct TodoState {
	new_todo_name: String,
	items: Vec<TodoItem>,
}

#[tokio::main]
async fn main() {
	simple_logger::init_with_level(Level::Info).unwrap();

	let ctx = Arc::new(Ctx::new(context::SharedContext::default()));
	let router = generated::routes::router(ctx);
	let app = Router::new().merge(router);

	let addr: SocketAddr = "0.0.0.0:12345".parse().unwrap();
	log::info!("listening on http://localhost:12345");
	let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
	::axum::serve(listener, app).await.unwrap();
}
